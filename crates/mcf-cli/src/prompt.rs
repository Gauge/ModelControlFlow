//! `mcf prompt`: what a prompt does to a model (§3.8, A19, A7).
//!
//! **A client of the control plane, like every other surface** (A22). The
//! measuring is the daemon's — one generation per sentence and one per seed —
//! and what is here is the asking and the reading. A window or a console
//! showing the same thing asks the same request and renders the same answer.
//!
//! **`--json` because a report is something a program acts on.** A person
//! reads which of their sentences reached the answer; an editor or an agent
//! wants the same thing as data, and printing it twice in two shapes is how
//! the two drift apart.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

/// The seed every report is taken under unless the caller says otherwise.
///
/// Stated rather than drawn: a report is a comparison of answers, and a seed
/// that moved between two runs of it would make them incomparable (D19).
const SEED: u64 = 41;

/// Asks the daemon what this prompt does.
pub(crate) fn report(named: &str, prompt: &str, as_json: bool) -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine".to_owned(),
            served: false,
        };
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        };
    }
    let Ok(mut connection) = UnixStream::connect(&socket) else {
        return Response {
            text: format!("mcf: nothing is listening on {}", socket.display()),
            served: false,
        };
    };
    // No deadline: a report is many generations, and a timeout here would
    // report a working run as a broken daemon.
    let request = Request::PromptReport {
        model: named.to_owned(),
        prompt: prompt.to_owned(),
        seed: SEED,
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return Response {
            text: "mcf: the request could not be sent".to_owned(),
            served: false,
        };
    }
    let mut line = String::new();
    if BufReader::new(&connection).read_line(&mut line).is_err() {
        return Response {
            text: "mcf: MCF did not answer".to_owned(),
            served: false,
        };
    }
    let Ok(answer) = Answer::read(&line) else {
        return Response {
            text: "mcf: MCF answered with something that is not an answer".to_owned(),
            served: false,
        };
    };
    if !answer.served {
        return Response {
            text: format!(
                "mcf: refused\n  {}",
                crate::say::refused_because(&answer.body)
            ),
            served: false,
        };
    }
    if as_json {
        return Response {
            text: answer.body.to_line(),
            served: true,
        };
    }
    Response {
        text: rendered(&answer.body, named).join("\n"),
        served: true,
    }
}

/// How many tenths of the answer moved, for the bar.
///
/// The truncation is the point: a bar has ten cells and a part-cell is not one
/// of them.
#[allow(
    clippy::integer_division,
    reason = "a bar has ten cells; a fraction of a cell is not drawn"
)]
const fn tenths_of_the_answer(parts_per_million: i64) -> i64 {
    parts_per_million / 100_000
}

/// Parts per million as whole and tenth of a per cent.
#[allow(
    clippy::integer_division,
    reason = "a tenth of a per cent is the resolution shown; the rest is not"
)]
const fn as_percent(parts_per_million: i64) -> (i64, i64) {
    (parts_per_million / 10_000, (parts_per_million / 1_000) % 10)
}

/// How much each sentence steered the answer, as a person reads it.
///
/// Its own function because it is the part of the report that grows: a bar, a
/// figure, the floor, and what the floor means.
fn steering_lines(body: &Value) -> Vec<String> {
    let count = |key: &str| body.get(key).and_then(Value::as_integer).unwrap_or(0);
    let mut lines = Vec::new();
    let clauses = body.get("clauses").and_then(Value::as_list).unwrap_or(&[]);
    if clauses.is_empty() {
        lines.push(
            "    this prompt is one sentence, so there is nothing to remove — a prompt of one \
             part cannot be taken apart"
                .to_owned(),
        );
    }
    for clause in clauses {
        let said = clause
            .get("text")
            .and_then(Value::as_text)
            .unwrap_or_default();
        let moved = clause
            .get("moved_parts_per_million")
            .and_then(Value::as_integer)
            .unwrap_or(0);
        // A bar, so the eye finds the sentences that steered the answer
        // without reading a column of numbers. Ten cells of ten per cent.
        let filled = usize::try_from(tenths_of_the_answer(moved))
            .unwrap_or(0)
            .min(10);
        let bar: String = "#".repeat(filled) + &"·".repeat(10_usize.saturating_sub(filled));
        let (whole, tenth) = as_percent(moved);
        lines.push(format!("    {bar}  {whole:>3}.{tenth}%  {said}"));
    }
    if !clauses.is_empty() {
        let floor = count("floor_parts_per_million");
        let (whole, tenth) = as_percent(floor);
        let at_floor = clauses
            .iter()
            .filter(|clause| {
                clause
                    .get("moved_parts_per_million")
                    .and_then(Value::as_integer)
                    .unwrap_or(0)
                    <= floor
            })
            .count();
        lines.push(String::new());
        // **Every removal giving the same answer is a finding, and it reads
        // like a broken tool.** A column of noughts is what a well-addressed
        // model does with a prompt it would have answered the same way
        // regardless — the sentences are not steering it. Saying so is the
        // difference between a reading and an apparent failure (A7).
        if floor == 0 && at_floor == clauses.len() {
            lines.push(
                "    every sentence removed gave the SAME answer, to the character — including \
                 the control. This prompt did not steer this model: it would have answered the \
                 same way with less. That is a reading, not a failure of the measurement."
                    .to_owned(),
            );
            lines.push(String::new());
        }
        lines.push(format!(
            "    the floor is {whole}.{tenth}% — how much the answer moved for a control \
             sentence carrying no instruction, put in and taken out again. Read the column as \
             an ORDERING, not as relevance: removing anything shifts what follows it, and a \
             sentence well above the floor may still have steered nothing."
        ));
        if at_floor > 0 {
            let held = if at_floor == 1 { "sits" } else { "sit" };
            lines.push(format!(
                "    {} {held} at or under it. That is not a claim they are wrong: a sentence \
                 restating another moves little and is not thereby mistaken.",
                count_of(i64::try_from(at_floor).unwrap_or(0), "sentence")
            ));
            // **And which ones.** A reader told that one of six sentences did
            // nothing has to work out which, from a column they were just told
            // not to read as relevance. Naming them is a fact about the same
            // measurement, not a further claim.
            for quiet in body
                .get("clauses")
                .and_then(Value::as_list)
                .unwrap_or(&[])
                .iter()
                .filter(|clause| {
                    clause
                        .get("moved_parts_per_million")
                        .and_then(Value::as_integer)
                        .unwrap_or(0)
                        <= floor
                })
            {
                if let Some(text) = quiet.get("text").and_then(Value::as_text) {
                    lines.push(format!("      · {text}"));
                }
            }
        }
    }
    let over = count("clauses_over_the_cap");
    if over > 0 {
        lines.push(format!(
            "    {} not removed: a report is one generation each, and this one stopped at \
             the cap",
            count_of(over, "further sentence")
        ));
    }

    lines
}

/// The report, as a person reads it.
/// A share of an answer, written out.
///
/// Integer arithmetic, because the workspace ships no floating point: a NaN
/// that reaches a record is a figure nobody can compare, and `as_percent`
/// above has split parts per million into whole and tenth for the same reason
/// since this file was written.
fn percent(parts_per_million: i64) -> String {
    let (whole, tenth) = as_percent(parts_per_million);
    format!("{whole}.{tenth}%")
}

/// A count and the thing counted, in English.
///
/// `1 sentence`, `2 sentences`. Every one of these read `1 sentence(s)`.
fn count_of(how_many: i64, noun: &str) -> String {
    if how_many == 1 {
        format!("1 {noun}")
    } else {
        format!("{how_many} {noun}s")
    }
}

/// What to call the model at the top of the report.
///
/// The file's own name where a path was given: a report headed with an
/// absolute path spends its first line on a directory the reader typed, and
/// what identifies the model to them is the last part of it. Anything that is
/// not a path is shown as they wrote it.
fn header_name(named: &str) -> &str {
    named
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(named)
}

/// Where the model ranked each word of the prompt, given the ones before it.
///
/// **A reading of the prompt that does not compare two answers.** The column
/// above measures what changed when a sentence was removed, and removing
/// anything shifts everything after it — which is why it is an ordering and
/// not a measure. This asks something narrower of the same prompt: at each
/// position, was this the token the model would have written anyway? One it
/// ranked first carried nothing from the writer; one it ranked low, or did not
/// list at all, is where the prompt said something the model did not expect.
///
/// Only the ones it did not expect are printed. A prompt is mostly words the
/// model would have chosen, and a list of every position would bury the few
/// that carry the writing (§3.15).
fn what_the_model_expected(body: &Value) -> Vec<String> {
    let ranked = body.get("expected").and_then(Value::as_list).unwrap_or(&[]);
    if ranked.is_empty() {
        // Absent and *why* absent are different states, and a report that
        // showed nothing for both would be reporting ignorance as absence (A7).
        return body
            .get("expected_refused")
            .and_then(Value::as_text)
            .map(|why| {
                vec![
                    String::new(),
                    "  WHICH WORDS THE MODEL DID NOT EXPECT".to_owned(),
                    format!("    not taken: {why}"),
                ]
            })
            .unwrap_or_default();
    }
    let depth = body
        .get("ranked_depth")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    let text = |held: &Value, key: &str| {
        held.get(key)
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned()
    };
    let mut surprising: Vec<(i64, String)> = Vec::new();
    let mut expected = 0_usize;
    for held in ranked {
        let text = text(held, "text");
        match held.get("rank").and_then(Value::as_integer) {
            // Outside the list asked for: a bound, not an absence (A7), and
            // sorted above everything that was in it.
            None => surprising.push((i64::MAX, text)),
            Some(1) => expected = expected.saturating_add(1),
            Some(rank) => surprising.push((rank, text)),
        }
    }
    surprising.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    let mut lines = vec![
        String::new(),
        "  WHICH WORDS THE MODEL DID NOT EXPECT".to_owned(),
        "  each token against what it would have written there itself".to_owned(),
        String::new(),
    ];
    for (rank, said) in surprising.iter().take(10) {
        if *rank == i64::MAX {
            lines.push(format!("    outside its top {depth}   {said:?}"));
        } else {
            lines.push(format!("    its number {rank} choice   {said:?}"));
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "    {} of {} were the model's own first choice — a word it would have written there \
         anyway carries nothing from the writer",
        expected,
        ranked.len()
    ));
    lines
}

/// The answer, and the conditions every figure above was computed under.
///
/// How long an answer was allowed to be, how the prompt reached the model, and
/// what the sampler was: each of them changes what the figures mean, and none
/// of them was on the page (§3.4, A6, F147).
fn the_answer_and_its_conditions(body: &Value) -> Vec<String> {
    let text = |key: &str| {
        body.get(key)
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned()
    };
    let count = |key: &str| body.get(key).and_then(Value::as_integer).unwrap_or(0);
    let mut lines = Vec::new();
    lines.push("  THE ANSWER TO THE PROMPT AS WRITTEN".to_owned());
    let baseline = text("baseline");
    let written: Vec<&str> = baseline.lines().collect();
    for said in written.iter().take(12) {
        lines.push(format!("    {said}"));
    }
    if written.len() > 12 {
        lines.push(format!(
            "    … {} — `--json` carries the whole of it",
            count_of(
                i64::try_from(written.len().saturating_sub(12)).unwrap_or(0),
                "further line"
            )
        ));
    }
    // **What every answer was allowed to be.** A figure computed from an
    // answer that stopped at a limit is a figure about a prefix, and a reader
    // comparing two prefixes is measuring whatever the model puts first
    // (§3.4, A6, F147).
    // How the model receives the question, beside what it did with it.
    let tokens = count("prompt_tokens");
    if tokens > 0 {
        lines.push(String::new());
        lines.push(format!(
            "    the prompt reached the model as {} — `mcf segment` shows every one of them, and \
             which words this vocabulary had no single piece for",
            count_of(tokens, "token")
        ));
    }
    let limit = count("token_limit");
    if limit > 0 {
        lines.push(String::new());
        lines.push(format!(
            "    each generation stopped at {}. An answer that reached it was cut, and a figure \
             comparing two cut answers is about their first {limit} tokens",
            count_of(limit, "token")
        ));
    }
    lines.push(String::new());
    lines
}

fn rendered(body: &Value, named: &str) -> Vec<String> {
    let count = |key: &str| body.get(key).and_then(Value::as_integer).unwrap_or(0);

    let floor_ppm = body
        .get("floor_parts_per_million")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    let mut lines = vec![
        format!("what this prompt does to {}", header_name(named)),
        String::new(),
        format!(
            "  {}: one for the prompt as written, one for each sentence left out, and one for \
             each extra seed",
            count_of(count("generations"), "generation")
        ),
        String::new(),
    ];

    // **Where the floor swamps the column, that is the finding.** A floor of
    // 94.3% means removing a sentence carrying no instruction moved almost the
    // whole answer, so nothing below separates one sentence from another —
    // and the report used to draw the bars first and put the number that
    // invalidates them underneath, in the same voice as everything else. A
    // reader reads the bars (§3.15, A7, F147).
    let readable = floor_ppm < 500_000;
    if !readable {
        lines.push("  THIS RUN CANNOT SEPARATE YOUR SENTENCES".to_owned());
        lines.push(format!(
            "    removing a sentence that carries no instruction moved {} of the answer, so a \
             sentence scoring near that has told you nothing. The column below is printed \
             because hiding a measurement is worse than showing a poor one — but read it as \
             *this run did not work*, not as an ordering.",
            percent(floor_ppm)
        ));
        lines.push(String::new());
        lines.push(
            "    a floor this high usually means the answer is long and open-ended: try a \
             prompt whose answer is short, or ask for one part of the work at a time"
                .to_owned(),
        );
        lines.push(String::new());
    }

    lines.push("  HOW MUCH EACH SENTENCE STEERED THE ANSWER".to_owned());
    lines.push("  each was removed in turn, with the seed held still".to_owned());
    lines.push(String::new());

    lines.extend(steering_lines(body));
    lines.push(String::new());
    lines.push("  WHETHER SEVERAL SEEDS GAVE SEVERAL ANSWERS".to_owned());
    let distinct = count("distinct_answers");
    let asked = count("seeds_asked");
    lines.push(if distinct <= 1 {
        format!(
            "    {} gave 1 answer — and under this sampler they could not have given more. \
             MCF asks every generation at temperature 0, which takes the likeliest token every \
             time, so the seed changes nothing and this line says the same for every prompt. \
             Whether *your* prompt settles the answer is not measured here (F147).",
            count_of(asked, "seed")
        )
    } else {
        format!(
            "    {} gave {distinct} different answers — which under temperature 0 should not \
             happen at all, and is a fact about the engine rather than the prompt. Worth \
             reporting rather than hiding (A2).",
            count_of(asked, "seed")
        )
    });

    lines.push(String::new());
    lines.extend(what_the_model_expected(body));
    lines.extend(the_answer_and_its_conditions(body));
    lines.push(
        "  Nothing here says whether the prompt is good, or whether the model understood it. \
         Those are judgements and they need a rater. What is above is which sentences changed \
         the answer when they were removed, and where each word sat in what the model would \
         have written itself — two readings of the same prompt, which fail in different ways \
         and are worth reading against each other."
            .to_owned(),
    );
    lines
}
