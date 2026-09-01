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
            text: format!("mcf: {}", answer.body.to_line()),
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

fn rendered(body: &Value, named: &str) -> Vec<String> {
    let text = |key: &str| {
        body.get(key)
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned()
    };
    let count = |key: &str| body.get(key).and_then(Value::as_integer).unwrap_or(0);

    let mut lines = vec![
        format!("what this prompt does to {}", header_name(named)),
        String::new(),
        format!(
            "  {}: one for the prompt as written, one for each sentence left out, and one for \
             each extra seed",
            count_of(count("generations"), "generation")
        ),
        String::new(),
        "  HOW MUCH EACH SENTENCE STEERED THE ANSWER".to_owned(),
        "  each was removed in turn, with the seed held still".to_owned(),
        String::new(),
    ];

    lines.extend(steering_lines(body));
    lines.push(String::new());
    lines.push("  WHETHER THE PROMPT SETTLES THE ANSWER".to_owned());
    let distinct = count("distinct_answers");
    let asked = count("seeds_asked");
    lines.push(if distinct <= 1 {
        format!(
            "    {} gave 1 answer: this prompt settles the answer on this model, \
             under these conditions",
            count_of(asked, "seed")
        )
    } else {
        format!(
            "    {} gave {distinct} different answers: this prompt does not settle \
             the answer on this model. That is not a fault in the prompt — an open question \
             deserves several answers and a specification does not",
            count_of(asked, "seed")
        )
    });

    lines.push(String::new());
    lines.push("  THE ANSWER TO THE PROMPT AS WRITTEN".to_owned());
    for said in text("baseline").lines().take(6) {
        lines.push(format!("    {said}"));
    }
    lines.push(String::new());
    lines.push(
        "  Nothing here says whether the prompt is good, or whether the model understood it. \
         Those are judgements and they need a rater. What is above is which sentences changed \
         the answer and how many answers there were."
            .to_owned(),
    );
    lines
}
