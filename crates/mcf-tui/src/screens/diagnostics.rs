//! Setting up a measurement: two buttons with what they cost, and the options.
//!
//! **The window implies the ladder.** Choosing a context window means every
//! power of two up to it is sampled — there is no useful run that measures
//! 8 192 and skips 2 048, because the shallow points are what the deep one is
//! read against. So the depths are stated under the window rather than offered
//! as a second set of choices somebody could contradict the first with.
//!
//! **The estimate is a range**, because MCF's own estimate has been measured
//! against what runs actually take and lands between 0.58× and 1.42× of it. A
//! single number would be a promise it cannot keep.
//!
//! **Four of the rows are one run.** The ladder that measures generation
//! speed against depth is two timed generations a rung, and the time to a
//! first token, the cost of a token of prompt and the memory a token of window
//! costs are all read off those same generations (`mcf_serve::ladder`). There
//! is no run that answers one of the four and not the others, so they are
//! chosen together, and the estimate is the run's — on the row that names the
//! run, and on no other, because an estimate for a test that is never run on
//! its own is a figure for nothing (A7, A20). The fifth row is its own run —
//! the cross-check of MCF's engine against the provisioned one — chosen and
//! unchosen alone, and costed on its own row (B-424).

use mcf_record::json::Value;

use crate::job::Job;
use crate::screen::{Ink, Screen};
use crate::screens::{columns, grouped};

/// What runs a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    /// One climb of the depth ladder, which answers every row marked with it
    /// at once.
    Ladder,
    /// One cross-check: MCF's own engine reading what the provisioned one
    /// produced (`mcf cross-check`).
    CrossCheck,
}

/// One measurement that can be asked for.
#[derive(Debug, Clone)]
pub struct Test {
    /// What it measures, in words.
    pub name: &'static str,
    /// Which devices it needs.
    pub devices: &'static str,
    /// Roughly how long the run takes, in seconds, at this machine's speed —
    /// on the row that names the run. `None` on a row another row's run
    /// answers, which has no time of its own and is not given one (A7).
    pub seconds: Option<u64>,
    /// What runs it.
    pub run: Run,
    /// Whether it is selected.
    pub chosen: bool,
    /// How long the last run of this test actually took, in seconds.
    ///
    /// **`None` until it has run, and `None` is not zero** (A7). A test that
    /// has never run has no run time, and the screen draws a dash rather than
    /// a figure somebody could read as *instant*.
    pub ran: Option<u64>,
    /// What the last run found, in the words the daemon used.
    ///
    /// **Absent until there is a result to show.** Nothing here is written by
    /// the surface: every line comes from the answer MCF sent.
    pub result: Option<Vec<String>>,
}

/// The tests MCF knows how to run, in the order the window and the console
/// both list them.
#[must_use]
pub fn tests() -> Vec<Test> {
    let ladder = |name: &'static str, seconds: Option<u64>| Test {
        name,
        devices: "both",
        seconds,
        run: Run::Ladder,
        chosen: true,
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
            chosen: false,
            ran: None,
            result: None,
        },
        ladder("Prompt reading speed", None),
    ]
}

/// Flips the choice at `at`, by the rule both surfaces share (B-072): the
/// rows one run answers are chosen and unchosen together.
pub fn toggle<'a>(rows: impl IntoIterator<Item = (Run, &'a mut bool)>, at: usize) {
    let rows: Vec<(Run, &'a mut bool)> = rows.into_iter().collect();
    let Some((flipped, chosen)) = rows.get(at) else {
        return;
    };
    let (flipped, now) = (*flipped, !**chosen);
    for (run, chosen) in rows {
        if run == flipped {
            *chosen = now;
        }
    }
}

/// Writes a finished ladder onto every row it answers.
///
/// **The figures are the daemon's** (B-072): each rung as it came, and the
/// three rows the same run measured — a prompt's cost, the time to a first
/// token, the memory a token of window costs — from the sentences the last
/// line carries, never worked out here. A run that was refused before it
/// climbed anything leaves those as they were, because it measured none of
/// them; the row that names the run carries the refusal (A2).
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
            lines.push(format!("at {} tokens   {said}", grouped(depth)));
        }
        if let Some(conditions) = job.conclusion().and_then(|body| body.get("conditions")) {
            // B65 and D31: which engine ran is a condition of every figure
            // above it, so it travels with them rather than being read off a
            // screen that has moved on.
            let engine = conditions
                .get("engine_ran")
                .and_then(Value::as_text)
                .unwrap_or("MCF did not say");
            lines.push(format!("measured on {engine}"));
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

/// Writes a finished cross-check onto the row that asked for it.
///
/// **The sentences are the daemon's** (B-072): the same ones `mcf
/// cross-check` prints, read off the last line rather than composed from its
/// figures here, so the window and the console cannot say one comparison two
/// ways. A refusal is the row's result too — the check ran and could not
/// compare, which is a thing to show, not a blank (A2).
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

/// The deepest rung a Quick Run climbs to: the shallowest and one above it,
/// so that it is a fall-off rather than a single number, and quick. One
/// depth for the window and the console (B-072) — the console once sent
/// half the model's context under a button whose estimate assumed this.
pub const QUICK_DEPTH: u64 = 1024;

/// The estimate for a Quick Run, in seconds: a sixth of the ladder's, since
/// it climbs two rungs of a ladder whose top rungs are most of its time.
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

/// The estimate for what is chosen, in seconds: each chosen run once.
#[must_use]
pub fn chosen_seconds(tests: &[Test]) -> u64 {
    tests
        .iter()
        .filter(|test| test.chosen)
        .filter_map(|test| test.seconds)
        .sum()
}

/// The measured spread of MCF's own estimate against what runs take.
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

/// The buttons, in the order the cursor visits them.
pub const BUTTONS: [&str; 3] = [" Quick Run ", " Run Selected ", " Back "];

/// Where the cursor is on this screen.
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    /// The highlighted row of the table.
    pub row: usize,
    /// The highlighted button.
    pub button: usize,
    /// Whether the cursor is in the buttons rather than the table.
    pub on_buttons: bool,
}

/// Draws the screen. `running` is the run in progress or the one just
/// finished, whose progress or result goes under the table.
pub fn draw(
    into: &mut Screen,
    from: usize,
    model: &str,
    window: Option<u64>,
    tests: &[Test],
    cursor: Cursor,
    running: Option<&Job>,
) {
    let mut row = from + 1;
    let button = |index: usize| {
        if cursor.on_buttons && cursor.button == index {
            Ink::Selected
        } else {
            Ink::Plain
        }
    };
    into.put(3, row, BUTTONS[0], button(0));
    into.put(20, row, BUTTONS[1], button(1));
    into.put_right(into.width().saturating_sub(3), row, BUTTONS[2], button(2));
    row += 1;
    into.put(3, row, &span(quick_seconds(tests)), Ink::Quiet);
    let chosen = chosen_seconds(tests);
    into.put(20, row, &span(chosen), Ink::Quiet);
    row += 2;

    into.put(3, row, "WHAT TO MEASURE", Ink::Heading);
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
    let after = tests_table(into, row + 2, tests, cursor.row, chosen);
    under_the_table(into, after, tests, cursor.row, running);
}

/// What goes under the table: a run's progress while it goes, and what the
/// highlighted row found once it has.
///
/// **Every line is the daemon's** — a rung as it came, the estimate it gave,
/// the sentences it composed — and a refusal is drawn as one (A2). The rows
/// the terminal has left bound it; a result longer than that says how much
/// more there is rather than stopping as if that were all (A7).
fn under_the_table(
    into: &mut Screen,
    from: usize,
    tests: &[Test],
    at: usize,
    running: Option<&Job>,
) {
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
            if let Some(test) = tests.get(at)
                && let (Some(ran), Some(found)) = (test.ran, &test.result)
            {
                lines.push((format!("{} — ran {}", test.name, plain(ran)), Ink::Heading));
                lines.extend(found.iter().map(|line| (line.clone(), Ink::Plain)));
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

/// What a run has said so far, one line each, in the order it said them.
fn progress_of(job: &Job) -> Vec<(String, Ink)> {
    let mut lines = Vec::new();
    if let Some(why) = &job.refused {
        lines.push((why.clone(), Ink::Refusal));
    }
    for answer in &job.answers {
        if let (Some(low), Some(high)) = (
            answer
                .get("estimate_low_seconds")
                .and_then(Value::as_integer),
            answer
                .get("estimate_high_seconds")
                .and_then(Value::as_integer),
        ) {
            lines.push((
                format!("somewhere between {low} and {high} seconds, MCF estimates"),
                Ink::Quiet,
            ));
        }
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
            lines.push((format!("at {} tokens   {said}", grouped(depth)), Ink::Plain));
        }
        if matches!(answer.get("reading"), Some(Value::Bool(true))) {
            lines.push((
                "the provisioned engine has produced; MCF's own engine is reading".to_owned(),
                Ink::Quiet,
            ));
        }
    }
    lines
}

/// The estimate column: the run's time on the row that names the run, and
/// on a row that same run answers, where the time is.
fn estimate_of(test: &Test) -> String {
    match (test.seconds, test.run) {
        (Some(seconds), _) => plain(seconds),
        (None, Run::Ladder) => "in the ladder".to_owned(),
        (None, Run::CrossCheck) => "in the cross-check".to_owned(),
    }
}

/// The tests, and what has been chosen of them. Returns the first free row
/// beneath.
fn tests_table(into: &mut Screen, from: usize, tests: &[Test], at: usize, chosen: u64) -> usize {
    let mut row = from;
    columns(
        into,
        3,
        row,
        &[
            ("TESTS", 42, false, Ink::Heading),
            ("DEVICES", 14, true, Ink::Quiet),
            ("TIME", 14, true, Ink::Quiet),
        ],
    );
    row += 1;
    for (index, test) in tests.iter().enumerate() {
        let here = index == at;
        let mark = if test.chosen { "x" } else { " " };
        into.put(3, row, "[", Ink::Plain);
        into.put(4, row, mark, Ink::Held);
        into.put(5, row, "]", Ink::Plain);
        let ink = if here {
            Ink::Selected
        } else if test.chosen {
            Ink::Plain
        } else {
            Ink::Quiet
        };
        columns(
            into,
            7,
            row,
            &[
                (test.name, 38, false, ink),
                (test.devices, 14, true, Ink::Quiet),
                (&estimate_of(test), 14, true, Ink::Quiet),
            ],
        );
        row += 1;
    }
    row += 1;
    let count = tests.iter().filter(|t| t.chosen).count();
    columns(
        into,
        3,
        row,
        &[
            ("selected", 12, false, Ink::Quiet),
            (
                &format!("{count} of {}", tests.len()),
                10,
                false,
                Ink::Plain,
            ),
            ("estimate", 12, false, Ink::Quiet),
            (&span(chosen), 26, false, Ink::Heading),
        ],
    );
    row + 2
}
