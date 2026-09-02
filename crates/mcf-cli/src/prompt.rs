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

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

/// The seed every report is taken under unless the caller says otherwise.
///
/// Stated rather than drawn: a report is a comparison of answers, and a seed
/// that moved between two runs of it would make them incomparable (D19).
const SEED: u64 = 41;

/// What the command line asked to have taken apart, before the file is read.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Asked<'a> {
    /// The document, given inline.
    pub prompt: Option<&'a str>,
    /// The document, in a file; `-` is the standard input.
    pub file: Option<&'a str>,
    /// What to take it apart into, where the caller said.
    pub by: Option<mcf_serve::prompt::Unit>,
    /// The most parts to remove, where the caller said.
    pub most: Option<usize>,
    /// The temperature to draw the settledness seeds at, where the caller
    /// stated one (B-431).
    pub temperature: Option<mcf_core::configuration::Thousandths>,
}

/// The document, from wherever the caller put it.
///
/// A file that cannot be read is said with its path and the reason, not
/// analysed as empty (A2).
fn document(asked: &Asked<'_>) -> Result<String, String> {
    match (asked.prompt, asked.file) {
        (Some(text), _) => Ok(text.to_owned()),
        (None, Some("-")) => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .map(|_read| text)
                .map_err(|why| format!("the standard input could not be read: {why}"))
        }
        (None, Some(path)) => {
            std::fs::read_to_string(path).map_err(|why| format!("{path} could not be read: {why}"))
        }
        (None, None) => Err("no document was given".to_owned()),
    }
}

/// Asks the daemon what this prompt does.
pub(crate) fn report(named: &str, asked: &Asked<'_>, as_json: bool) -> Response {
    let prompt = match document(asked) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) => {
            return Response {
                text: "mcf: the document is empty — there is nothing to take apart".to_owned(),
                served: false,
            };
        }
        Err(why) => {
            return Response {
                text: format!("mcf: {why}"),
                served: false,
            };
        }
    };
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
        prompt,
        by: asked.by,
        most: asked.most,
        temperature: asked.temperature,
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

/// What the document was taken apart into, as the report calls one part.
fn unit_of(body: &Value) -> &str {
    body.get("unit")
        .and_then(Value::as_text)
        .unwrap_or("sentence")
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
    let depth = count("forced_depth");
    let unit = unit_of(body);
    let mut lines = Vec::new();
    let clauses = body.get("clauses").and_then(Value::as_list).unwrap_or(&[]);
    if clauses.is_empty() {
        lines.push(format!(
            "    this prompt is one {unit}, so there is nothing to remove — a prompt of one \
             part cannot be taken apart"
        ));
    }
    for clause in clauses {
        lines.extend(clause_lines(clause, depth));
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
            lines.push(format!(
                "    every {unit} removed gave the SAME answer, to the character — including \
                 the control. This prompt did not steer this model: it would have answered the \
                 same way with less. That is a reading, not a failure of the measurement."
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "    the floor is {whole}.{tenth}% — how much the answer moved for a control \
             sentence carrying no instruction, put in and taken out again. Read the column as \
             an ORDERING, not as relevance: removing anything shifts what follows it, and a \
             {unit} well above the floor may still have steered nothing."
        ));
        match held_said(body.get("floor_held"), depth) {
            Some(read) => lines.push(format!("    with the control sentence in, {read}")),
            None => lines.push(
                "    whether the answer would still have begun the same way was not read: it \
                 needs the served engine, and this run had none"
                    .to_owned(),
            ),
        }
        if at_floor > 0 {
            let held = if at_floor == 1 { "sits" } else { "sit" };
            lines.push(format!(
                "    {} {held} at or under it. That is not a claim they are wrong: a {unit} \
                 restating another moves little and is not thereby mistaken.",
                count_of(i64::try_from(at_floor).unwrap_or(0), unit)
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
             {} — `--most {}` would remove every one",
            count_of(over, &format!("further {}", unit_of(body))),
            count_of(count("most"), "part"),
            count("most").saturating_add(over)
        ));
    }

    lines
}

/// One sentence's row: the bar, the figure, the sentence — and under it what
/// the answer became without it and whether it would still have begun the
/// same way.
///
/// On a one-word answer every removal that changes the word scores a hundred
/// per cent and the bars tie; the answer itself and the forced rank are what
/// order them (B-429).
fn clause_lines(clause: &Value, depth: i64) -> Vec<String> {
    let said = clause
        .get("text")
        .and_then(Value::as_text)
        .unwrap_or_default();
    let moved = clause
        .get("moved_parts_per_million")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    // A bar, so the eye finds the sentences that steered the answer without
    // reading a column of numbers. Ten cells of ten per cent.
    let filled = usize::try_from(tenths_of_the_answer(moved))
        .unwrap_or(0)
        .min(10);
    let bar: String = "#".repeat(filled) + &"·".repeat(10_usize.saturating_sub(filled));
    let (whole, tenth) = as_percent(moved);
    let mut lines = vec![format!("    {bar}  {whole:>3}.{tenth}%  {said}")];
    if clause.get("changed").and_then(Value::as_bool) == Some(true) {
        lines.push(format!(
            "                        without it: {}",
            first_line_of(
                clause
                    .get("without")
                    .and_then(Value::as_text)
                    .unwrap_or_default()
            )
        ));
    }
    if let Some(read) = held_said(clause.get("held"), depth) {
        lines.push(format!("                        {read}"));
    }
    lines
}

/// The first line of an answer, cut to a width the column holds.
fn first_line_of(said: &str) -> String {
    let line = said.trim().lines().next().unwrap_or_default();
    let mut shown: String = line.chars().take(72).collect();
    if shown.chars().count() < line.chars().count() || said.trim().lines().count() > 1 {
        shown.push('…');
    }
    format!("{shown:?}")
}

/// A forced reading, in words: where the answer's first token went, and how
/// much of its opening stayed the model's first choice.
///
/// `None` where none was taken, which the caller says once rather than on
/// every line.
fn held_said(held: Option<&Value>, depth: i64) -> Option<String> {
    let held = held.filter(|held| !matches!(held, Value::Null))?;
    let count = |key: &str| held.get(key).and_then(Value::as_integer).unwrap_or(0);
    let kept = count("kept");
    let of = count("of");
    let opening = format!(
        "{kept} of {} of the opening stayed its first choice",
        count_of(of, "token")
    );
    Some(match held.get("first_rank").and_then(Value::as_integer) {
        Some(1) if kept == of => {
            format!("the answer would still have begun the same way: {opening}")
        }
        Some(1) => format!("the answer's first token stayed its first choice; {opening}"),
        Some(rank) => {
            format!("the answer's first token fell to its number {rank} choice; {opening}")
        }
        None => format!("the answer's first token fell outside its top {depth}; {opening}"),
    })
}

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

/// Whether the prompt settles the answer — under the temperature it was asked
/// at, or not asked at all.
///
/// **Not asked is said as not asked.** Every other generation in the report
/// is greedy, and under a greedy sampler the seed changes nothing; a report
/// that drew two more seeds at temperature 0 measured nothing and said so on
/// every prompt (F147). So the seeds are drawn only at a temperature the
/// caller states, and a report with none says the question is open rather
/// than that the answer settled (A7, B-431).
fn settled_lines(settled: Option<&Value>) -> Vec<String> {
    let Some(settled) = settled.filter(|held| !matches!(held, Value::Null)) else {
        return vec![
            "    not asked: no temperature was stated, and at temperature 0 — which every \
             other generation here is — the seed changes nothing, so no generation was spent \
             on the question. `--temperature 0.7` draws three seeds at 0.7 and says how many \
             answers they gave and how far apart they sat. The temperature is yours to state: \
             MCF has no house value (B60), and this model's own recommendation, if it \
             publishes one, is what `mcf explain` shows."
                .to_owned(),
        ];
    };
    let count = |key: &str| settled.get(key).and_then(Value::as_integer).unwrap_or(0);
    let temperature = settled
        .get("temperature")
        .and_then(Value::as_text)
        .unwrap_or("?");
    let distinct = count("distinct_answers");
    let asked = count("seeds_asked");
    let spread = count("spread_parts_per_million");
    let from_greedy = count("from_greedy_parts_per_million");
    let mut lines = vec![if distinct <= 1 {
        format!(
            "    {} at temperature {temperature} gave 1 answer, to the character. Under this \
             temperature this model settles this prompt — a fact about the pair, not a merit \
             of the prompt: a question with one answer should settle, and an open one need not.",
            count_of(asked, "seed")
        )
    } else {
        format!(
            "    {} at temperature {temperature} gave {distinct} different answers. The two \
             farthest apart differ in {} of their words. Several answers is what an open \
             question deserves and what a specification does not; which this is, the report \
             cannot say.",
            count_of(asked, "seed"),
            percent(spread)
        )
    }];
    lines.push(format!(
        "    the farthest of them sits {} of its words from the greedy answer above — how far \
         sampling at {temperature} takes this model from the answer the rest of this report is \
         about.",
        percent(from_greedy)
    ));
    lines
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
    ];
    // Under what addressing: a prompt read bare and one read inside the turn
    // the answer was given are two different prompts, and the report says
    // which this was (§3.4, B-429).
    if let Some(under) = body.get("ranked_under").and_then(Value::as_text) {
        lines.push(format!("  read under {under}"));
    }
    lines.push(String::new());
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
    lines.push(String::new());
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
    let unit = unit_of(body);
    let mut lines = vec![
        format!("what this prompt does to {}", header_name(named)),
        String::new(),
        format!(
            "  {}: one for the prompt as written, one for each {unit} left out, one for the \
             control sentence, and one for each further seed",
            count_of(count("generations"), "generation")
        ),
    ];
    // **The unit, and who chose it** (§3.15): a report by paragraph and one
    // by sentence are different measurements of the same text.
    if let Some(chosen_by) = body.get("unit_chosen_by").and_then(Value::as_text) {
        lines.push(format!("  taken apart by {unit}, decided by {chosen_by}"));
    }
    // **How the prompt reached the model.** Whole, in one user turn: the
    // report says so because a reader crafting a system prompt will assume a
    // system turn, and MCF has not probed for one (D43).
    if let Some(addressed) = body.get("addressed_as").and_then(Value::as_text) {
        lines.push(format!("  addressed as {addressed}"));
    }
    lines.push(String::new());

    // **Where the floor swamps the column, that is the finding.** A floor of
    // 94.3% means removing a sentence carrying no instruction moved almost the
    // whole answer, so nothing below separates one sentence from another —
    // and the report used to draw the bars first and put the number that
    // invalidates them underneath, in the same voice as everything else. A
    // reader reads the bars (§3.15, A7, F147).
    let readable = floor_ppm < 500_000;
    if !readable {
        lines.push(format!(
            "  THIS RUN CANNOT SEPARATE YOUR {}S",
            unit.to_uppercase()
        ));
        lines.push(format!(
            "    removing a sentence that carries no instruction moved {} of the answer, so a \
             {unit} scoring near that has told you nothing. The column below is printed \
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

    lines.push(format!(
        "  HOW MUCH EACH {} STEERED THE ANSWER",
        unit.to_uppercase()
    ));
    lines.push("  each was removed in turn, with the seed held still".to_owned());
    lines.push(String::new());

    lines.extend(steering_lines(body));
    lines.push(String::new());
    lines.push("  WHETHER SEVERAL SEEDS GAVE SEVERAL ANSWERS".to_owned());
    lines.extend(settled_lines(body.get("settled")));

    lines.push(String::new());
    lines.extend(what_the_model_expected(body));
    lines.extend(the_answer_and_its_conditions(body));
    lines.push(format!(
        "  Nothing here says whether the prompt is good, or whether the model understood it. \
         Those are judgements and they need a rater. What is above is which {}s changed \
         the answer when they were removed, whether the model would still have begun the same \
         answer without each, and where each word sat in what the model would have written \
         itself — three readings of the same prompt, which fail in different ways and are \
         worth reading against each other.",
        unit_of(body)
    ));
    lines
}

#[cfg(test)]
mod tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::*;

    fn body() -> Value {
        mcf_record::json::parse(
            r#"{"baseline":"Blue.","floor_parts_per_million":0,
                "floor_held":{"first_rank":1,"kept":2,"of":2},"forced_depth":60,
                "ranked_under":"chatml — set by a probe",
                "clauses":[
                  {"text":"Answer in one word.","changed":true,"moved_parts_per_million":1000000,
                   "without":"The room is painted blue.\nAnd more.","held":{"first_rank":17,"kept":0,"of":2}},
                  {"text":"What colour is the room?","changed":true,"moved_parts_per_million":1000000,
                   "without":"Yes.","held":{"first_rank":null,"kept":0,"of":2}},
                  {"text":"You are careful.","changed":false,"moved_parts_per_million":0,
                   "without":"Blue.","held":{"first_rank":1,"kept":2,"of":2}}],
                "clauses_over_the_cap":3,"settled":null,"generations":6,
                "unit":"sentence","unit_chosen_by":"the text: it has no blank line, so it is sentences",
                "most":3,
                "addressed_as":"one user turn, the whole prompt",
                "expected":[{"text":" are","rank":null,"engine_said":null}],"prompt_tokens":10}"#,
        )
        .expect("a well-formed report")
    }

    /// Three sentences at a hundred per cent tie on the bar; what the answer
    /// became and where its first token went are what order them (B-429).
    #[test]
    fn a_tied_column_is_ordered_by_the_answer_and_the_forced_rank() {
        let text = steering_lines(&body()).join("\n");
        assert!(
            text.contains("without it: \"The room is painted blue.…\""),
            "the first line of the ablated answer is shown, marked as cut: {text}"
        );
        assert!(
            text.contains("fell to its number 17 choice; 0 of 2 tokens of the opening"),
            "{text}"
        );
        assert!(text.contains("fell outside its top 60"), "{text}");
        assert!(
            text.contains("would still have begun the same way: 2 of 2"),
            "{text}"
        );
        assert!(
            !text.contains("without it: \"Blue.\""),
            "an unchanged answer is not repeated: {text}"
        );
        assert!(
            text.contains("with the control sentence in, the answer would still have begun"),
            "the floor's own forced reading is beside the floor: {text}"
        );
    }

    /// A reading nobody took is said once, not drawn as a rank (A7).
    #[test]
    fn an_untaken_forced_reading_is_said_rather_than_ranked() {
        let mut held = body();
        if let Value::Map(fields) = &mut held {
            fields.insert("floor_held".to_owned(), Value::Null);
        }
        let text = steering_lines(&held).join("\n");
        assert!(
            text.contains("was not read: it needs the served engine"),
            "{text}"
        );
        assert_eq!(held_said(Some(&Value::Null), 60), None);
        assert_eq!(held_said(None, 60), None);
    }

    /// The addressing the ranks were read under is on the page (§3.4).
    #[test]
    fn the_rank_reading_says_what_it_was_read_under() {
        let text = what_the_model_expected(&body()).join("\n");
        assert!(
            text.contains("read under chatml — set by a probe"),
            "{text}"
        );
    }

    /// The unit, who decided it, and how the prompt reached the model are
    /// the conditions of the run, and they are on the page; the cap is said
    /// with the flag that raises it (§3.4, §3.15, B-430).
    #[test]
    fn the_report_says_what_it_took_apart_and_how_it_was_addressed() {
        let text = rendered(&body(), "m").join("\n");
        assert!(
            text.contains("taken apart by sentence, decided by the text: it has no blank line"),
            "{text}"
        );
        assert!(
            text.contains("addressed as one user turn, the whole prompt"),
            "{text}"
        );
        assert!(text.contains("HOW MUCH EACH SENTENCE STEERED"), "{text}");
        assert!(
            text.contains("3 further sentences not removed") && text.contains("`--most 6`"),
            "the cap is said with what raises it: {text}"
        );
        let mut alone = body();
        if let Value::Map(fields) = &mut alone {
            fields.insert("unit".to_owned(), Value::text("paragraph".to_owned()));
        }
        let text = rendered(&alone, "m").join("\n");
        assert!(text.contains("HOW MUCH EACH PARAGRAPH STEERED"), "{text}");
    }

    /// Settledness is said under the temperature it was asked at, and where
    /// none was stated it is said as not asked — never as settled (A7, B-431).
    #[test]
    fn settledness_is_said_under_its_temperature_or_as_not_asked() {
        let text = rendered(&body(), "m").join("\n");
        assert!(
            text.contains("not asked: no temperature was stated")
                && text.contains("`--temperature 0.7`"),
            "{text}"
        );
        assert!(!text.contains("settles this prompt"), "{text}");

        let mut warm = body();
        if let Value::Map(fields) = &mut warm {
            fields.insert(
                "settled".to_owned(),
                Value::map([
                    ("temperature_thousandths", Value::Integer(700)),
                    ("temperature", Value::text("0.700")),
                    ("seeds_asked", Value::Integer(3)),
                    ("distinct_answers", Value::Integer(3)),
                    ("spread_parts_per_million", Value::Integer(412_000)),
                    ("from_greedy_parts_per_million", Value::Integer(250_000)),
                ]),
            );
        }
        let text = rendered(&warm, "m").join("\n");
        assert!(
            text.contains("3 seeds at temperature 0.700 gave 3 different answers"),
            "{text}"
        );
        assert!(
            text.contains("farthest apart differ in 41.2% of their words"),
            "{text}"
        );
        assert!(
            text.contains("sits 25.0% of its words from the greedy answer"),
            "{text}"
        );

        if let Value::Map(fields) = &mut warm
            && let Some(Value::Map(settled)) = fields.get_mut("settled")
        {
            settled.insert("distinct_answers".to_owned(), Value::Integer(1));
            settled.insert("spread_parts_per_million".to_owned(), Value::Integer(0));
        }
        let text = rendered(&warm, "m").join("\n");
        assert!(
            text.contains("3 seeds at temperature 0.700 gave 1 answer, to the character"),
            "{text}"
        );
    }

    /// A document in a file that is not there is said with the path, not
    /// analysed as empty (A2).
    #[test]
    fn a_missing_file_is_said_with_its_path() {
        let asked = Asked {
            file: Some("/nowhere/persona.md"),
            ..Asked::default()
        };
        let why = document(&asked).expect_err("a file that is not there");
        assert!(
            why.contains("/nowhere/persona.md could not be read"),
            "{why}"
        );
        let inline = Asked {
            prompt: Some("A. B."),
            ..Asked::default()
        };
        assert_eq!(document(&inline).as_deref(), Ok("A. B."));
    }
}
