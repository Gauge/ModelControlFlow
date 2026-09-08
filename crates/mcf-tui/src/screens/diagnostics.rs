use mcf_record::json::Value;

use crate::job::Job;
use crate::screen::{Ink, Screen};
use crate::screens::{columns, grouped};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    Ladder,
    CrossCheck,
    Probes,
}

#[derive(Debug, Clone)]
pub struct Test {
    pub name: &'static str,
    pub devices: &'static str,
    pub seconds: Option<u64>,
    pub run: Run,
    pub ran: Option<u64>,
    pub result: Option<Vec<String>>,
}

#[must_use]
pub fn tests() -> Vec<Test> {
    let ladder = |name: &'static str, seconds: Option<u64>| Test {
        name,
        devices: "both",
        seconds,
        run: Run::Ladder,
        ran: None,
        result: None,
    };
    vec![
        ladder("Generation speed against depth", Some(180)),
        ladder("Start-up to first token", None),
        ladder("Memory ceiling — largest context", None),
        ladder("Fall-off with depth", None),
        Test {
            name: "MCF's engine and the provisioned one agree",
            devices: "both engines",
            seconds: Some(90),
            run: Run::CrossCheck,
            ran: None,
            result: None,
        },
        ladder("Prompt reading speed", None),
        Test {
            name: "Capabilities — the probes",
            devices: "an engine",
            seconds: Some(240),
            run: Run::Probes,
            ran: None,
            result: None,
        },
    ]
}

pub fn keep_the_probes(tests: &mut [Test], job: &Job) {
    let mut lines: Vec<String> = Vec::new();
    if let Some(why) = &job.refused {
        lines.push(why.clone());
    } else {
        for answer in &job.answers {
            lines.extend(
                answer
                    .get("lines")
                    .and_then(Value::as_list)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(Value::as_text)
                    .map(str::to_owned),
            );
        }
    }
    for test in tests.iter_mut().filter(|test| test.run == Run::Probes) {
        test.ran = Some(job.ran());
        test.result = Some(lines.clone());
    }
}

#[must_use]
pub fn seconds_of(tests: &[Test], run: Run) -> u64 {
    tests
        .iter()
        .find(|test| test.run == run)
        .and_then(|test| test.seconds)
        .unwrap_or(30)
}

pub fn keep_the_ladder(tests: &mut [Test], job: &Job) {
    let ran = job.ran();
    let mut lines: Vec<String> = Vec::new();
    if let Some(why) = &job.refused {
        lines.push(why.clone());
    } else {
        for answer in &job.answers {
            let Some(reading) = answer.get("reading") else {
                continue;
            };
            let depth = reading
                .get("depth")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
                .unwrap_or(0);
            let said = if matches!(reading.get("measured"), Some(Value::Bool(true))) {
                reading
                    .get("ms_per_token")
                    .and_then(Value::as_text)
                    .map_or_else(|| "Unknown".to_owned(), |ms| format!("{ms} ms a token"))
            } else {
                "Unknown".to_owned()
            };
            lines.push(format!(
                "at {} tokens   {said}{}",
                grouped(depth),
                pairs_note(reading)
            ));
        }
        if let Some(conditions) = job.conclusion().and_then(|body| body.get("conditions")) {
            let engine = conditions
                .get("engine_ran")
                .and_then(Value::as_text)
                .unwrap_or("MCF did not say");
            lines.push(format!("measured on {engine}{}", on_device(conditions)));
        }
    }
    let derived = job.conclusion().map(|body| {
        [
            (
                "Prompt reading speed",
                mcf_serve::ladder::prompt_reading_said(body.get("prompt_reading")),
            ),
            (
                "Start-up to first token",
                mcf_serve::ladder::first_token_said(body.get("first_token")),
            ),
            (
                "Memory ceiling — largest context",
                mcf_serve::ladder::memory_said(body.get("memory")),
            ),
            (
                "Fall-off with depth",
                mcf_serve::ladder::fall_off_said(body.get("fall_off")),
            ),
        ]
    });
    for test in tests {
        if test.name == "Generation speed against depth" {
            test.ran = Some(ran);
            test.result = Some(lines.clone());
        } else if let Some((_, said)) = derived
            .as_ref()
            .and_then(|rows| rows.iter().find(|(name, _)| *name == test.name))
        {
            test.ran = Some(ran);
            test.result = Some(said.clone());
        }
    }
}

pub fn keep_the_cross_check(tests: &mut [Test], job: &Job) {
    let ran = job.ran();
    let lines: Vec<String> = if let Some(why) = &job.refused {
        vec![why.clone()]
    } else {
        let last = job.conclusion();
        let mut said: Vec<String> = last
            .and_then(|body| body.get("said"))
            .and_then(Value::as_list)
            .map(|sentences| {
                sentences
                    .iter()
                    .filter_map(Value::as_text)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(engine) = last
            .and_then(|body| body.get("conditions"))
            .and_then(|conditions| conditions.get("engine_ran"))
            .and_then(Value::as_text)
        {
            said.push(format!("against {engine}"));
        }
        said
    };
    for test in tests {
        if test.run == Run::CrossCheck {
            test.ran = Some(ran);
            test.result = Some(lines.clone());
        }
    }
}

pub const QUICK_DEPTH: u64 = 1024;

#[must_use]
pub fn quick_seconds(tests: &[Test]) -> u64 {
    #[expect(
        clippy::integer_division,
        reason = "a sixth of a test, in whole seconds"
    )]
    let sixth = tests
        .iter()
        .find(|test| test.run == Run::Ladder)
        .and_then(|test| test.seconds)
        .map_or(30, |seconds| seconds / 6);
    sixth
}

const SLOWEST: u64 = 142;
const QUICKEST: u64 = 58;

#[allow(
    clippy::integer_division,
    reason = "a bound on a duration; a fraction of a second means nothing here"
)]
fn span(seconds: u64) -> String {
    let low = seconds.saturating_mul(100) / SLOWEST;
    let high = seconds.saturating_mul(100) / QUICKEST;
    format!("{} – {}", plain(low), plain(high))
}

#[allow(clippy::integer_division, reason = "seconds into minutes and seconds")]
fn plain(seconds: u64) -> String {
    if seconds < 90 {
        format!("{seconds} s")
    } else {
        format!("{} min {:02} s", seconds / 60, seconds % 60)
    }
}

pub const BUTTONS: [&str; 5] = [
    " Quick run ",
    " Run ",
    " Cross-check ",
    " Capabilities ",
    " Back ",
];

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub row: usize,
    pub button: usize,
    pub on_buttons: bool,
}

pub fn draw(
    into: &mut Screen,
    from: usize,
    setup: Setup<'_>,
    tests: &[Test],
    cursor: Cursor,
    running: Option<&Job>,
) {
    let (model, window) = (setup.model, setup.window);
    let mut row = from + 1;
    let button = |index: usize| {
        if cursor.on_buttons && cursor.button == index {
            Ink::Selected
        } else {
            Ink::Plain
        }
    };
    let mut x = 3;
    let mut costs = Vec::new();
    for (index, label) in BUTTONS.iter().enumerate().take(4) {
        into.put(x, row, label, button(index));
        costs.push((x, index));
        x += label.chars().count() + 2;
    }
    into.put_right(into.width().saturating_sub(3), row, BUTTONS[4], button(4));
    row += 1;
    for (x, index) in costs {
        let seconds = match index {
            0 => quick_seconds(tests),
            1 => seconds_of(tests, Run::Ladder),
            2 => seconds_of(tests, Run::CrossCheck),
            _ => seconds_of(tests, Run::Probes),
        };
        into.put(x, row, &span(seconds), Ink::Quiet);
    }
    row += 2;

    into.put(3, row, "SETUP", Ink::Heading);
    row += 1;
    let field = |into: &mut Screen, row: usize, label: &str, value: &str| {
        into.put(3, row, label, Ink::Quiet);
        into.put(21, row, "( ", Ink::Quiet);
        into.put(23, row, value, Ink::Plain);
        into.put(59, row, "▾ )", Ink::Quiet);
    };
    let short: String = model.chars().take(34).collect();
    field(into, row, "model", &short);
    row += 1;
    let window_text = window.map_or_else(
        || "largest this machine affords".to_owned(),
        |w| format!("{w} tokens"),
    );
    field(into, row, "context window", &window_text);
    row += 1;
    let unresolved = || "not resolved".to_owned();
    into.put(3, row, "engine", Ink::Quiet);
    into.put(
        21,
        row,
        &setup.engine.map_or_else(unresolved, str::to_owned),
        Ink::Plain,
    );
    row += 1;
    into.put(3, row, "runs on", Ink::Quiet);
    let on: String = runs_on(&setup)
        .chars()
        .take(into.width().saturating_sub(24))
        .collect();
    into.put(21, row, &on, Ink::Plain);
    into.put_right(
        into.width().saturating_sub(3),
        row,
        "d changes · x stops a run",
        Ink::Quiet,
    );
    row += 1;
    if let Some(component) = setup.card_unused {
        let said =
            format!("a card is here that no engine drives; `mcf provision {component}` builds one");
        let shown: String = said.chars().take(into.width().saturating_sub(6)).collect();
        into.put(3, row, &shown, Ink::Held);
        row += 1;
    }
    if let Some(window) = window {
        let mut depths = Vec::new();
        let mut depth = 512_u64;
        #[allow(clippy::integer_division, reason = "half a window, exactly")]
        let deepest = window / 2;
        while depth <= deepest {
            depths.push(depth.to_string());
            depth *= 2;
        }
        into.put(3, row, "samples", Ink::Quiet);
        let joined = depths.join(" · ");
        let shown: String = joined.chars().take(45).collect();
        into.put(21, row, &shown, Ink::Plain);
        into.put_right(
            into.width().saturating_sub(3),
            row,
            "every step",
            Ink::Quiet,
        );
    }
    let after = runs_table(into, row + 2, tests);
    under_the_table(into, after, tests, running);
}

#[must_use]
pub fn pairs_note(reading: &Value) -> String {
    let Some(pairs) = reading.get("pairs") else {
        return String::new();
    };
    let figure = |key: &str| pairs.get(key).and_then(Value::as_integer).unwrap_or(0);
    let (of, separated) = (figure("of"), figure("separated"));
    if separated >= of {
        return String::new();
    }
    let mut why = Vec::new();
    for (key, said) in [
        ("did_not_separate", "did not separate"),
        ("missed_the_pin", "missed the pin"),
        ("refused", "were refused"),
    ] {
        let count = figure(key);
        if count > 0 {
            why.push(format!("{count} {said}"));
        }
    }
    format!(
        " — over {separated} of {of} pairs{}{}",
        if why.is_empty() { "" } else { ": " },
        why.join(", ")
    )
}

#[must_use]
pub fn on_device(conditions: &Value) -> String {
    let Some(device) = conditions.get("device").and_then(Value::as_text) else {
        return String::new();
    };
    match conditions.get("gpu_layers").and_then(Value::as_integer) {
        Some(layers) if layers > 0 => format!(", {device} with {layers} layers on the card"),
        _ => format!(", {device}"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Setup<'a> {
    pub model: &'a str,
    pub window: Option<u64>,
    pub engine: Option<&'a str>,
    pub device: Option<&'a str>,
    pub on: Option<mcf_serve::control::On>,
    pub card_unused: Option<&'a str>,
}

#[must_use]
pub fn runs_on(setup: &Setup<'_>) -> String {
    let resolved = setup.device.unwrap_or("not resolved");
    match setup.on {
        None => resolved.to_owned(),
        Some(mcf_serve::control::On::Processor) => {
            format!("the processor, nothing on a card — chosen; MCF resolves it to {resolved}")
        }
        Some(mcf_serve::control::On::Card) => {
            format!("the card, the whole model on it — chosen; MCF resolves it to {resolved}")
        }
    }
}

fn under_the_table(into: &mut Screen, from: usize, tests: &[Test], running: Option<&Job>) {
    let last = into.height().saturating_sub(2);
    let mut row = from;
    if row > last {
        return;
    }
    let mut lines: Vec<(String, Ink)> = Vec::new();
    match running {
        Some(job) if !job.finished => {
            lines.push((format!("{}, {} s so far", job.what, job.ran()), Ink::Held));
            lines.extend(progress_of(job));
        }
        _ => {
            for test in tests {
                let (Some(ran), Some(found)) = (test.ran, &test.result) else {
                    continue;
                };
                lines.push((format!("{} — ran {}", test.name, plain(ran)), Ink::Heading));
                lines.extend(found.iter().map(|line| (line.clone(), Ink::Plain)));
                lines.push((String::new(), Ink::Plain));
            }
        }
    }
    let room = last.saturating_sub(row) + 1;
    let shown = if lines.len() > room {
        room.saturating_sub(1)
    } else {
        lines.len()
    };
    for (line, ink) in lines.iter().take(shown) {
        let fitted: String = line.chars().take(into.width().saturating_sub(6)).collect();
        into.put(3, row, &fitted, *ink);
        row += 1;
    }
    if lines.len() > shown {
        into.put(
            3,
            row,
            &format!(
                "… {} more line(s) than this terminal has rows",
                lines.len() - shown
            ),
            Ink::Quiet,
        );
    }
}

pub fn progress_of(job: &Job) -> Vec<(String, Ink)> {
    let mut lines = Vec::new();
    if let Some(why) = &job.refused {
        lines.push((why.clone(), Ink::Refusal));
    }
    if let Some(estimate) = estimated_seconds(job) {
        lines.push((estimate, Ink::Quiet));
    }
    for answer in &job.answers {
        if let Some(reading) = answer.get("reading") {
            let depth = reading
                .get("depth")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
                .unwrap_or(0);
            let said = reading
                .get("ms_per_token")
                .and_then(Value::as_text)
                .map_or_else(|| "Unknown".to_owned(), |ms| format!("{ms} ms a token"));
            lines.push((
                format!(
                    "at {} tokens   {said}{}",
                    grouped(depth),
                    pairs_note(reading)
                ),
                Ink::Plain,
            ));
        }
        if matches!(answer.get("reading"), Some(Value::Bool(true))) {
            lines.push((
                "the provisioned engine has produced; MCF's own engine is reading".to_owned(),
                Ink::Quiet,
            ));
        }
    }
    if !job.finished
        && let Some(step) = step_of(job)
    {
        lines.push((step, Ink::Held));
    }
    lines
}

#[must_use]
pub fn estimated_seconds(job: &Job) -> Option<String> {
    job.answers.iter().find_map(|answer| {
        let low = answer
            .get("estimate_low_seconds")
            .and_then(Value::as_integer)?;
        let high = answer
            .get("estimate_high_seconds")
            .and_then(Value::as_integer)?;
        Some(format!(
            "somewhere between {low} and {high} seconds, MCF estimates"
        ))
    })
}

#[must_use]
pub fn step_of(job: &Job) -> Option<String> {
    job.answers.iter().rev().find_map(step_line)
}

#[must_use]
pub fn step_line(answer: &Value) -> Option<String> {
    let figure = |held: &Value, key: &str| {
        held.get(key)
            .and_then(Value::as_integer)
            .and_then(|found| u64::try_from(found).ok())
    };
    if let Some(running) = answer.get("running") {
        return Some(format!(
            "at {} tokens, repeat {} of {}: loading the model and asking for {} token(s)",
            grouped(figure(running, "depth")?),
            figure(running, "repeat")?,
            figure(running, "of_repeats")?,
            figure(running, "produce")?,
        ));
    }
    let starting = answer.get("starting")?;
    Some(format!(
        "step {} of {}: measuring at {} tokens",
        figure(starting, "step")?,
        figure(starting, "of")?,
        grouped(figure(starting, "depth")?)
    ))
}

fn runs_table(into: &mut Screen, from: usize, tests: &[Test]) -> usize {
    let mut row = from;
    columns(
        into,
        3,
        row,
        &[
            ("RUN", 16, false, Ink::Heading),
            ("ANSWERS", 44, false, Ink::Quiet),
            ("TIME", 10, true, Ink::Quiet),
            ("LAST RAN", 10, true, Ink::Quiet),
        ],
    );
    row += 1;
    let ran_of = |run: Run| {
        tests
            .iter()
            .find(|test| test.run == run)
            .and_then(|test| test.ran)
            .map_or_else(|| "—".to_owned(), plain)
    };
    let listed: [(&str, &str, String, String); 5] = [
        (
            "Throughput",
            "speed at each depth, prefill, first token, KV cache",
            plain(seconds_of(tests, Run::Ladder)),
            ran_of(Run::Ladder),
        ),
        (
            "Cross-check",
            "whether MCF's engine agrees with the provisioned one",
            plain(seconds_of(tests, Run::CrossCheck)),
            ran_of(Run::CrossCheck),
        ),
        (
            "Capabilities",
            "template, stop conditions, thinking, tools, context",
            plain(seconds_of(tests, Run::Probes)),
            ran_of(Run::Probes),
        ),
        (
            "Prompt analysis",
            "what each part of a prompt does — `mcf prompt`",
            "—".to_owned(),
            "—".to_owned(),
        ),
        (
            "Comparison",
            "two models on one question — `mcf bench`",
            "—".to_owned(),
            "—".to_owned(),
        ),
    ];
    for (name, answers, cost, ran) in &listed {
        columns(
            into,
            3,
            row,
            &[
                (name, 16, false, Ink::Plain),
                (answers, 44, false, Ink::Quiet),
                (cost, 10, true, Ink::Quiet),
                (ran, 10, true, Ink::Quiet),
            ],
        );
        row += 1;
    }
    row + 1
}
