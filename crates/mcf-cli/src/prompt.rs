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
#[derive(Debug, Clone, Default)]
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
    /// The further readings asked for (B-434, B-435).
    pub extras: mcf_serve::prompt::Extras,
    /// How the turn is framed, where the person asked for a system turn or
    /// one of the template's switches: a persona is read under the frame it
    /// will be used in (B-455).
    pub turn: mcf_serve::turn::Turn,
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
        extras: asked.extras,
        turn: asked.turn.asks_anything().then(|| asked.turn.clone()),
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
    // **Printed as it comes.** A report is many generations, and the daemon
    // says which it is on before each; a terminal that showed nothing until
    // the report showed a run that looked stopped (B-479, A7). The report
    // is the line that says it is done.
    let Some(answer) = the_report_as_it_comes(&connection) else {
        return Response {
            text: "mcf: MCF did not answer".to_owned(),
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

/// Reads the daemon's lines, printing each step as it is announced, and
/// returns the line that ends the run: the report, or the refusal. `None`
/// where the connection ended with neither.
fn the_report_as_it_comes(connection: &UnixStream) -> Option<Answer> {
    for read in BufReader::new(connection).lines() {
        let read = read.ok()?;
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served || matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            return Some(answer);
        }
        if let Some(step) = mcf_serve::prompt::step_said(&answer.body) {
            println!("{step}");
            let _flushed = std::io::stdout().flush();
        }
    }
    None
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

/// A share of an answer, written out.
///
/// Integer arithmetic, because the workspace ships no floating point: a NaN
/// that reaches a record is a figure nobody can compare, and `as_percent`
/// above has split parts per million into whole and tenth for the same reason
/// since this file was written.
pub(crate) fn percent(parts_per_million: i64) -> String {
    let (whole, tenth) = as_percent(parts_per_million);
    format!("{whole}.{tenth}%")
}

/// A difference of two shares, with its sign: `+2.0`, `-1.3`, `0.0`.
fn signed_percent(difference: i64) -> String {
    let (whole, tenth) = as_percent(difference.abs());
    match difference.signum() {
        1 => format!("+{whole}.{tenth}"),
        -1 => format!("-{whole}.{tenth}"),
        _ => format!("{whole}.{tenth}"),
    }
}

/// Ten cells of ten per cent, so the eye finds the parts that moved the
/// answer without reading a column of numbers.
fn bar(parts_per_million: i64) -> String {
    let filled = usize::try_from(tenths_of_the_answer(parts_per_million))
        .unwrap_or(0)
        .min(10);
    "#".repeat(filled) + &"·".repeat(10_usize.saturating_sub(filled))
}

/// A column of a table: what heads it, and whether its cells sit against
/// the right edge, as figures do, or the left, as text does.
struct Column {
    head: String,
    right: bool,
}

fn figure(head: &str) -> Column {
    Column {
        head: head.to_owned(),
        right: true,
    }
}

fn text(head: &str) -> Column {
    Column {
        head: head.to_owned(),
        right: false,
    }
}

/// Rows under their heads, each column as wide as its widest cell and the
/// last one unpadded. A row with fewer cells than columns is drawn as far
/// as it goes; one starting with `→` is a line under the row before it.
fn table(columns: &[Column], rows: &[Vec<String>]) -> Vec<String> {
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(at, column)| {
            rows.iter()
                .filter(|row| !row.first().is_some_and(|cell| cell.starts_with('→')))
                .filter_map(|row| row.get(at))
                .map(|cell| cell.chars().count())
                .chain(std::iter::once(column.head.chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let heads: Vec<String> = columns.iter().map(|column| column.head.clone()).collect();
    let mut lines = vec![laid_out(&heads, columns, &widths)];
    for row in rows {
        if row.first().is_some_and(|cell| cell.starts_with('→')) {
            lines.push(format!("       {}", row.join(" ")));
        } else {
            lines.push(laid_out(row, columns, &widths));
        }
    }
    lines
}

fn laid_out(row: &[String], columns: &[Column], widths: &[usize]) -> String {
    let mut line = String::from("  ");
    let last = columns.len().saturating_sub(1);
    for (at, cell) in row.iter().enumerate() {
        let width = widths.get(at).copied().unwrap_or(0);
        let right = columns.get(at).is_some_and(|column| column.right);
        let pad = " ".repeat(width.saturating_sub(cell.chars().count()));
        if at == last {
            line.push_str(cell);
        } else if right {
            line.push_str(&pad);
            line.push_str(cell);
            line.push_str("  ");
        } else {
            line.push_str(cell);
            line.push_str(&pad);
            line.push_str("  ");
        }
    }
    line.trim_end().to_owned()
}

/// A section's head: a label, and the conditions its figures were read
/// under, `·`-separated (§3.4).
fn head(label: &str, conditions: &[String]) -> String {
    if conditions.is_empty() {
        label.to_owned()
    } else {
        format!("{label:<9}{}", conditions.join(" · "))
    }
}

/// Where the answer's first token ranked, as the window's cell (B-072).
fn rank_cell(held: Option<&Value>, depth: i64) -> String {
    mcf_desk::held_mark(held, depth)
}

/// How much of the answer's opening stayed, as the window's cell (B-072).
fn open_cell(held: Option<&Value>) -> String {
    mcf_desk::open_mark(held)
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

/// A part's text, cut to a width the column holds.
fn cut(said: &str, width: usize) -> String {
    let line = said.trim().lines().next().unwrap_or_default();
    let mut shown: String = line.chars().take(width).collect();
    if shown.chars().count() < line.chars().count() || said.trim().lines().count() > 1 {
        shown.push('…');
    }
    shown
}

/// A row under a table's row: what the model wrote, where it was carried.
fn answer_row(read: &Value, key: &str) -> Option<Vec<String>> {
    read.get(key)
        .and_then(Value::as_text)
        .map(|answer| vec![format!("→ {}", first_line_of(answer))])
}

fn integer(held: &Value, key: &str) -> i64 {
    held.get(key).and_then(Value::as_integer).unwrap_or(0)
}

fn moved_of(read: &Value) -> i64 {
    integer(read, "moved_parts_per_million")
}

fn clauses_of(body: &Value) -> &[Value] {
    body.get("clauses").and_then(Value::as_list).unwrap_or(&[])
}

fn part_text(body: &Value, at: usize) -> String {
    clauses_of(body)
        .get(at)
        .and_then(|clause| clause.get("text"))
        .and_then(Value::as_text)
        .map(|said| cut(said, 64))
        .unwrap_or_default()
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

/// The conditions of the run, a figure a line (§3.4, §3.15): what the
/// text was cut into and who decided, how it reached the model, what every
/// generation was allowed to be, what the generations were spent on, the
/// floor and how it was drawn, whether the floor leaves the rows readable,
/// and where the figures survive this page.
fn conditions(body: &Value) -> Vec<String> {
    let unit = unit_of(body);
    let text = |key: &str| body.get(key).and_then(Value::as_text);
    let mut rows: Vec<(&str, String)> = Vec::new();
    rows.push((
        "unit",
        match text("unit_chosen_by") {
            Some(by) => format!("{unit} · {by}"),
            None => unit.to_owned(),
        },
    ));
    if let Some(addressed) = text("addressed_as") {
        rows.push(("addressed", addressed.to_owned()));
    }
    // What was asked of the model's own template, where anything was: every
    // figure below is of the prompt inside that turn (B-455, §3.4).
    if let Some(asked) = text("asked_as") {
        rows.push(("asked", asked.to_owned()));
    }
    // **Who read the prompt** (B-441): the engine that answered it, named,
    // or why there is no count at all (A7).
    let read_by = text("read_by").unwrap_or("not recorded");
    let tokens = integer(body, "prompt_tokens");
    if tokens > 0 {
        rows.push(("prompt", format!("{tokens} tokens · read by {read_by}")));
    } else if let Some(refused) = text("prompt_tokens_refused") {
        rows.push((
            "prompt",
            format!("not counted · {refused} · read by {read_by}"),
        ));
    }
    let limit = integer(body, "token_limit");
    if limit > 0 {
        rows.push(("cap", format!("{limit} tokens a generation")));
    }
    if let Some(sampler) = text("sampler") {
        rows.push((
            "sampler",
            sampler.split(" — ").next().unwrap_or(sampler).to_owned(),
        ));
    }
    rows.push(("generations", what_the_generations_were(body)));
    rows.push(("floor", the_floor(body, unit)));
    let floor = integer(body, "floor_parts_per_million");
    // **Where the floor swamps the column, that is the finding** (§3.15,
    // A7, F147): said before the rows, in a line of its own.
    rows.push((
        "separable",
        if floor < 500_000 {
            "yes".to_owned()
        } else {
            format!(
                "NO · floor {} ≥ 50.0% · rows below say this run did not work, not an ordering \
                 · try a prompt with a short answer, or one part of the work at a time",
                percent(floor)
            )
        },
    ));
    rows.push((
        "record",
        match text("recorded") {
            Some(id) => {
                format!("{id} · figures and conditions, length and digest, no text · mcf log")
            }
            None => "NOT RECORDED · the daemon could not write it · this page is the only copy"
                .to_owned(),
        },
    ));
    rows.into_iter()
        .map(|(label, value)| format!("  {label:<13}{value}"))
        .collect()
}

/// What the generations were spent on — each thing that cost some, and
/// only the things this run asked for (A19, §3.4).
fn what_the_generations_were(body: &Value) -> String {
    let list = |key: &str| body.get(key).and_then(Value::as_list).map(<[Value]>::len);
    let mut spent = vec![
        format!("{}", integer(body, "generations")),
        "as written 1".to_owned(),
        format!("removed {}", clauses_of(body).len()),
        match list("floors") {
            Some(positions) => format!("control {positions}"),
            None => "control 1".to_owned(),
        },
    ];
    if let Some(alone) = list("alone") {
        spent.push(format!("alone {}", alone.saturating_add(1)));
    }
    if let Some(prefixes) = list("prefixes") {
        spent.push(format!("prefixes {prefixes}"));
    }
    if let Some(swaps) = list("swaps") {
        spent.push(format!("swaps {swaps}"));
    }
    // The forms rendered, not the forms: one the prompt is already in
    // cost nothing (B-444).
    if let Some(forms) = body.get("forms").and_then(Value::as_list) {
        spent.push(format!(
            "forms {}",
            forms
                .iter()
                .filter(|formed| formed.get("not_rendered").is_none())
                .count()
        ));
    }
    if let Some(settled) = body
        .get("settled")
        .filter(|held| matches!(held, Value::Map(_)))
    {
        spent.push(format!("seeds {}", integer(settled, "seeds_asked")));
    }
    spent.join(" · ")
}

/// The floor's line: the figure, and how it was drawn — one draw before the
/// last part, or at every position with its spread (B-434, §3.4).
fn the_floor(body: &Value, unit: &str) -> String {
    let floor = percent(integer(body, "floor_parts_per_million"));
    let depth = integer(body, "forced_depth");
    let held = match body.get("floor_held") {
        Some(held) if !matches!(held, Value::Null) => format!(
            " · control in: 1st {} · open {}",
            rank_cell(Some(held), depth),
            open_cell(Some(held))
        ),
        // Not taken, and why: a dash alone sent a reader to look for an
        // engine that was running the whole time (A2, F160).
        _ => format!(
            " · control in: not taken · {}",
            body.get("held_refused")
                .and_then(Value::as_text)
                .unwrap_or("needs the served engine")
        ),
    };
    match body
        .get("floor_spread")
        .filter(|spread| !matches!(spread, Value::Null))
    {
        Some(spread) => format!(
            "{floor} · drawn at {} · {}–{} · middle {}{held}",
            count_of(
                i64::try_from(
                    body.get("floors")
                        .and_then(Value::as_list)
                        .map_or(0, <[Value]>::len)
                )
                .unwrap_or(0),
                "position"
            ),
            percent(integer(spread, "least_parts_per_million")),
            percent(integer(spread, "most_parts_per_million")),
            percent(integer(spread, "middle_parts_per_million")),
        ),
        None => format!(
            "{floor} · 1 draw · control before the last {unit} · --floors: every position{held}"
        ),
    }
}

/// The parts removed in turn (§3.8, B-429): a row a part, the control's
/// row under them, and the parts at or under the floor named.
fn removed(body: &Value) -> Vec<String> {
    let unit = unit_of(body);
    let depth = integer(body, "forced_depth");
    let floor = integer(body, "floor_parts_per_million");
    let clauses = clauses_of(body);
    let mut lines = vec![head(
        "IMPACT",
        &[
            format!("answer moved without each {unit}"),
            "moved = words changed, punctuation aside".to_owned(),
            "seed held".to_owned(),
            "an ordering, not relevance".to_owned(),
            "own = pieces the model would have written".to_owned(),
        ],
    )];
    if clauses.is_empty() {
        lines.push(format!("  one {unit} · nothing to remove"));
        lines.push(String::new());
        return lines;
    }
    let by_part = body.get("expected_by_part");
    // The thought's cost is a column only where there was a thought: a
    // column of dashes for every model that does not think would be a
    // column about MCF (B-455, §3.15).
    let thinking = clauses
        .iter()
        .any(|clause| thought_of(clause, "thought").is_some());
    let mut rows = Vec::new();
    for (at, clause) in clauses.iter().enumerate() {
        let moved = moved_of(clause);
        let mut row = vec![
            format!("{}", at.saturating_add(1)),
            percent(moved),
            bar(moved),
            signed_percent(moved.saturating_sub(floor)),
            rank_cell(clause.get("held"), depth),
            open_cell(clause.get("held")),
            own_cell(by_part, at),
        ];
        if thinking {
            row.push(thought_cell(clause));
        }
        row.push(part_text(body, at));
        rows.push(row);
        if clause.get("changed").and_then(Value::as_bool) == Some(true)
            && let Some(row) = answer_row(clause, "without")
        {
            rows.push(row);
        }
    }
    let mut control = vec![
        "ctl".to_owned(),
        percent(floor),
        bar(floor),
        "floor".to_owned(),
        rank_cell(body.get("floor_held"), depth),
        open_cell(body.get("floor_held")),
        "—".to_owned(),
    ];
    if thinking {
        control.push(thought_cell_of(thought_of(body, "floor_thought")));
    }
    control.push("control sentence".to_owned());
    rows.push(control);
    let mut columns = vec![
        figure("#"),
        figure("moved"),
        text(""),
        figure("vs floor"),
        figure("1st"),
        figure("open"),
        figure("own"),
    ];
    if thinking {
        columns.push(figure("thought"));
    }
    columns.push(text(unit));
    lines.extend(table(&columns, &rows));
    lines.extend(under_the_floor(body, floor));
    lines.extend(floors(body, unit));
    let over = integer(body, "clauses_over_the_cap");
    if over > 0 {
        lines.push(format!(
            "  not removed   {over} · --most {}",
            integer(body, "most").saturating_add(over)
        ));
    }
    lines.extend(least_expected(body));
    lines.push(String::new());
    lines
}

/// What the model spent thinking without this part, in tokens; a dash
/// where nothing was counted (A7).
fn thought_cell(clause: &Value) -> String {
    thought_cell_of(thought_of(clause, "thought"))
}

/// The same, from a figure already read.
fn thought_cell_of(thought: Option<i64>) -> String {
    thought.map_or_else(|| "—".to_owned(), |thought| thought.to_string())
}

/// A part's share of the rank reading, `first choice/pieces`; a dash
/// where the reading was not taken or the part fell past it (A7).
fn own_cell(by_part: Option<&Value>, at: usize) -> String {
    by_part
        .and_then(|grouped| grouped.get("parts"))
        .and_then(Value::as_list)
        .and_then(|parts| parts.get(at))
        .map_or_else(
            || "—".to_owned(),
            |part| {
                format!(
                    "{}/{}",
                    integer(part, "first_choice"),
                    integer(part, "tokens")
                )
            },
        )
}

/// Which parts sit at or under the floor — and, where every one did and the
/// floor was nought, that the prompt did not steer this model (A7).
fn under_the_floor(body: &Value, floor: i64) -> Vec<String> {
    let clauses = clauses_of(body);
    let quiet: Vec<String> = clauses
        .iter()
        .enumerate()
        .filter(|(_, clause)| moved_of(clause) <= floor)
        .map(|(at, _)| format!("#{}", at.saturating_add(1)))
        .collect();
    let mut lines = Vec::new();
    // **Every removal giving the same answer is a finding, and it reads
    // like a broken tool** (A7).
    if floor == 0 && quiet.len() == clauses.len() {
        lines.push(
            "  same answer  every removal and the control · the prompt did not steer this model"
                .to_owned(),
        );
    }
    lines.push(format!(
        "  at/under floor  {}",
        if quiet.is_empty() {
            "none".to_owned()
        } else {
            format!("{} · {}", quiet.len(), quiet.join(" "))
        }
    ));
    lines
}

/// The floor at every position, where it was drawn (B-434): a row a
/// position, and how many parts sit under the widest of them.
fn floors(body: &Value, unit: &str) -> Vec<String> {
    let Some(floors) = body.get("floors").and_then(Value::as_list) else {
        return Vec::new();
    };
    let depth = integer(body, "forced_depth");
    let rows: Vec<Vec<String>> = floors
        .iter()
        .map(|at| {
            let position = integer(at, "position");
            let place = if position.saturating_add(1) == i64::try_from(floors.len()).unwrap_or(0) {
                format!("after #{position}")
            } else {
                format!("before #{}", position.saturating_add(1))
            };
            vec![
                place,
                percent(moved_of(at)),
                bar(moved_of(at)),
                rank_cell(at.get("held"), depth),
                open_cell(at.get("held")),
            ]
        })
        .collect();
    let most = body
        .get("floor_spread")
        .map_or(0, |spread| integer(spread, "most_parts_per_million"));
    let under = clauses_of(body)
        .iter()
        .filter(|clause| moved_of(clause) <= most)
        .count();
    let mut lines = vec![
        String::new(),
        head(
            "FLOORS",
            &[format!(
                "control at every position · a {unit} the control matched somewhere is not shown to steer"
            )],
        ),
    ];
    lines.extend(table(
        &[
            text("control"),
            figure("moved"),
            text(""),
            figure("1st"),
            figure("open"),
        ],
        &rows,
    ));
    lines.push(format!("  at/under widest  {under}"));
    lines
}

/// Each part asked as the whole prompt in turn (B-435), read against the
/// control alone; not asked is said with the flag and its cost.
fn alone(body: &Value) -> Vec<String> {
    let unit = unit_of(body);
    let depth = integer(body, "forced_depth");
    let Some(alone) = body.get("alone").and_then(Value::as_list) else {
        return Vec::new();
    };
    let mut lines = vec![head(
        "ALONE",
        &[
            format!("each {unit} as the whole prompt"),
            "vs the answer as written".to_owned(),
            "low = carries it alone".to_owned(),
        ],
    )];
    let mut rows = Vec::new();
    for (at, read) in alone.iter().enumerate() {
        rows.push(vec![
            format!("{}", at.saturating_add(1)),
            percent(moved_of(read)),
            bar(moved_of(read)),
            rank_cell(read.get("held"), depth),
            open_cell(read.get("held")),
            part_text(body, at),
        ]);
        rows.extend(answer_row(read, "answer"));
    }
    let control = body
        .get("alone_floor")
        .filter(|held| matches!(held, Value::Map(_)));
    if let Some(control) = control {
        rows.push(vec![
            "ctl".to_owned(),
            percent(moved_of(control)),
            bar(moved_of(control)),
            rank_cell(control.get("held"), depth),
            open_cell(control.get("held")),
            "control sentence alone".to_owned(),
        ]);
        rows.extend(answer_row(control, "answer"));
    }
    lines.extend(table(
        &[
            figure("#"),
            figure("moved"),
            text(""),
            figure("1st"),
            figure("open"),
            text(unit),
        ],
        &rows,
    ));
    lines.push(match control {
        Some(control) => format!(
            "  as far as control or further  {}",
            alone
                .iter()
                .filter(|read| moved_of(read) >= moved_of(control))
                .count()
        ),
        None => "  control alone  not read · nothing to read these against".to_owned(),
    });
    lines.push(String::new());
    lines
}

/// The prompt grown from the front (B-436): a row a prefix, and the first
/// within the floor of the answer as written.
fn prefixes(body: &Value) -> Vec<String> {
    let unit = unit_of(body);
    let depth = integer(body, "forced_depth");
    let floor = integer(body, "floor_parts_per_million");
    let Some(prefixes) = body.get("prefixes").and_then(Value::as_list) else {
        return Vec::new();
    };
    let mut lines = vec![head(
        "PREFIXES",
        &[
            format!("prompt grown a {unit} at a time from the front"),
            "vs the answer as written".to_owned(),
            "low = already had it".to_owned(),
        ],
    )];
    let mut rows = Vec::new();
    let mut arrived = None;
    for (at, read) in prefixes.iter().enumerate() {
        let kept = at.saturating_add(1);
        if arrived.is_none() && moved_of(read) <= floor {
            arrived = Some(kept);
        }
        rows.push(vec![
            format!("1–{kept}"),
            percent(moved_of(read)),
            bar(moved_of(read)),
            rank_cell(read.get("held"), depth),
            open_cell(read.get("held")),
            part_text(body, at),
        ]);
        rows.extend(answer_row(read, "answer"));
    }
    lines.extend(table(
        &[
            figure("parts"),
            figure("moved"),
            text(""),
            figure("1st"),
            figure("open"),
            text("through"),
        ],
        &rows,
    ));
    lines.push(format!(
        "  within floor {}  {}",
        percent(floor),
        match arrived {
            Some(kept) => format!("first at 1–{kept} · not a claim the rest is idle"),
            None => format!("none short of the whole · the last {unit} still moved the answer"),
        }
    ));
    lines.push(String::new());
    lines
}

/// Neighbouring parts swapped (B-437): a row a pair, and how many of them
/// moved the answer past the floor — the pairs whose order the model reads.
fn swaps(body: &Value) -> Vec<String> {
    let unit = unit_of(body);
    let depth = integer(body, "forced_depth");
    let floor = integer(body, "floor_parts_per_million");
    let Some(swaps) = body.get("swaps").and_then(Value::as_list) else {
        return Vec::new();
    };
    let mut lines = vec![head(
        "SWAPS",
        &[
            format!("each {unit} and the next in each other's places"),
            "vs the answer as written".to_owned(),
            "high = the order carries it".to_owned(),
        ],
    )];
    let rows: Vec<Vec<String>> = swaps
        .iter()
        .enumerate()
        .flat_map(|(at, read)| {
            let mut rows = vec![vec![
                mcf_desk::pair_mark(at),
                percent(moved_of(read)),
                bar(moved_of(read)),
                rank_cell(read.get("held"), depth),
                open_cell(read.get("held")),
                part_text(body, at),
            ]];
            rows.extend(answer_row(read, "answer"));
            rows
        })
        .collect();
    lines.extend(table(
        &[
            figure("pair"),
            figure("moved"),
            text(""),
            figure("1st"),
            figure("open"),
            text("first of the pair"),
        ],
        &rows,
    ));
    lines.push(format!(
        "  past floor {}  {} of {} · order read, not words",
        percent(floor),
        swaps.iter().filter(|read| moved_of(read) > floor).count(),
        swaps.len()
    ));
    lines.push(String::new());
    lines
}

/// The same parts in each form (B-444): a row a form read, how far its
/// answer sat from the answer as written, and how many forms moved it
/// past the floor — the forms this model reads as forms. A form not
/// rendered is a line under the table saying why, never a row at nought
/// (A7).
fn forms(body: &Value) -> Vec<String> {
    let depth = integer(body, "forced_depth");
    let floor = integer(body, "floor_parts_per_million");
    let Some(forms) = body.get("forms").and_then(Value::as_list) else {
        return Vec::new();
    };
    let mut lines = vec![head(
        "FORMS",
        &[
            "the same parts, dressed another way".to_owned(),
            "vs the answer as written".to_owned(),
            "high = the form carries it".to_owned(),
        ],
    )];
    let read: Vec<&Value> = forms
        .iter()
        .filter(|formed| formed.get("not_rendered").is_none())
        .collect();
    let rows: Vec<Vec<String>> = read
        .iter()
        .flat_map(|formed| {
            let mut rows = vec![vec![
                form_name(formed),
                percent(moved_of(formed)),
                bar(moved_of(formed)),
                rank_cell(formed.get("held"), depth),
                open_cell(formed.get("held")),
            ]];
            rows.extend(answer_row(formed, "answer"));
            rows
        })
        .collect();
    if !rows.is_empty() {
        lines.extend(table(
            &[
                text("form"),
                figure("moved"),
                text(""),
                figure("1st"),
                figure("open"),
            ],
            &rows,
        ));
    }
    for formed in forms
        .iter()
        .filter(|formed| formed.get("not_rendered").is_some())
    {
        lines.push(format!(
            "  {:<10} not rendered · {}",
            form_name(formed),
            formed
                .get("not_rendered")
                .and_then(Value::as_text)
                .unwrap_or("")
        ));
    }
    lines.push(format!(
        "  past floor {}  {} of {} · form read, not words",
        percent(floor),
        read.iter()
            .filter(|formed| moved_of(formed) > floor)
            .count(),
        read.len()
    ));
    lines.push(String::new());
    lines
}

/// The form a served row names.
fn form_name(formed: &Value) -> String {
    formed
        .get("form")
        .and_then(Value::as_text)
        .unwrap_or("")
        .to_owned()
}

/// A reading that was not asked for, said as not asked with the flag that
/// asks it and what it would cost (A7). The forms' cost is a ceiling, and
/// says so: a form the prompt is already in is not asked (B-444).
fn not_asked(extra: mcf_serve::prompt::Extra, body: &Value) -> Vec<String> {
    let removed = clauses_of(body).len();
    let parts =
        removed.saturating_add(usize::try_from(integer(body, "clauses_over_the_cap")).unwrap_or(0));
    let asks = if extra.at_most() {
        format!(
            "{} — at most: a form the prompt is already in is not asked",
            extra.asks()
        )
    } else {
        extra.asks().to_owned()
    };
    vec![
        format!("--{}", extra.name()),
        count_of(
            i64::try_from(extra.generations(parts, removed)).unwrap_or(0),
            "generation",
        ),
        asks,
    ]
}

/// The readings not asked for, each with its flag and its cost, on one
/// table rather than four empty sections (§3.15, B-443). Nothing where
/// every one was asked.
fn more(body: &Value) -> Vec<String> {
    let mut rows = Vec::new();
    if body.get("alone").and_then(Value::as_list).is_none() {
        rows.push(not_asked(mcf_serve::prompt::Extra::Alone, body));
    }
    if body.get("prefixes").and_then(Value::as_list).is_none() {
        rows.push(not_asked(mcf_serve::prompt::Extra::Prefixes, body));
    }
    if body.get("swaps").and_then(Value::as_list).is_none() {
        rows.push(not_asked(mcf_serve::prompt::Extra::Swaps, body));
    }
    if body.get("forms").and_then(Value::as_list).is_none() {
        rows.push(not_asked(mcf_serve::prompt::Extra::Forms, body));
    }
    if !matches!(body.get("settled"), Some(Value::Map(_))) {
        rows.push(vec![
            "--temperature <t>".to_owned(),
            "3 generations".to_owned(),
            "the same prompt under three seeds at t, to say whether the answer is settled; at \
             temperature 0 the seed changes nothing, and there is no house temperature — the \
             model's own: mcf explain"
                .to_owned(),
        ]);
    }
    if rows.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![head("MORE", &["not asked".to_owned()])];
    lines.extend(table(&[text("flag"), figure("cost"), text("asks")], &rows));
    lines.push(String::new());
    lines
}

/// Whether several seeds gave several answers, under the temperature it
/// was asked at — or not asked, which is never *settled* (A7, B-431, B60).
fn seeds(body: &Value) -> Vec<String> {
    let Some(settled) = body
        .get("settled")
        .filter(|held| matches!(held, Value::Map(_)))
    else {
        return Vec::new();
    };
    let temperature = settled
        .get("temperature")
        .and_then(Value::as_text)
        .unwrap_or("?");
    vec![
        head(
            "SEEDS",
            &[
                format!(
                    "{} at temperature {temperature}",
                    count_of(integer(settled, "seeds_asked"), "seed")
                ),
                format!("distinct answers {}", integer(settled, "distinct_answers")),
                format!(
                    "farthest apart {}",
                    percent(integer(settled, "spread_parts_per_million"))
                ),
                format!(
                    "farthest from greedy {}",
                    percent(integer(settled, "from_greedy_parts_per_million"))
                ),
                cut_line(settled),
                "a fact about the pair, not a merit of the prompt".to_owned(),
            ],
        ),
        String::new(),
    ]
}

/// How each seeded draw was cut before it was taken, and whose cut it was
/// (B-440): `top_k 20 · top_p 0.950 · min_p off · declared by the file`. A
/// report from before the cut was stated says so rather than *off* (A7).
fn cut_line(settled: &Value) -> String {
    let named = |key: &str| settled.get(key).and_then(Value::as_text);
    match (
        named("top_k"),
        named("top_p"),
        named("min_p"),
        named("truncation"),
    ) {
        (Some(top_k), Some(top_p), Some(min_p), Some(whose)) => {
            format!("cut top_k {top_k} · top_p {top_p} · min_p {min_p} · {whose}")
        }
        _ => "cut not recorded".to_owned(),
    }
}

/// How the model received each word (B-443): where its first piece ranked
/// in the model's own choice, how many of its pieces the model would have
/// written itself, and the part it begins in. Least expected first, a
/// word the model would have written whole left off the table; past the
/// depth read is a bound, not an absence (A7). Not taken is said with why.
fn expected(body: &Value) -> Vec<String> {
    let Some(by_word) = body
        .get("expected_by_word")
        .filter(|held| matches!(held, Value::Map(_)))
    else {
        return body
            .get("expected_refused")
            .and_then(Value::as_text)
            .map(|why| {
                vec![
                    head("EXPECTED", &["not taken".to_owned(), why.to_owned()]),
                    String::new(),
                ]
            })
            .unwrap_or_default();
    };
    let depth = integer(body, "ranked_depth");
    let words = by_word.get("words").and_then(Value::as_list).unwrap_or(&[]);
    let mut surprising: Vec<(i64, &Value)> = Vec::new();
    let (mut whole, mut own, mut pieces, mut past, mut unread, mut no_context) =
        (0_i64, 0_i64, 0_i64, 0_i64, 0_i64, 0_i64);
    for word in words {
        // **A word no piece fell in was not read, and is not a first
        // choice.** Nought of nought counted as every piece the model's own,
        // and a report whose placement had failed on every piece said
        // *first choice 11/11 words · 0/0 pieces* (A7, F160).
        if integer(word, "pieces") == 0 {
            unread = unread.saturating_add(1);
            continue;
        }
        // **A word nothing preceded was not ranked, and is not a first
        // choice either.** Only its read pieces are counted (F160).
        let read = integer(word, "pieces").saturating_sub(integer(word, "unread"));
        if read == 0 {
            no_context = no_context.saturating_add(1);
            continue;
        }
        pieces = pieces.saturating_add(read);
        own = own.saturating_add(integer(word, "first_choice"));
        // Outside the list asked for: a bound, not an absence (A7).
        let rank = word
            .get("rank")
            .and_then(Value::as_integer)
            .unwrap_or(i64::MAX);
        if rank == i64::MAX {
            past = past.saturating_add(1);
        }
        if integer(word, "first_choice") == read {
            whole = whole.saturating_add(1);
        } else {
            surprising.push((rank, word));
        }
    }
    surprising.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    let mut conditions = vec![
        "how the model received each word".to_owned(),
        "rank = its first piece in the model's own choice, 1 = it would have written that"
            .to_owned(),
        "own = pieces it would have written".to_owned(),
        format!("# = {}", unit_of(body)),
    ];
    // Under what addressing (§3.4, B-429).
    if let Some(under) = body.get("ranked_under").and_then(Value::as_text) {
        conditions.push(format!("read under {under}"));
    }
    if let Some(by) = body.get("read_by").and_then(Value::as_text) {
        conditions.push(format!("read by {by}"));
    }
    let mut lines = vec![head("EXPECTED", &conditions)];
    let rows: Vec<Vec<String>> = surprising
        .iter()
        .take(MOST_WORDS)
        .map(|(rank, word)| word_row(*rank, word, depth))
        .collect();
    lines.extend(table(
        &[figure("rank"), figure("own"), figure("#"), text("word")],
        &rows,
    ));
    let left = surprising.len().saturating_sub(MOST_WORDS);
    if left > 0 {
        lines.push(format!("  {left} more · --json"));
    }
    lines.push(format!(
        "  first choice  {whole}/{} words · {own}/{pieces} pieces",
        words.len()
    ));
    if past > 0 {
        lines.push(format!(
            "  past depth  {} · rank {depth} or further",
            count_of(past, "word")
        ));
    }
    if no_context > 0 {
        lines.push(format!(
            "  no context  {} · first in the prompt, nothing before it to rank against",
            count_of(no_context, "word")
        ));
    }
    if unread > 0 {
        lines.push(format!("  in no piece  {}", count_of(unread, "word")));
    }
    let unplaced = integer(by_word, "unplaced");
    if unplaced > 0 {
        lines.push(format!("  in no word  {}", count_of(unplaced, "piece")));
    }
    lines.push(String::new());
    lines
}

/// How many words the table shows before it says how many more there are.
const MOST_WORDS: usize = 20;

/// One word's row: its first piece's rank, its pieces the model would have
/// written itself over its pieces, the part it begins in, and the word.
fn word_row(rank: i64, word: &Value, depth: i64) -> Vec<String> {
    vec![
        if rank == i64::MAX {
            format!(">{depth}")
        } else {
            rank.to_string()
        },
        format!(
            "{}/{}",
            integer(word, "first_choice"),
            integer(word, "pieces").saturating_sub(integer(word, "unread"))
        ),
        word.get("part")
            .and_then(Value::as_integer)
            .map_or_else(|| "—".to_owned(), |part| part.to_string()),
        format!(
            "{:?}",
            word.get("text")
                .and_then(Value::as_text)
                .unwrap_or_default()
        ),
    ]
}

/// The part the model least expected, by its share of first-choice pieces
/// (B-433). Ties name nobody, and one part is nobody's least (A19). Shares
/// are compared crosswise, so no division is done.
fn least_expected(body: &Value) -> Vec<String> {
    let Some(parts) = body
        .get("expected_by_part")
        .and_then(|grouped| grouped.get("parts"))
        .and_then(Value::as_list)
    else {
        return Vec::new();
    };
    let mut least: Option<(usize, i64, i64)> = None;
    let mut tied = false;
    for (at, part) in parts.iter().enumerate() {
        let (tokens, first) = (integer(part, "tokens"), integer(part, "first_choice"));
        if tokens == 0 {
            continue;
        }
        match least {
            Some((_, held_first, held_tokens)) => {
                let mine = first.saturating_mul(held_tokens);
                let theirs = held_first.saturating_mul(tokens);
                if mine < theirs {
                    least = Some((at, first, tokens));
                    tied = false;
                } else if mine == theirs {
                    tied = true;
                }
            }
            None => least = Some((at, first, tokens)),
        }
    }
    let mut lines = vec![format!(
        "  least expected  {}",
        match least.filter(|_| !tied && parts.len() > 1) {
            Some((at, _, _)) => format!("#{}", at.saturating_add(1)),
            None => "tied".to_owned(),
        }
    )];
    let unplaced = body
        .get("expected_by_part")
        .map_or(0, |grouped| integer(grouped, "unplaced"));
    if unplaced > 0 {
        lines.push(format!("  in no part  {}", count_of(unplaced, "piece")));
    }
    lines
}

/// The answer to the prompt as written, under the cap it was drawn to.
fn answer(body: &Value) -> Vec<String> {
    let baseline = body
        .get("baseline")
        .and_then(Value::as_text)
        .unwrap_or_default();
    let limit = integer(body, "token_limit");
    let mut conditions = vec!["as written".to_owned()];
    if limit > 0 {
        conditions.push(format!("cap {limit} tokens"));
    }
    conditions.extend(mcf_desk::answer_marks(body));
    // What the model spent before this answer began, where the turn gave it
    // a marker to think inside: a persona that costs three hundred tokens of
    // thinking costs them on every turn it is used (B-455, B-451).
    if let Some(thought) = thought_of(body, "baseline_thought") {
        conditions.push(format!("{} before it", count_of(thought, "token")));
    }
    let mut lines = vec![head("ANSWER", &conditions)];
    let written: Vec<&str> = baseline.lines().collect();
    // An empty answer is a figure, not a blank (A7, F160): Devstral, bare,
    // ended a build instruction at one token that printed as nothing.
    if baseline.trim().is_empty() {
        lines.push(format!("  {}", mcf_desk::NOTHING_WRITTEN));
    }
    for said in written.iter().take(12) {
        lines.push(format!("  {said}"));
    }
    if written.len() > 12 {
        lines.push(format!(
            "  … {} more · --json",
            count_of(
                i64::try_from(written.len().saturating_sub(12)).unwrap_or(0),
                "line"
            )
        ));
    }
    lines.push(String::new());
    lines
}

/// What was spent thinking, where it was counted at all: null is *not
/// counted*, which is not nought (A7).
fn thought_of(held: &Value, key: &str) -> Option<i64> {
    held.get(key).and_then(Value::as_integer)
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

/// The report: the conditions, then a table a reading, then the answer.
/// Figures and short labels; a not-asked reading is a line that says so
/// and names the flag (A7, §3.4, §3.15).
fn rendered(body: &Value, named: &str) -> Vec<String> {
    let mut lines = vec![format!("PROMPT · {}", header_name(named)), String::new()];
    lines.extend(conditions(body));
    lines.push(String::new());
    lines.extend(expected(body));
    lines.extend(removed(body));
    lines.extend(alone(body));
    lines.extend(prefixes(body));
    lines.extend(swaps(body));
    lines.extend(forms(body));
    lines.extend(seeds(body));
    lines.extend(more(body));
    lines.extend(answer(body));
    lines.push("  no verdict on the prompt · that needs a rater".to_owned());
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
                "unit":"sentence","unit_chosen_by":"text: no blank line",
                "most":3,
                "addressed_as":"one user turn, the whole prompt",
                "expected":[{"text":" are","rank":null,"engine_said":null}],"prompt_tokens":10,"ranked_depth":60,
                "read_by":"a test's tokenizer",
                "expected_by_part":{"parts":[{"tokens":4,"first_choice":1,"past_depth":1},
                   {"tokens":3,"first_choice":3,"past_depth":0},{"tokens":2,"first_choice":1,"past_depth":0},
                   {"tokens":5,"first_choice":4,"past_depth":0}],"unplaced":2},
                "expected_by_word":{"words":[
                   {"text":"Answer","pieces":1,"rank":17,"first_choice":0,"part":1},
                   {"text":"in","pieces":1,"rank":1,"first_choice":1,"part":1},
                   {"text":"one","pieces":1,"rank":1,"first_choice":1,"part":1},
                   {"text":"word.","pieces":2,"rank":1,"first_choice":1,"part":1},
                   {"text":"What","pieces":1,"rank":null,"first_choice":0,"part":2},
                   {"text":"colour","pieces":1,"rank":3,"first_choice":0,"part":2},
                   {"text":"is","pieces":1,"rank":1,"first_choice":1,"part":2},
                   {"text":"the","pieces":1,"rank":1,"first_choice":1,"part":null},
                   {"text":"room?","pieces":2,"rank":1,"first_choice":2,"part":2}],
                   "unplaced":1},
                "recorded":"01J0000000000000000000000A"}"#,
        )
        .expect("a well-formed report")
    }

    /// Three sentences at a hundred per cent tie on the bar; what the answer
    /// became and where its first token went are the columns that order
    /// them (B-429).
    #[test]
    fn a_tied_column_is_ordered_by_the_answer_and_the_forced_rank() {
        let text = removed(&body()).join("\n");
        assert!(
            text.contains("→ \"The room is painted blue.…\""),
            "the first line of the ablated answer is under its row, marked as cut: {text}"
        );
        assert!(
            text.contains("1  100.0%  ##########    +100.0   17   0/2  1/4  Answer in one word."),
            "{text}"
        );
        assert!(
            text.contains(
                "2  100.0%  ##########    +100.0  >60   0/2  3/3  What colour is the room?"
            ),
            "outside the depth read is a bound, not a rank: {text}"
        );
        assert!(
            text.contains("3    0.0%  ··········       0.0    1   2/2  1/2  You are careful."),
            "{text}"
        );
        assert!(
            !text.contains("→ \"Blue.\""),
            "an unchanged answer is not repeated: {text}"
        );
        assert!(
            text.contains("ctl    0.0%  ··········     floor    1   2/2    —  control sentence"),
            "the control's own row is under the parts: {text}"
        );
        assert!(text.contains("at/under floor  1 · #3"), "{text}");
    }

    /// A reading nobody took is a dash and a reason, not a rank (A7).
    #[test]
    fn an_untaken_forced_reading_is_said_rather_than_ranked() {
        let mut held = body();
        if let Value::Map(fields) = &mut held {
            fields.insert("floor_held".to_owned(), Value::Null);
        }
        let text = conditions(&held).join("\n");
        assert!(
            text.contains("control in: not taken · needs the served engine"),
            "{text}"
        );
        if let Value::Map(fields) = &mut held {
            fields.insert(
                "held_refused".to_owned(),
                Value::text("no engine resolves this model: it does not fit"),
            );
        }
        let text = conditions(&held).join("\n");
        assert!(
            text.contains("control in: not taken · no engine resolves this model: it does not fit"),
            "{text}"
        );
        assert_eq!(rank_cell(Some(&Value::Null), 60), "—");
        assert_eq!(rank_cell(None, 60), "—");
        assert_eq!(open_cell(None), "—");
    }

    /// The addressing the ranks were read under is on the page (§3.4), and
    /// the words come least expected first — past the depth read before
    /// any rank, a word the model would have written whole not at all
    /// (B-443, A7).
    #[test]
    fn the_words_come_least_expected_first_under_what_they_were_read_under() {
        let text = expected(&body()).join("\n");
        assert!(
            text.contains("read under chatml — set by a probe"),
            "{text}"
        );
        assert!(text.contains("read by a test's tokenizer"), "{text}");
        assert!(text.contains("# = sentence"), "{text}");
        let rows: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.starts_with("  rank"))
            .skip(1)
            .take_while(|line| !line.starts_with("  first"))
            .collect();
        assert_eq!(
            rows,
            vec![
                "   >60  0/1  2  \"What\"",
                "    17  0/1  1  \"Answer\"",
                "     3  0/1  2  \"colour\"",
                "     1  1/2  1  \"word.\"",
            ],
            "{text}"
        );
        assert!(
            text.contains("first choice  5/9 words · 7/11 pieces"),
            "{text}"
        );
        assert!(
            text.contains("past depth  1 word · rank 60 or further"),
            "{text}"
        );
        assert!(text.contains("in no word  1 piece"), "{text}");
        assert!(!text.contains("more ·"), "{text}");
    }

    /// A reading nobody took is said with why, not drawn empty (A7); a
    /// long prompt's table stops and says how many words it left off.
    #[test]
    fn an_untaken_word_reading_says_why_and_a_long_one_says_how_many_more() {
        let mut none = body();
        if let Value::Map(fields) = &mut none {
            fields.insert("expected_by_word".to_owned(), Value::Null);
            fields.insert(
                "expected_refused".to_owned(),
                Value::text("needs the served engine"),
            );
        }
        let text = expected(&none).join("\n");
        assert!(
            text.starts_with("EXPECTED not taken · needs the served engine"),
            "{text}"
        );
        let mut long = body();
        if let Value::Map(fields) = &mut long {
            let words: Vec<Value> = (0..25)
                .map(|at| {
                    Value::map([
                        ("text", Value::text(format!("w{at}"))),
                        ("pieces", Value::Integer(1)),
                        ("rank", Value::Integer(2)),
                        ("first_choice", Value::Integer(0)),
                        ("part", Value::Integer(1)),
                    ])
                })
                .collect();
            fields.insert(
                "expected_by_word".to_owned(),
                Value::map([
                    ("words", Value::List(words)),
                    ("unplaced", Value::Integer(0)),
                ]),
            );
        }
        let text = expected(&long).join("\n");
        assert!(text.contains("  5 more · --json"), "{text}");
        assert!(text.contains("first choice  0/25 words"), "{text}");
        // A word no piece fell in is not a first choice, and is counted as
        // what it is (A7).
        let mut unread = body();
        if let Value::Map(fields) = &mut unread {
            let words: Vec<Value> = (0..3)
                .map(|at| {
                    Value::map([
                        ("text", Value::text(format!("w{at}"))),
                        ("pieces", Value::Integer(0)),
                        ("rank", Value::Null),
                        ("first_choice", Value::Integer(0)),
                        ("part", Value::Integer(1)),
                    ])
                })
                .collect();
            fields.insert(
                "expected_by_word".to_owned(),
                Value::map([
                    ("words", Value::List(words)),
                    ("unplaced", Value::Integer(12)),
                ]),
            );
        }
        let text = expected(&unread).join("\n");
        assert!(
            text.contains("first choice  0/3 words · 0/0 pieces"),
            "{text}"
        );
        assert!(text.contains("in no piece  3 words"), "{text}");
        assert!(text.contains("in no word  12 pieces"), "{text}");
        assert!(!text.contains("past depth"), "{text}");
        // A word nothing preceded was not ranked: it is counted apart, and
        // a word partly so is scored on the pieces that were read (F160).
        let mut first = body();
        if let Value::Map(fields) = &mut first {
            let word = |text: &str, pieces, first_choice, unread| {
                Value::map([
                    ("text", Value::text(text)),
                    ("pieces", Value::Integer(pieces)),
                    ("rank", Value::Null),
                    ("first_choice", Value::Integer(first_choice)),
                    ("unread", Value::Integer(unread)),
                    ("part", Value::Integer(1)),
                ])
            };
            fields.insert(
                "expected_by_word".to_owned(),
                Value::map([
                    (
                        "words",
                        Value::List(vec![word("Be", 1, 0, 1), word("terse.", 3, 2, 1)]),
                    ),
                    ("unplaced", Value::Integer(0)),
                ]),
            );
        }
        let text = expected(&first).join("\n");
        assert!(text.contains("no context  1 word ·"), "{text}");
        assert!(
            text.contains("first choice  1/2 words · 2/2 pieces"),
            "{text}"
        );
        assert!(!text.contains("\"Be\""), "{text}");
        assert!(!text.contains("in no piece"), "{text}");
    }

    /// The rank reading grouped by part names the part the model least
    /// expected under the impact table, with each part's share on its row,
    /// and counts the pieces that fell in no part (B-433, A7).
    #[test]
    fn the_report_names_the_part_the_model_least_expected() {
        let text = removed(&body()).join("\n");
        assert!(text.contains("open  own  sentence"), "{text}");
        assert!(text.contains("least expected  #1"), "{text}");
        assert!(text.contains("in no part  2 pieces"), "{text}");
        let mut none = body();
        if let Value::Map(fields) = &mut none {
            fields.insert("expected_by_part".to_owned(), Value::Null);
        }
        let text = removed(&none).join("\n");
        assert!(!text.contains("least expected"), "{text}");
        assert!(text.contains("2/2    —  You are careful."), "{text}");
    }

    /// The floor at every position is a spread on the floor's line and a
    /// table of draws with the parts under its widest; a report that drew
    /// it once says so and names the flag (B-434, §3.4).
    #[test]
    fn the_floor_is_a_spread_when_it_was_drawn_everywhere_and_one_draw_when_not() {
        let text = rendered(&body(), "m").join("\n");
        assert!(
            text.contains(
                "floor        0.0% · 1 draw · control before the last sentence · --floors: \
                 every position · control in: 1st 1 · open 2/2"
            ),
            "{text}"
        );
        assert!(!text.contains("FLOORS"), "{text}");
        let mut everywhere = body();
        if let Value::Map(fields) = &mut everywhere {
            let at = |position: i64, moved: i64, rank: Option<i64>| {
                Value::map([
                    ("position", Value::Integer(position)),
                    ("moved_parts_per_million", Value::Integer(moved)),
                    (
                        "held",
                        rank.map_or(Value::Null, |rank| {
                            Value::map([
                                ("first_rank", Value::Integer(rank)),
                                ("kept", Value::Integer(2)),
                                ("of", Value::Integer(2)),
                            ])
                        }),
                    ),
                ])
            };
            fields.insert(
                "floors".to_owned(),
                Value::List(vec![
                    at(0, 300_000, Some(1)),
                    at(1, 0, None),
                    at(2, 1_000_000, Some(4)),
                    at(3, 0, Some(1)),
                ]),
            );
            fields.insert(
                "floor_spread".to_owned(),
                Value::map([
                    ("least_parts_per_million", Value::Integer(0)),
                    ("middle_parts_per_million", Value::Integer(300_000)),
                    ("most_parts_per_million", Value::Integer(1_000_000)),
                ]),
            );
        }
        let text = rendered(&everywhere, "m").join("\n");
        assert!(
            text.contains("floor        0.0% · drawn at 4 positions · 0.0%–100.0% · middle 30.0%"),
            "{text}"
        );
        assert!(text.contains("control 4"), "{text}");
        assert!(
            text.contains("before #1   30.0%  ###·······    1  2/2"),
            "{text}"
        );
        assert!(
            text.contains("before #2    0.0%  ··········    —  —"),
            "{text}"
        );
        assert!(
            text.contains("after #3     0.0%  ··········    1  2/2"),
            "{text}"
        );
        assert!(
            text.contains("at/under widest  3"),
            "every part is under a floor that reached a hundred: {text}"
        );
        assert!(!text.contains("--floors"), "{text}");
    }

    /// The unit, who decided it, and how the prompt reached the model are
    /// the conditions of the run, and they are on the page; the cap is said
    /// with the flag that raises it (§3.4, §3.15, B-430).
    #[test]
    fn the_report_says_what_it_took_apart_and_how_it_was_addressed() {
        let text = rendered(&body(), "m").join("\n");
        assert!(
            text.contains("unit         sentence · text: no blank line"),
            "{text}"
        );
        assert!(
            text.contains("addressed    one user turn, the whole prompt"),
            "{text}"
        );
        assert!(text.contains("prompt       10 tokens"), "{text}");
        assert!(
            text.contains("IMPACT   answer moved without each sentence"),
            "{text}"
        );
        assert!(
            text.contains("not removed   3 · --most 6"),
            "the cap is said with what raises it: {text}"
        );
        let mut alone = body();
        if let Value::Map(fields) = &mut alone {
            fields.insert("unit".to_owned(), Value::text("paragraph".to_owned()));
        }
        let text = rendered(&alone, "m").join("\n");
        assert!(
            text.contains("IMPACT   answer moved without each paragraph"),
            "{text}"
        );
    }

    /// A floor past a half is said as the finding, before the rows (§3.15).
    #[test]
    fn a_floor_past_a_half_says_the_run_did_not_separate_the_parts() {
        let text = rendered(&body(), "m").join("\n");
        assert!(text.contains("separable    yes"), "{text}");
        let mut swamped = body();
        if let Value::Map(fields) = &mut swamped {
            fields.insert(
                "floor_parts_per_million".to_owned(),
                Value::Integer(943_000),
            );
        }
        let text = rendered(&swamped, "m").join("\n");
        assert!(
            text.contains("separable    NO · floor 94.3% ≥ 50.0%"),
            "{text}"
        );
    }

    /// The report says where it survives the terminal, and says plainly
    /// when it does not (A1, A2, B-432).
    #[test]
    fn the_report_says_where_it_was_recorded_or_that_it_was_not() {
        let text = rendered(&body(), "m").join("\n");
        assert!(
            text.contains("record       01J0000000000000000000000A") && text.contains("no text"),
            "{text}"
        );
        let mut unwritten = body();
        if let Value::Map(fields) = &mut unwritten {
            fields.insert("recorded".to_owned(), Value::Null);
        }
        let text = rendered(&unwritten, "m").join("\n");
        assert!(text.contains("record       NOT RECORDED"), "{text}");
    }

    /// Settledness is said under the temperature it was asked at, and where
    /// none was stated it is said as not asked — never as settled (A7, B-431).
    #[test]
    fn settledness_is_said_under_its_temperature_or_as_not_asked() {
        let text = rendered(&body(), "m").join("\n");
        assert!(text.contains("--temperature <t>  3 generations"), "{text}");
        assert!(!text.contains("distinct answers"), "{text}");
        assert!(
            text.contains("generations  6 · as written 1 · removed 3 · control 1\n"),
            "{text}"
        );

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
                    ("top_k", Value::text("20")),
                    ("top_p", Value::text("0.950")),
                    ("min_p", Value::text("off")),
                    ("truncation", Value::text("declared by the file")),
                ]),
            );
        }
        let text = rendered(&warm, "m").join("\n");
        assert!(
            text.contains(
                "SEEDS    3 seeds at temperature 0.700 · distinct answers 3 · farthest apart \
                 41.2% · farthest from greedy 25.0% · cut top_k 20 · top_p 0.950 · min_p off · \
                 declared by the file"
            ),
            "{text}"
        );
        assert!(text.contains("control 1 · seeds 3"), "{text}");
        // A report from before the cut was stated does not say *off* (A7).
        if let Value::Map(fields) = &mut warm
            && let Some(Value::Map(settled)) = fields.get_mut("settled")
        {
            settled.remove("top_k");
        }
        let text = rendered(&warm, "m").join("\n");
        assert!(text.contains("· cut not recorded"), "{text}");
    }

    /// Each part alone is a table read against the control alone, with the
    /// answer each drew; where it was not asked the report says so with the
    /// flag and the cost, never a figure (A7, B-435).
    #[test]
    fn each_part_alone_is_said_against_the_control_alone_or_as_not_asked() {
        let text = rendered(&body(), "m").join("\n");
        assert!(text.contains("--alone            4 generations"), "{text}");
        assert!(!text.contains("control sentence alone"), "{text}");

        let reading = |moved: i64, rank: i64, answer: &str| {
            Value::map([
                ("moved_parts_per_million", Value::Integer(moved)),
                (
                    "held",
                    Value::map([
                        ("first_rank", Value::Integer(rank)),
                        ("kept", Value::Integer(i64::from(rank == 1))),
                        ("of", Value::Integer(1)),
                    ]),
                ),
                ("answer", Value::text(answer.to_owned())),
            ])
        };
        let mut asked = body();
        if let Value::Map(fields) = &mut asked {
            fields.insert(
                "alone".to_owned(),
                Value::List(vec![
                    reading(1_000_000, 40, "What would you like me to answer?"),
                    reading(250_000, 1, "Blue, I think.\nOr green."),
                    reading(1_000_000, 40, "Thank you."),
                ]),
            );
            fields.insert(
                "alone_floor".to_owned(),
                reading(1_000_000, 40, "Hello! How can I help?"),
            );
        }
        let text = rendered(&asked, "m").join("\n");
        assert!(text.contains("control 1 · alone 4"), "{text}");
        assert!(
            text.contains("2   25.0%  ##········    1   1/1  What colour is the room?"),
            "{text}"
        );
        assert!(
            text.contains("→ \"Blue, I think.…\""),
            "the answer to the part alone is under its row, marked as cut: {text}"
        );
        assert!(
            text.contains("ctl  100.0%  ##########   40   0/1  control sentence alone"),
            "{text}"
        );
        assert!(text.contains("→ \"Hello! How can I help?\""), "{text}");
        assert!(text.contains("as far as control or further  2"), "{text}");
    }

    /// The prompt grown from the front is a row a prefix with the first
    /// that came within the floor; where it was not asked the report says
    /// so with the flag and the cost (A7, B-436).
    #[test]
    fn the_prompt_grown_from_the_front_says_where_the_answer_arrived_or_that_it_was_not_asked() {
        let text = rendered(&body(), "m").join("\n");
        assert!(text.contains("--prefixes         3 generations"), "{text}");
        let reading = |moved: i64, answer: &str| {
            Value::map([
                ("moved_parts_per_million", Value::Integer(moved)),
                ("held", Value::Null),
                ("answer", Value::text(answer.to_owned())),
            ])
        };
        let mut asked = body();
        if let Value::Map(fields) = &mut asked {
            fields.insert(
                "prefixes".to_owned(),
                Value::List(vec![
                    reading(1_000_000, "One word? Which?"),
                    reading(0, "Blue."),
                ]),
            );
        }
        let text = rendered(&asked, "m").join("\n");
        assert!(text.contains("control 1 · prefixes 2"), "{text}");
        assert!(
            text.contains("1–1  100.0%  ##########    —     —  Answer in one word."),
            "{text}"
        );
        assert!(
            text.contains("1–2    0.0%  ··········    —     —  What colour is the room?"),
            "{text}"
        );
        assert!(text.contains("→ \"One word? Which?\""), "{text}");
        assert!(text.contains("within floor 0.0%  first at 1–2"), "{text}");

        if let Value::Map(fields) = &mut asked {
            fields.insert(
                "prefixes".to_owned(),
                Value::List(vec![reading(1_000_000, "One word? Which?")]),
            );
        }
        let text = rendered(&asked, "m").join("\n");
        assert!(
            text.contains("within floor 0.0%  none short of the whole"),
            "{text}"
        );
    }

    /// Neighbours swapped is a row a pair with how many moved the answer
    /// past the floor; where it was not asked the report says so with the
    /// flag and the cost (A7, B-437).
    #[test]
    fn neighbours_swapped_says_how_many_pairs_the_order_carries_or_that_it_was_not_asked() {
        let text = rendered(&body(), "m").join("\n");
        assert!(text.contains("--swaps            3 generations"), "{text}");
        let reading = |moved: i64, answer: &str| {
            Value::map([
                ("moved_parts_per_million", Value::Integer(moved)),
                ("held", Value::Null),
                ("answer", Value::text(answer.to_owned())),
            ])
        };
        let mut asked = body();
        if let Value::Map(fields) = &mut asked {
            fields.insert(
                "swaps".to_owned(),
                Value::List(vec![reading(0, "Blue."), reading(200_000, "It's blue.")]),
            );
        }
        let text = rendered(&asked, "m").join("\n");
        assert!(text.contains("control 1 · swaps 2"), "{text}");
        assert!(
            text.contains("1&2   0.0%  ··········    —     —  Answer in one word."),
            "{text}"
        );
        assert!(
            text.contains("2&3  20.0%  ##········    —     —  What colour is the room?"),
            "{text}"
        );
        assert!(text.contains("→ \"It's blue.\""), "{text}");
        assert!(
            text.contains("past floor 0.0%  1 of 2 · order read, not words"),
            "{text}"
        );
    }

    /// The forms are a row a form read, with how many moved the answer
    /// past the floor; a form not rendered is a line saying why; where
    /// the forms were not asked the report says so with the flag and a
    /// cost that is a ceiling (A7, B-444).
    #[test]
    fn the_forms_say_which_dress_the_model_reads_or_that_they_were_not_asked() {
        let text = rendered(&body(), "m").join("\n");
        assert!(
            text.contains("--forms            6 generations  the same parts"),
            "{text}"
        );
        assert!(text.contains("at most: a form the prompt"), "{text}");
        let read = |form: &str, moved: i64, answer: &str| {
            Value::map([
                ("form", Value::text(form.to_owned())),
                ("moved_parts_per_million", Value::Integer(moved)),
                ("held", Value::Null),
                ("answer", Value::text(answer.to_owned())),
            ])
        };
        let mut asked = body();
        if let Value::Map(fields) = &mut asked {
            fields.insert(
                "forms".to_owned(),
                Value::List(vec![
                    Value::map([
                        ("form", Value::text("one line")),
                        (
                            "not_rendered",
                            Value::text("the prompt is written this way"),
                        ),
                    ]),
                    read("bullets", 0, "Blue."),
                    read("capitals", 350_000, "BLUE."),
                ]),
            );
        }
        let text = rendered(&asked, "m").join("\n");
        assert!(text.contains("control 1 · forms 2"), "{text}");
        assert!(text.contains("FORMS"), "{text}");
        assert!(
            text.contains("bullets    0.0%  ··········    —  —"),
            "{text}"
        );
        assert!(
            text.contains("capitals  35.0%  ###·······    —  —"),
            "{text}"
        );
        assert!(text.contains("→ \"BLUE.\""), "{text}");
        assert!(
            text.contains("one line   not rendered · the prompt is written this way"),
            "{text}"
        );
        assert!(
            text.contains("past floor 0.0%  1 of 2 · form read, not words"),
            "{text}"
        );
    }

    /// A table pads every column to its widest cell, figures against the
    /// right edge and text against the left, and never pads the last.
    #[test]
    fn a_table_lines_its_columns_up() {
        let lines = table(
            &[figure("#"), text("name"), figure("n"), text("note")],
            &[
                vec!["1".into(), "ab".into(), "100".into(), "x".into()],
                vec!["→ under".into()],
                vec!["12".into(), "a".into(), "7".into(), "yy".into()],
            ],
        );
        assert_eq!(
            lines,
            vec![
                "   #  name    n  note",
                "   1  ab    100  x",
                "       → under",
                "  12  a       7  yy",
            ]
        );
        assert_eq!(signed_percent(-13_000), "-1.3");
        assert_eq!(signed_percent(20_000), "+2.0");
        assert_eq!(signed_percent(0), "0.0");
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

    /// Devstral, bare, ended a build instruction at one token that
    /// printed as nothing, and the page showed a heading over a blank
    /// (F160). The answer's length and what ended it are on the heading,
    /// and an empty answer is said to be one.
    #[test]
    fn an_empty_answer_is_said_with_its_length_and_what_ended_it() {
        let mut body = self::body();
        if let Value::Map(fields) = &mut body {
            let _was = fields.insert("baseline".to_owned(), Value::text(String::new()));
            let _was = fields.insert("answer_tokens".to_owned(), Value::Integer(1));
            let _was = fields.insert(
                "answer_stopped".to_owned(),
                Value::text("stop_token".to_owned()),
            );
        }
        let page = rendered(&body, "m.gguf").join("\n");
        assert!(
            page.contains("ANSWER   as written · 1 token · ended at its stop token"),
            "{page}"
        );
        assert!(page.contains("\n  nothing written\n"), "{page}");

        let mut body = self::body();
        if let Value::Map(fields) = &mut body {
            let _was = fields.insert("answer_tokens".to_owned(), Value::Integer(600));
            let _was = fields.insert("answer_stopped".to_owned(), Value::text("limit".to_owned()));
        }
        let page = rendered(&body, "m.gguf").join("\n");
        assert!(
            page.contains("ANSWER   as written · 600 tokens · ran to the cap\n  Blue."),
            "{page}"
        );
        assert!(!page.contains("nothing written"), "{page}");
    }
}
