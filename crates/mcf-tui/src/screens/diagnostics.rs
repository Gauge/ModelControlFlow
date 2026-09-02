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

use crate::screen::{Ink, Screen};
use crate::screens::columns;

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
    };
    vec![
        ladder("Generation speed against depth", Some(180)),
        ladder("Start-up to first token", None),
        ladder("Memory ceiling — largest context", None),
        Test {
            name: "MCF's engine and the provisioned one agree",
            devices: "both engines",
            seconds: Some(90),
            run: Run::CrossCheck,
            chosen: false,
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

/// Draws the screen.
pub fn draw(
    into: &mut Screen,
    from: usize,
    model: &str,
    window: Option<u64>,
    tests: &[Test],
    at: usize,
) {
    let mut row = from + 1;
    into.put(3, row, " Quick Run ", Ink::Selected);
    into.put(20, row, " Run Selected ", Ink::Plain);
    into.put_right(into.width().saturating_sub(3), row, " Back ", Ink::Plain);
    row += 1;
    into.put(3, row, &span(40), Ink::Quiet);
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
    tests_table(into, row + 2, tests, at, chosen);
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

/// The tests, and what has been chosen of them.
fn tests_table(into: &mut Screen, from: usize, tests: &[Test], at: usize, chosen: u64) {
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
}
