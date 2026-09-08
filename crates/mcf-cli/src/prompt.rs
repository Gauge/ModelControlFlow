use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

const SEED: u64 = 41;

#[derive(Debug, Clone, Default)]
pub(crate) struct Asked<'a> {
    pub prompt: Option<&'a str>,
    pub file: Option<&'a str>,
    pub by: Option<mcf_serve::prompt::Unit>,
    pub most: Option<usize>,
    pub temperature: Option<mcf_core::configuration::Thousandths>,
    pub extras: mcf_serve::prompt::Extras,
    pub turn: mcf_serve::turn::Turn,
}

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

fn unit_of(body: &Value) -> &str {
    body.get("unit")
        .and_then(Value::as_text)
        .unwrap_or("sentence")
}

#[allow(
    clippy::integer_division,
    reason = "a bar has ten cells; a fraction of a cell is not drawn"
)]
const fn tenths_of_the_answer(parts_per_million: i64) -> i64 {
    parts_per_million / 100_000
}

#[allow(
    clippy::integer_division,
    reason = "a tenth of a per cent is the resolution shown; the rest is not"
)]
const fn as_percent(parts_per_million: i64) -> (i64, i64) {
    (parts_per_million / 10_000, (parts_per_million / 1_000) % 10)
}

pub(crate) fn percent(parts_per_million: i64) -> String {
    let (whole, tenth) = as_percent(parts_per_million);
    format!("{whole}.{tenth}%")
}

fn signed_percent(difference: i64) -> String {
    let (whole, tenth) = as_percent(difference.abs());
    match difference.signum() {
        1 => format!("+{whole}.{tenth}"),
        -1 => format!("-{whole}.{tenth}"),
        _ => format!("{whole}.{tenth}"),
    }
}

fn bar(parts_per_million: i64) -> String {
    let filled = usize::try_from(tenths_of_the_answer(parts_per_million))
        .unwrap_or(0)
        .min(10);
    "#".repeat(filled) + &"·".repeat(10_usize.saturating_sub(filled))
}

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

fn head(label: &str, conditions: &[String]) -> String {
    if conditions.is_empty() {
        label.to_owned()
    } else {
        format!("{label:<9}{}", conditions.join(" · "))
    }
}

fn rank_cell(held: Option<&Value>, depth: i64) -> String {
    mcf_desk::held_mark(held, depth)
}

fn open_cell(held: Option<&Value>) -> String {
    mcf_desk::open_mark(held)
}

fn first_line_of(said: &str) -> String {
    let line = said.trim().lines().next().unwrap_or_default();
    let mut shown: String = line.chars().take(72).collect();
    if shown.chars().count() < line.chars().count() || said.trim().lines().count() > 1 {
        shown.push('…');
    }
    format!("{shown:?}")
}

fn cut(said: &str, width: usize) -> String {
    let line = said.trim().lines().next().unwrap_or_default();
    let mut shown: String = line.chars().take(width).collect();
    if shown.chars().count() < line.chars().count() || said.trim().lines().count() > 1 {
        shown.push('…');
    }
    shown
}

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
    if let Some(asked) = text("asked_as") {
        rows.push(("asked", asked.to_owned()));
    }
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

fn the_floor(body: &Value, unit: &str) -> String {
    let floor = percent(integer(body, "floor_parts_per_million"));
    let depth = integer(body, "forced_depth");
    let held = match body.get("floor_held") {
        Some(held) if !matches!(held, Value::Null) => format!(
            " · control in: 1st {} · open {}",
            rank_cell(Some(held), depth),
            open_cell(Some(held))
        ),
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

fn thought_cell(clause: &Value) -> String {
    thought_cell_of(thought_of(clause, "thought"))
}

fn thought_cell_of(thought: Option<i64>) -> String {
    thought.map_or_else(|| "—".to_owned(), |thought| thought.to_string())
}

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

fn under_the_floor(body: &Value, floor: i64) -> Vec<String> {
    let clauses = clauses_of(body);
    let quiet: Vec<String> = clauses
        .iter()
        .enumerate()
        .filter(|(_, clause)| moved_of(clause) <= floor)
        .map(|(at, _)| format!("#{}", at.saturating_add(1)))
        .collect();
    let mut lines = Vec::new();
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

fn form_name(formed: &Value) -> String {
    formed
        .get("form")
        .and_then(Value::as_text)
        .unwrap_or("")
        .to_owned()
}

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
        if integer(word, "pieces") == 0 {
            unread = unread.saturating_add(1);
            continue;
        }
        let read = integer(word, "pieces").saturating_sub(integer(word, "unread"));
        if read == 0 {
            no_context = no_context.saturating_add(1);
            continue;
        }
        pieces = pieces.saturating_add(read);
        own = own.saturating_add(integer(word, "first_choice"));
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

const MOST_WORDS: usize = 20;

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
    if let Some(thought) = thought_of(body, "baseline_thought") {
        conditions.push(format!("{} before it", count_of(thought, "token")));
    }
    let mut lines = vec![head("ANSWER", &conditions)];
    let written: Vec<&str> = baseline.lines().collect();
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

fn thought_of(held: &Value, key: &str) -> Option<i64> {
    held.get(key).and_then(Value::as_integer)
}

fn count_of(how_many: i64, noun: &str) -> String {
    if how_many == 1 {
        format!("1 {noun}")
    } else {
        format!("{how_many} {noun}s")
    }
}

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
        if let Value::Map(fields) = &mut warm
            && let Some(Value::Map(settled)) = fields.get_mut("settled")
        {
            settled.remove("top_k");
        }
        let text = rendered(&warm, "m").join("\n");
        assert!(text.contains("· cut not recorded"), "{text}");
    }

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
