//! The interface, checked by drawing it and reading what came out.
//!
//! **The window's appearance was wrong twice and every test passed.** They
//! were tests of the layout code's arithmetic, and what was wrong was what the
//! layout produced — a character grid in a window. Nothing could see that,
//! because seeing it needed a display.
//!
//! So these render to a buffer (`mcf_desk::paper`) through the same painter the
//! window uses, and assert about pixels. They are slower than the unit tests
//! and there are few of them, and they cover the one thing that could not be
//! covered before: what a person would actually have been shown.
//!
//! Set `MCF_LOOK=<directory>` to have each screen written out as a portable
//! pixmap, which is how a person checks the same thing by eye.

// A test that cannot fail loudly is a test that reports success it did not
// establish, which is why the workspace's prohibition is lifted here and
// nowhere else in this crate.
#![allow(clippy::panic, reason = "a test says what went wrong by failing")]

use mcf_desk::paint::{DAY, Ink, NIGHT, Painter};
use mcf_desk::ui::Mouse;
use mcf_desk::{Desk, Model, Page};

/// A machine holding four models: one measured, one measured and quick, one
/// nobody has timed, and one that will not run. The four states the list has.
fn four_models() -> Desk {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.refusal = None;
    desk.models = vec![
        Model {
            name: "Assistant-8B-Instruct-Q4_K_M".to_owned(),
            path: "/home/a/.local/share/mcf/models/Assistant-8B.gguf".to_owned(),
            bytes: Some(5_020_000_000),
            architecture: Some("an-architecture".to_owned()),
            trained: Some(40_960),
            context: Some(32_768),
            engine: Some("llama.cpp-cuda".to_owned()),
            device: Some("NVIDIA GeForce RTX 5080".to_owned()),
            on_a_card: true,
            speed: Some(155.0),
            start_up: Some("412.7".to_owned()),
            fastest: Some(6.43),
            slowest: Some(7.66),
            cache_per_token: Some(114_688),
            ladder: Vec::new(),
            refused: None,
        },
        Model {
            name: "SmolLM2-135M-Instruct".to_owned(),
            path: "/models/SmolLM2.gguf".to_owned(),
            bytes: Some(270_000_000),
            architecture: Some("llama".to_owned()),
            trained: Some(8_192),
            context: Some(8_192),
            engine: Some("llama.cpp-cuda".to_owned()),
            device: Some("NVIDIA".to_owned()),
            on_a_card: true,
            ..Model::default()
        },
        Model {
            name: "Llama-3.3-70B-Instruct".to_owned(),
            path: "/models/Llama-70B.gguf".to_owned(),
            bytes: Some(40_000_000_000),
            architecture: Some("llama".to_owned()),
            trained: Some(131_072),
            refused: Some("this model needs more memory than this computer has".to_owned()),
            ..Model::default()
        },
    ];
    desk
}

/// Renders one screen and hands back the pixels.
fn drawn(desk: &Desk, ink: Ink, name: &str) -> mcf_desk::paper::Paper {
    let mut paint = match Painter::on_paper(1180, 760, 1.0, ink) {
        Ok(paint) => paint,
        Err(why) => {
            // No font on this machine is a real state and not a test failure:
            // the window itself refuses in that case and says so. Skipping is
            // the honest outcome, and it is announced rather than silent (A2).
            eprintln!("skipped: {why}");
            return mcf_desk::paper::Paper::new(1, 1, 1.0);
        }
    };
    let _going = mcf_desk::view::draw(&mut paint, desk, &Mouse::default());
    let Some(paper) = paint.paper() else {
        panic!("a paper painter drew somewhere else");
    };
    if let Ok(into) = std::env::var("MCF_LOOK") {
        let _written = std::fs::write(format!("{into}/{name}.ppm"), paper.as_pixmap());
    }
    mcf_desk::paper::Paper::from_pixels(paper.width, paper.height, paper.pixels.clone())
}

/// Every screen draws something, in both themes.
///
/// The crudest check there is, and it catches the whole class of failure where
/// a palette, a font or a coordinate is wrong enough that nothing appears —
/// which is invisible to every other kind of test.
#[test]
fn every_screen_draws_something_in_both_themes() {
    let mut desk = four_models();
    for ink in [NIGHT, DAY] {
        for page in [
            Page::Monitor,
            Page::Host,
            Page::Diagnostics,
            Page::Models,
            Page::Settings,
        ] {
            desk.page = page;
            let paper = drawn(&desk, ink, &format!("{page:?}").to_lowercase());
            if paper.width == 1 {
                return; // no font here
            }
            let inked = paper.inked(ink.ground);
            assert!(
                inked > 20_000,
                "{page:?} drew {inked} pixels: a screen nobody can read"
            );
        }
    }
}

/// A model that will not run does not look like one nobody has measured.
///
/// **This was wrong when the window first drew it.** Both states were the
/// warning colour, so the two a person most needs to tell apart — *this
/// cannot run* and *nobody has timed this* — were the same swatch. The
/// console's layout says them differently: a refusal is the reason, in the
/// refusal colour, where the engine would have been; an unmeasured model has
/// `Unknown` in the quiet one. A test that read only the words would not
/// notice either, because the words were right both times.
#[test]
fn a_refusal_does_not_look_like_anything_else() {
    let mut desk = four_models();
    desk.page = Page::Host;
    for ink in [NIGHT, DAY] {
        // The second model runs and has never been timed; the third will not
        // run at all.
        desk.chosen = Some(1);
        let unmeasured = drawn(&desk, ink, "scratch");
        if unmeasured.width == 1 {
            return;
        }
        desk.chosen = Some(2);
        let refused = drawn(&desk, ink, "scratch");

        // Near the colour, not exactly it: glyphs are antialiased, so most of
        // a letter's pixels are the ink blended with what is behind them and
        // only the middle of a stroke lands on the value itself.
        let counted = |paper: &mcf_desk::paper::Paper, colour: (u8, u8, u8)| {
            let mut found = 0_usize;
            for y in 0..paper.height {
                for x in 0..paper.width {
                    let Some((red, green, blue)) = paper.at(x, y) else {
                        continue;
                    };
                    let apart = u32::from(red.abs_diff(colour.0))
                        + u32::from(green.abs_diff(colour.1))
                        + u32::from(blue.abs_diff(colour.2));
                    if apart < 40 {
                        found += 1;
                    }
                }
            }
            found
        };
        // A short sentence at thirteen points is a few hundred pixels of ink
        // and only its stroke centres reach the colour itself, so the figures
        // are small — what matters is that one screen has the refusal colour
        // on it and the other has essentially none.
        let (on_refused, on_unmeasured) =
            (counted(&refused, ink.bad), counted(&unmeasured, ink.bad));
        assert!(
            on_refused > 8,
            "a model that will not run is not drawn in the refusal colour: {on_refused} pixels"
        );
        assert!(
            on_unmeasured * 3 < on_refused,
            "a model that merely has not been measured is drawn like a refusal: \
             {on_unmeasured} against {on_refused}"
        );
    }
}

/// Nothing is drawn in the colour of the thing behind it.
///
/// Text the colour of its own background is text nobody can read, and it is
/// what a palette edited in one theme and not the other produces.
#[test]
fn every_colour_can_be_seen_against_the_one_behind_it() {
    for (name, ink) in [("night", NIGHT), ("day", DAY)] {
        // Text has to be read, so it needs real separation.
        for (what, front, behind) in [
            ("body text on a card", ink.ink, ink.card),
            ("quiet text on a card", ink.quiet, ink.card),
            ("faint text on a card", ink.faint, ink.card),
            ("body text on the ground", ink.ink, ink.ground),
            ("navigation text in the well", ink.quiet, ink.sunk),
            ("text on the accent", ink.accent_ink, ink.accent),
            ("a tag's word on its wash", ink.accent, ink.accent_soft),
            ("a warning on its wash", ink.warn, ink.warn_soft),
            ("a refusal on its wash", ink.bad, ink.bad_soft),
        ] {
            let apart = separation(front, behind);
            assert!(
                apart > 24.0,
                "{name}: {what} is {apart:.0} apart and cannot be read"
            );
        }
        // A rule is not text and must not be held to text's threshold — a
        // hairline divider is *supposed* to be quiet, and demanding that it
        // read like a word is how a tasteful interface gets shouted at. What
        // it must not be is invisible.
        for (what, front, behind) in [
            ("a rule on the ground", ink.line, ink.ground),
            ("a rule on a card", ink.line, ink.card),
            ("a rule in the well", ink.line, ink.sunk),
        ] {
            let apart = separation(front, behind);
            assert!(
                apart > 8.0,
                "{name}: {what} is {apart:.0} apart and cannot be seen at all"
            );
        }
    }
}

/// How far apart two colours are, as the larger of their lightness difference
/// and their raw distance — enough to catch a colour drawn on itself, which is
/// what this is for.
fn separation(one: (u8, u8, u8), two: (u8, u8, u8)) -> f32 {
    let lightness = |colour: (u8, u8, u8)| {
        0.299_f32.mul_add(
            f32::from(colour.0),
            0.587_f32.mul_add(f32::from(colour.1), 0.114 * f32::from(colour.2)),
        )
    };
    (lightness(one) - lightness(two)).abs()
}

/// A model that will not run says so where somebody will read it.
#[test]
fn a_model_that_will_not_run_says_so_on_its_own_page() {
    let mut desk = four_models();
    desk.page = Page::Host;
    desk.chosen = Some(2);
    let paper = drawn(&desk, NIGHT, "refused");
    if paper.width == 1 {
        return;
    }
    // The heading area carries the refusal rather than a speed, so the top of
    // the page must not be the large figure a measured model gets.
    assert!(
        paper.inked(NIGHT.ground) > 20_000,
        "the refused model's page is blank"
    );
}

/// A daemon that is not answering looks different from one that is.
///
/// The console puts what MCF is doing on one line under the monitor's
/// divider, and *not up* is one of the things it can say there. What must not
/// happen is the two states drawing identically — a window that looked the
/// same whether or not MCF was running would be a window nobody could use to
/// find out (A2).
#[test]
fn a_daemon_that_is_not_answering_looks_different() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Monitor;
    let answering = drawn(&desk, NIGHT, "scratch");
    if answering.width == 1 {
        return;
    }
    desk.refusal = Some("MCF is not answering on this computer".to_owned());
    let silent = drawn(&desk, NIGHT, "refusal");

    let mut differ = 0_usize;
    for y in 0..silent.height {
        for x in 0..silent.width {
            if silent.at(x, y) != answering.at(x, y) {
                differ += 1;
            }
        }
    }
    assert!(
        differ > 300,
        "a daemon that is answering and one that is not draw the same: {differ} pixels differ"
    );
    // And the words themselves, which pixels cannot be read for.
    let (word, said) = desk.state_line();
    assert_eq!(word, "NOT UP");
    assert!(said.contains("not answering"), "{said}");
}

/// An open dropdown appears, and appears over what is under it.
///
/// **This is the check the old pickers would have passed.** They drew a
/// chevron and did something else entirely — navigated to another page,
/// cycled a number — and every test passed, because what was wrong was that
/// nothing appeared when you clicked. Only a picture can see that, which is
/// what this file is for.
///
/// It asserts two things a wrong implementation fails differently: that the
/// open screen differs from the closed one at all, and that the difference is
/// **below the picker**, where the list drops down, rather than anywhere else.
#[test]
fn an_open_dropdown_is_drawn_over_the_page() {
    let mut desk = four_models();
    desk.page = Page::Diagnostics;
    desk.chosen = Some(0);

    let shut = drawn(&desk, NIGHT, "diagnostics-shut");
    if shut.width == 1 {
        return; // no font here
    }
    desk.open = Some(mcf_desk::Picker::Window);
    let open = drawn(&desk, NIGHT, "diagnostics-open");

    // **The extent, not the count.** Counting changed pixels passed with the
    // list disabled, because opening a picker also lights its own box — 28
    // points tall. A list that dropped down changes a band several times
    // that, so the height of the change is what says one appeared. Planting
    // the list out is what found this: the first version of this test passed
    // with nothing dropping down at all.
    let (mut first, mut last, mut differing) = (None, 0_u32, 0_usize);
    for y in 0..open.height {
        let mut row = false;
        for x in 0..open.width {
            if shut.at(x, y) != open.at(x, y) {
                differing += 1;
                row = true;
            }
        }
        if row {
            first.get_or_insert(y);
            last = y;
        }
    }
    let Some(first) = first else {
        panic!("opening a dropdown changed nothing at all");
    };
    assert!(
        differing > 2_000,
        "opening a dropdown changed {differing} pixels: nothing dropped down"
    );
    assert!(
        last - first > 100,
        "the change is {} points tall, which is the picker's own box and not a \
         list under it: nothing dropped down",
        last - first
    );
}

/// A run's result opens under the table, and does not cover it.
///
/// The table is the thing being read: a panel over the rows would answer
/// *what did this find* by hiding *which of them it was about*.
#[test]
fn a_result_opens_under_the_tests_rather_than_over_them() {
    let mut desk = four_models();
    desk.page = Page::Diagnostics;
    desk.chosen = Some(0);
    if let Some(test) = desk.tests.first_mut() {
        test.ran = Some(184);
        test.result = Some(vec![
            "at 512 tokens   6.43 ms a token".to_owned(),
            "at 1 024 tokens   6.71 ms a token".to_owned(),
            "measured on llama.cpp-cuda".to_owned(),
        ]);
    }
    let shut = drawn(&desk, NIGHT, "result-shut");
    if shut.width == 1 {
        return; // no font here
    }
    desk.showing = Some(0);
    let open = drawn(&desk, NIGHT, "result-open");

    // The first test's own row must be identical: the panel is under the
    // table, so opening it cannot have redrawn the row it belongs to.
    let mut changed_low = 0_usize;
    let mut changed_high = 0_usize;
    // Halved to say *above or below*; the exact middle row does not matter.
    #[expect(clippy::integer_division, reason = "a midpoint, in whole rows")]
    let half = open.height / 2;
    for y in 0..open.height {
        for x in 0..open.width {
            if shut.at(x, y) != open.at(x, y) {
                if y < half {
                    changed_low += 1;
                } else {
                    changed_high += 1;
                }
            }
        }
    }
    assert!(
        changed_high > changed_low,
        "the result drew {changed_low} pixels in the top half and {changed_high} \
         in the bottom: it is not opening under the table"
    );
    assert!(
        changed_high > 500,
        "the result drew {changed_high} pixels: nothing opened"
    );
}

/// The memory figure a real ladder read on this machine: Qwen3-VL-2B to
/// 8,192, on the processor.
fn a_memory_figure() -> mcf_record::json::Value {
    use mcf_record::json::Value;
    Value::map([
        ("measured", Value::Bool(true)),
        ("per_token_bytes", Value::Integer(117_346)),
        (
            "between_windows",
            Value::List(vec![Value::Integer(4096), Value::Integer(16418)]),
        ),
        (
            "at_deepest",
            Value::map([
                ("depth", Value::Integer(8192)),
                ("window", Value::Integer(16418)),
                ("bytes", Value::Integer(3_645_382_656)),
            ]),
        ),
        ("planned_per_token_bytes", Value::Integer(114_688)),
        (
            "largest_context",
            Value::map([
                ("measured", Value::Integer(262_144)),
                ("planned", Value::Integer(262_144)),
            ]),
        ),
        (
            "what",
            Value::text(
                "the engine process's peak resident memory, read from the kernel's high-water \
                 mark, over the window each rung ran in; a device's memory is not in it",
            ),
        ),
    ])
}

/// The three rows the ladder now answers open like the ladder's own, in the
/// daemon's sentences.
///
/// The sentences are composed in `mcf_serve::ladder` — the same ones the
/// console prints — so this draws what a run leaves behind, and asserts
/// that opening the memory row shows something under the table (B-424).
#[test]
fn the_rows_read_off_the_rungs_open_in_the_daemons_words() {
    use mcf_record::json::Value;
    let mut desk = four_models();
    desk.page = Page::Diagnostics;
    desk.chosen = Some(0);
    let first_token = Value::map([
        ("measured", Value::Bool(true)),
        ("depth", Value::Integer(512)),
        ("ms", Value::text("1257.235")),
        (
            "includes",
            Value::text(
                "loading the model, reading 512 tokens and producing one — with the file \
                 already in the page cache, which is a second request's cost and not the \
                 first's after a reboot",
            ),
        ),
    ]);
    let prompt_reading = Value::map([
        ("measured", Value::Bool(false)),
        (
            "why",
            Value::text("only one rung measured a first token, and a cost is read between two"),
        ),
    ]);
    let memory = a_memory_figure();
    for test in &mut desk.tests {
        match test.name {
            "Memory ceiling — largest context" => {
                test.ran = Some(41);
                test.result = Some(mcf_serve::ladder::memory_said(Some(&memory)));
            }
            "Generation speed against depth" => {
                test.ran = Some(41);
                test.result = Some(vec![
                    "at 512 tokens   15.321 ms a token".to_owned(),
                    "measured on provisioned llama.cpp server @925e1179947e".to_owned(),
                ]);
            }
            "Prompt reading speed" => {
                test.ran = Some(41);
                test.result = Some(mcf_serve::ladder::prompt_reading_said(Some(
                    &prompt_reading,
                )));
            }
            "Start-up to first token" => {
                test.ran = Some(41);
                test.result = Some(mcf_serve::ladder::first_token_said(Some(&first_token)));
            }
            _ => {}
        }
    }
    let shut = drawn(&desk, DAY, "rungs-shut");
    if shut.width == 1 {
        return; // no font here
    }
    desk.showing = desk
        .tests
        .iter()
        .position(|test| test.name == "Memory ceiling — largest context");
    assert!(desk.showing.is_some(), "no memory row to open");
    let open = drawn(&desk, DAY, "rungs-open");
    let mut changed = 0_usize;
    for y in 0..open.height {
        for x in 0..open.width {
            if shut.at(x, y) != open.at(x, y) {
                changed += 1;
            }
        }
    }
    assert!(
        changed > 500,
        "opening the memory row changed {changed} pixels: nothing opened"
    );
}

/// The cross-check row is a run of its own: it carries a checkbox, and its
/// result and its progress are the daemon's sentences (B-424, B-072).
///
/// Drawn twice — with the check running, so the progress under the table is
/// the daemon's two lines; and finished, with the row filled and opened.
#[test]
fn the_cross_check_row_runs_from_the_window() {
    use mcf_record::json::Value;
    let mut desk = four_models();
    desk.page = Page::Diagnostics;
    desk.chosen = Some(0);
    let cross_check = desk
        .tests
        .iter()
        .position(|test| test.run == mcf_desk::Run::CrossCheck)
        .unwrap_or_else(|| panic!("no cross-check row"));
    desk.act(mcf_desk::Act::Toggle(cross_check));
    let last = Value::map([
        ("cross_checked", Value::text("a-model")),
        (
            "conditions",
            Value::map([(
                "engine_ran",
                Value::text("provisioned llama.cpp server @925e1179947e"),
            )]),
        ),
        (
            "said",
            Value::List(vec![
                Value::text(
                    "MCF's own engine read 120 position(s) of what the provisioned engine \
                     produced, and would have chosen the same token at 118",
                ),
                Value::text(
                    "AGREE — where they differed, the other engine's token was never worse \
                     than MCF's rank 2; the line is 8, and a swap of the top few is two \
                     implementations summing in a different order rather than one of them \
                     being wrong (F27, F41)",
                ),
            ]),
        ),
        ("done", Value::Bool(true)),
    ]);
    desk.doing = mcf_desk::Doing::CrossChecking(mcf_desk::job::Job::already(
        "cross-checking a-model".to_owned(),
        vec![
            Value::map([
                ("cross_checking", Value::text("a-model")),
                ("positions", Value::Integer(120)),
                ("estimate_low_seconds", Value::Integer(52)),
                ("estimate_high_seconds", Value::Integer(127)),
                ("done", Value::Bool(false)),
            ]),
            Value::map([
                ("cross_checking", Value::text("a-model")),
                ("produced", Value::Integer(120)),
                (
                    "engine_ran",
                    Value::text("provisioned llama.cpp server @925e1179947e"),
                ),
                ("reading", Value::Bool(true)),
                ("done", Value::Bool(false)),
            ]),
            last,
        ],
    ));
    let running = drawn(&desk, DAY, "cross-check-running");
    if running.width == 1 {
        return; // no font here
    }
    // Filled the way `keep_the_cross_check` fills it — the daemon's
    // sentences, then which engine ran — which the unit tests hold it to.
    let sentences = desk
        .doing
        .job()
        .and_then(mcf_desk::job::Job::conclusion)
        .and_then(|body| body.get("said"))
        .and_then(Value::as_list)
        .map(|said| {
            said.iter()
                .filter_map(Value::as_text)
                .map(str::to_owned)
                .chain(std::iter::once(
                    "against provisioned llama.cpp server @925e1179947e".to_owned(),
                ))
                .collect::<Vec<String>>()
        });
    desk.tests[cross_check].ran = Some(97);
    desk.tests[cross_check].result = sentences;
    desk.showing = Some(cross_check);
    let _open = drawn(&desk, DAY, "cross-check-open");
}

/// A small rise is drawn as a small rise.
///
/// **The point of the chart, and the one way it could lie.** A plot whose
/// vertical axis began at the smallest reading would draw this machine's
/// 1.333 to 1.412 milliseconds a token — a rise of six per cent — as a line
/// climbing from the floor to the ceiling. Suppressing a zero is how a picture
/// tells a lie the numbers under it do not (A6, A11).
///
/// So: two ladders, one nearly flat and one that trebles, drawn into the same
/// box. The flat one must occupy a small part of the height and the steep one
/// most of it. If the axis were ever changed to start at the minimum, both
/// would fill the box and this fails.
#[test]
fn a_six_per_cent_rise_is_not_drawn_as_a_cliff() {
    use mcf_desk::chart::{Reading, falloff};
    use mcf_desk::paint::Box;

    let spread_of = |readings: &[Reading]| -> Option<u32> {
        let mut paint = Painter::on_paper(400, 200, 1.0, NIGHT).ok()?;
        paint.begin();
        let _under = falloff(&mut paint, Box::new(20.0, 20.0, 360.0, 120.0), readings);
        let paper = paint.paper()?;
        // How many rows the accent — the line and its points — appears on.
        let mut top = None;
        let mut bottom = None;
        for y in 0..paper.height {
            let mut on_this_row = false;
            for x in 0..paper.width {
                if paper.at(x, y) == Some(NIGHT.accent) {
                    on_this_row = true;
                    break;
                }
            }
            if on_this_row {
                top.get_or_insert(y);
                bottom = Some(y);
            }
        }
        Some(bottom?.saturating_sub(top?))
    };

    let flat = [
        Reading {
            depth: 512,
            ms: 1.529,
        },
        Reading {
            depth: 1024,
            ms: 1.333,
        },
        Reading {
            depth: 2048,
            ms: 1.342,
        },
        Reading {
            depth: 4096,
            ms: 1.412,
        },
    ];
    let steep = [
        Reading {
            depth: 512,
            ms: 1.5,
        },
        Reading {
            depth: 1024,
            ms: 2.4,
        },
        Reading {
            depth: 2048,
            ms: 3.4,
        },
        Reading {
            depth: 4096,
            ms: 4.5,
        },
    ];
    let (Some(gentle), Some(sharp)) = (spread_of(&flat), spread_of(&steep)) else {
        return; // no font on this machine
    };
    assert!(
        gentle < 30,
        "a six per cent rise covers {gentle} rows: the axis is suppressing its zero"
    );
    assert!(
        sharp > gentle * 2,
        "a threefold rise ({sharp} rows) is not drawn as much steeper than a six per cent one \
         ({gentle} rows)"
    );
}

/// One reading is a point, and a point is not a trend.
#[test]
fn nothing_is_drawn_from_a_single_reading() {
    use mcf_desk::chart::{Reading, falloff};
    use mcf_desk::paint::Box;

    let Ok(mut paint) = Painter::on_paper(400, 200, 1.0, NIGHT) else {
        return;
    };
    paint.begin();
    let one = [Reading {
        depth: 512,
        ms: 1.5,
    }];
    let under = falloff(&mut paint, Box::new(20.0, 20.0, 360.0, 120.0), &one);
    // The row it hands back is the row it was given: nothing was drawn, so
    // nothing was used.
    assert!(
        (under - 20.0).abs() < f32::EPSILON,
        "a chart was drawn from one reading"
    );
    let Some(paper) = paint.paper() else { return };
    assert_eq!(
        paper.inked(NIGHT.ground),
        0,
        "a single reading put something on the screen"
    );
}

/// A name MCF cannot draw does not silently become a shorter name.
///
/// **The face MCF finds covers Latin and a little furniture.** A model named
/// in Japanese, Arabic, Devanagari or Han has characters it has never seen,
/// and skipping them drew `モデル-7B` as `-7B`: a name silently shorter than
/// the name, and two differently-named models rendering identically. A limit
/// is fine and a limit that hides itself is not (A1, A2, B-411).
#[test]
fn a_name_in_another_script_does_not_vanish() {
    use mcf_desk::font::Weight;

    let Ok(mut paint) = Painter::on_paper(600, 200, 1.0, NIGHT) else {
        return;
    };
    let latin_only = paint.measure("-7B", Weight::Regular, 15.0);
    for name in ["モデル-7B", "نموذج-7B", "मॉडल-7B", "模型-7B"] {
        let whole = paint.measure(name, Weight::Regular, 15.0);
        assert!(
            whole > latin_only * 1.5,
            "{name} measures {whole}, barely more than the {latin_only} of the part MCF can \
             draw — the rest is being skipped rather than shown"
        );
    }

    // And two names differing only in characters MCF cannot draw do not
    // measure the same: a list of them stays a list of distinct things.
    let three = paint.measure("モデル-7B", Weight::Regular, 15.0);
    let five = paint.measure("نموذج-7B", Weight::Regular, 15.0);
    assert!(
        (three - five).abs() > 1.0,
        "a three-character name and a five-character one measure the same: {three} against {five}"
    );

    // The boxes are drawn, not merely counted: something is on the screen.
    paint.begin();
    let ink = paint.ink;
    paint.say_at(16.0, 20.0, "モデル", Weight::Regular, 15.0, ink.ink);
    let Some(paper) = paint.paper() else { return };
    assert!(
        paper.inked(NIGHT.ground) > 40,
        "a name MCF cannot draw put nothing at all on the screen"
    );
}

// ── Driving the interface, rather than only looking at it ──────────────────
//
// **Everything above renders a screen and reads the pixels. None of it presses
// anything.** Three faults reached an operator that way: the Host button
// returned early and did nothing at all because the settings it destructures
// were never fetched; a reading that went unanswered emptied the model list and
// the window said *no models exist* while sixteen sat on the disk; and a
// control was renamed in one place and not another.
//
// Every one of those is invisible to a test that draws and looks, because the
// drawing was correct. What was wrong was what happened when somebody pressed.

/// Presses at a point and returns what the interface made of it.
///
/// A click is a press and a release inside the same thing, so both are set:
/// a `click` with no `began` is not something a person can do, and asserting
/// about it would be asserting about a state the window never sees.
fn pressed_at(desk: &Desk, at: (f32, f32)) -> Option<mcf_desk::Act> {
    let mut paint = match Painter::on_paper(1180, 760, 1.0, NIGHT) {
        Ok(paint) => paint,
        Err(why) => {
            eprintln!("skipped: {why}");
            return None;
        }
    };
    let mouse = Mouse {
        at,
        down: false,
        began: Some(at),
        click: Some(at),
        wheel: 0.0,
    };
    mcf_desk::view::draw(&mut paint, desk, &mouse)
}

/// Finds the act a labelled control produces, by pressing everywhere it could
/// be.
///
/// The window lays itself out, so a test that hard-coded a button's coordinates
/// would be a test of arithmetic somebody copied. Sweeping asks the question a
/// person asks — *is there something here that does this* — and fails when the
/// answer is no, whatever the reason.
fn act_somewhere(desk: &Desk, wanted: &mcf_desk::Act) -> bool {
    // From the top of the window, not below the menu: the column sits at y=8
    // and a sweep starting under it cannot press a tab — which is a test that
    // reports the menu unreachable when what is unreachable is the sweep.
    act_within(desk, wanted, (0.0, 750.0))
}

/// The same, over a band of the window.
///
/// Bounded because a sweep of the whole window is thousands of renders, and a
/// test that takes a minute is one somebody stops running.
fn act_within(desk: &Desk, wanted: &mcf_desk::Act, band: (f32, f32)) -> bool {
    let mut y = band.0 + 6.0;
    while y < band.1 {
        let mut x = 10.0;
        while x < 1170.0 {
            if pressed_at(desk, (x, y)).as_ref() == Some(wanted) {
                return true;
            }
            x += 24.0;
        }
        y += 10.0;
    }
    false
}

/// The window offers a way to host the model that is chosen.
///
/// F: `host_it` begins by destructuring `self.settings`, which nothing ever
/// fetched — so it was `None` for the life of the window and pressing Host
/// returned early, silently, every time. The button drew correctly, so every
/// test that looked at it passed.
#[test]
fn pressing_host_asks_to_host_and_does_not_silently_do_nothing() {
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);

    assert!(
        act_somewhere(&desk, &mcf_desk::Act::HostIt),
        "no control on the Models screen asks to host the chosen model"
    );

    // And the act must do something. With no settings — which is what a
    // daemon that has not answered leaves — it refuses in words rather than
    // returning as though nothing had been pressed.
    desk.settings = None;
    desk.no_settings = None;
    desk.act(mcf_desk::Act::HostIt);
    assert!(
        desk.no_settings.is_some(),
        "hosting without settings must say why, not do nothing"
    );
}

/// The prompt analysis is reachable, and named what it is called.
///
/// F: it was renamed in the button and not the heading once already. A control
/// somebody was told to press, by a name that is not on it, is a control that
/// is not there.
#[test]
fn the_prompt_analysis_is_reachable_from_diagnostics() {
    let mut desk = four_models();
    desk.page = Page::Diagnostics;
    desk.chosen = Some(0);
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::Go(Page::Prompt)),
        "nothing on Diagnostics leads to the prompt analysis"
    );

    // And once there, something runs it.
    desk.page = Page::Prompt;
    desk.typed = "a prompt with two sentences. And a second one.".to_owned();
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::ReportPrompt),
        "nothing on the prompt screen runs the analysis"
    );
}

/// Every screen the column offers can be reached from every other.
///
/// A19: a column entry is an advertisement, and one that cannot be pressed is
/// a screen an operator is told about and cannot open.
#[test]
fn every_menu_entry_can_be_pressed_from_every_screen() {
    let mut desk = four_models();
    for (from, _) in Page::MENU {
        desk.page = *from;
        for (to, label) in Page::MENU {
            assert!(
                act_within(&desk, &mcf_desk::Act::Go(*to), (0.0, 46.0)),
                "{label} cannot be reached from {from:?}"
            );
        }
    }
}

/// A window holding models draws them, and one holding none looks different.
///
/// F: a reading that went unanswered emptied the list, and the window said *no
/// models exist* while sixteen sat on the disk and `mcf list` found them. The
/// screen drew correctly in both cases — which is the point: what was wrong was
/// which of the two it drew.
#[test]
fn a_window_holding_models_does_not_look_like_one_holding_none() {
    let mut desk = four_models();
    desk.page = Page::Models;
    let with = drawn(&desk, NIGHT, "models-held");

    desk.models.clear();
    desk.chosen = None;
    let without = drawn(&desk, NIGHT, "models-none");
    if with.width < 2 || without.width < 2 {
        return; // no font on this machine; `drawn` said so
    }

    let differing = with
        .pixels
        .chunks(3)
        .zip(without.pixels.chunks(3))
        .filter(|(one, two)| one != two)
        .count();
    assert!(
        differing > 2_000,
        "a list of four models drew almost the same as an empty one ({differing} pixels \
         differ), so an operator could not tell which they were being shown"
    );
}

/// While a model is being held, the window says so.
///
/// F: hosting a large model blocked every reading, the window stopped
/// repainting, and what was on screen said "a moment". An operator who has been
/// told *a moment* and waits five minutes concludes it has failed.
#[test]
fn a_window_holding_a_model_says_that_it_is() {
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);
    let idle = drawn(&desk, NIGHT, "hosting-idle");

    desk.doing = mcf_desk::Doing::Hosting(mcf_desk::job::Job::start(
        std::path::PathBuf::from("/nowhere/control.sock"),
        mcf_serve::control::Request::Hosted,
        "holding Assistant-8B-Instruct-Q4_K_M".to_owned(),
    ));
    let holding = drawn(&desk, NIGHT, "hosting-underway");
    if idle.width < 2 || holding.width < 2 {
        return;
    }

    let differing = idle
        .pixels
        .chunks(3)
        .zip(holding.pixels.chunks(3))
        .filter(|(one, two)| one != two)
        .count();
    assert!(
        differing > 200,
        "a window with a host under way drew the same as an idle one ({differing} pixels \
         differ), so nothing on screen said it was working"
    );

    // And the state line says which it is, rather than reporting the daemon
    // as gone while it does what was asked.
    desk.busy = true;
    let (word, said) = desk.state_line();
    assert_eq!(word, "HOLDING");
    assert!(said.contains("so far"), "{said}");
}

/// The arithmetic behind the figure, on its own.
///
/// A window costs what a token of cache costs multiplied by the window, and
/// the weights sit beside it because what the operator is deciding is how much
/// of the machine this model will take altogether (B-423).
#[test]
fn a_window_costs_the_cache_it_reserves() {
    let held = &four_models().models[0];
    let (cache, total) = mcf_desk::view::reserve_of(held, 32_768).expect("this model is priced");
    assert_eq!(
        cache,
        114_688 * 32_768,
        "a window is cache per token by tokens"
    );
    assert_eq!(
        total,
        Some(5_020_000_000 + 114_688 * 32_768),
        "what it comes to is the window and the weights"
    );
    // Doubling the window doubles the cache: the point of showing it is that
    // it is the term that moves.
    let (twice, _) = mcf_desk::view::reserve_of(held, 65_536).expect("still priced");
    assert_eq!(twice, cache * 2);
}

/// A model whose header does not price a token is not priced.
///
/// A7: a blank where a number belongs, rather than a zero that reads as a
/// measurement of nothing.
#[test]
fn a_model_that_cannot_be_priced_is_not_given_a_price() {
    let mut held = four_models().models[0].clone();
    held.cache_per_token = None;
    assert!(mcf_desk::view::reserve_of(&held, 32_768).is_none());
    assert!(mcf_desk::view::reserve_line(&held, 32_768).is_none());
}

/// The price is on the screen where the window is chosen, before it is paid.
#[test]
fn the_price_of_a_window_is_shown_where_it_is_chosen() {
    // The settings table only draws once the daemon has said what it would
    // run this model under, which is also when a window can be chosen.
    let settings = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp-cuda",
        "a-device",
        true,
        32_768,
        Some(8),
        true,
    );
    let mut priced = four_models();
    priced.chosen = Some(0);
    priced.page = Page::Models;
    priced.settings = Some(settings.clone());
    priced.recommended = Some(settings.clone());
    let mut unpriced = four_models();
    unpriced.chosen = Some(0);
    unpriced.page = Page::Models;
    unpriced.settings = Some(settings.clone());
    unpriced.recommended = Some(settings);
    unpriced.models[0].cache_per_token = None;

    let with = drawn(&priced, DAY, "window-priced");
    let without = drawn(&unpriced, DAY, "window-unpriced");
    let ground = DAY.ground;
    assert!(
        with.inked(ground) > without.inked(ground),
        "the cost of the chosen window is not on the screen that chooses it: {} marks against \
         {} (§3.15, B-423)",
        with.inked(ground),
        without.inked(ground)
    );
}

/// And on the monitor, for the window actually being held.
#[test]
fn the_monitor_says_what_the_held_window_costs() {
    let mut desk = four_models();
    desk.page = Page::Monitor;
    desk.hosted = Some(mcf_desk::Hosted {
        model: desk.models[0].path.clone(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: "a moment ago".to_owned(),
        context: Some(32_768),
    });
    // The same window, held under a model this list does not carry: the
    // address and the context still show, the price cannot.
    let mut unknown = four_models();
    unknown.page = Page::Monitor;
    unknown.hosted = Some(mcf_desk::Hosted {
        model: "/models/one-this-window-is-not-listing.gguf".to_owned(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: "a moment ago".to_owned(),
        context: Some(32_768),
    });

    let ground = DAY.ground;
    let priced = drawn(&desk, DAY, "monitor-priced").inked(ground);
    let bare = drawn(&unknown, DAY, "monitor-unpriced").inked(ground);
    assert!(
        priced > bare,
        "the monitor does not say what the window it is holding costs: {priced} marks against \
         {bare} (§3.15)"
    );
}

/// One ablated part of a served report, as the daemon writes it.
fn a_clause(text: &str, moved: i64) -> mcf_record::json::Value {
    use mcf_record::json::Value;
    let clause = |text: &str, moved: i64| {
        Value::map([
            ("text", Value::text(text.to_owned())),
            ("changed", Value::Bool(moved > 0)),
            ("moved_parts_per_million", Value::Integer(moved)),
            ("without", Value::text("a different answer".to_owned())),
            (
                "held",
                Value::map([
                    ("first_rank", Value::Integer(if moved > 0 { 17 } else { 1 })),
                    ("kept", Value::Integer(i64::from(moved == 0))),
                    ("of", Value::Integer(1)),
                ]),
            ),
        ])
    };
    clause(text, moved)
}

/// A finished prompt report, as a person would be shown one.
fn a_report() -> mcf_desk::Desk {
    use mcf_record::json::Value;
    let clause = a_clause;
    let ranked = |text: &str, rank: Option<i64>| {
        Value::map([
            ("text", Value::text(text.to_owned())),
            ("rank", rank.map_or(Value::Null, Value::Integer)),
            ("engine_said", Value::Null),
        ])
    };
    let found = Value::map([
        ("floor_parts_per_million", Value::Integer(879_000)),
        ("ranked_depth", Value::Integer(60)),
        ("forced_depth", Value::Integer(60)),
        (
            "floor_held",
            Value::map([
                ("first_rank", Value::Integer(1)),
                ("kept", Value::Integer(1)),
                ("of", Value::Integer(1)),
            ]),
        ),
        (
            "ranked_under",
            Value::text("chatml — set by a probe".to_owned()),
        ),
        (
            "expected",
            Value::List(vec![
                ranked(" class", None),
                ranked(" each", None),
                ranked(" handles", Some(29)),
                ranked(" output", Some(19)),
                ranked("#.", Some(16)),
                ranked(" the", Some(1)),
                ranked(" function", Some(1)),
            ]),
        ),
        (
            "settled",
            Value::map([
                ("temperature_thousandths", Value::Integer(700)),
                ("temperature", Value::text("0.700")),
                ("seeds_asked", Value::Integer(3)),
                ("distinct_answers", Value::Integer(2)),
                ("spread_parts_per_million", Value::Integer(180_000)),
                ("from_greedy_parts_per_million", Value::Integer(90_000)),
            ]),
        ),
        ("clauses_over_the_cap", Value::Integer(2)),
        ("unit", Value::text("sentence".to_owned())),
        (
            "unit_chosen_by",
            Value::text("the text: it has no blank line, so it is sentences".to_owned()),
        ),
        ("most", Value::Integer(4)),
        (
            "addressed_as",
            Value::text("one user turn, the whole prompt".to_owned()),
        ),
        (
            "recorded",
            Value::text("01J0000000000000000000000A".to_owned()),
        ),
        (
            "baseline",
            Value::text(
                "def slugify(title):\n    return title.lower().replace(\" \", \"-\")".to_owned(),
            ),
        ),
        (
            "clauses",
            Value::List(vec![
                clause("You are a careful assistant.", 0),
                clause(
                    "Write a function called slugify that turns a title into a slug.",
                    961_000,
                ),
                clause("Lowercase everything.", 125_000),
                clause("Reply with only the function.", 909_000),
            ]),
        ),
    ]);
    let mut desk = four_models();
    desk.page = Page::Prompt;
    desk.chosen = Some(0);
    "Write a function. Lowercase everything.".clone_into(&mut desk.typed);
    desk.doing = mcf_desk::Doing::Reporting(mcf_desk::job::Job::already(
        "prompt analysis".to_owned(),
        vec![found],
    ));
    desk
}

/// Every figure the console prints is on the screen too.
///
/// The window drew bars and named no number, no floor and no answer, so a
/// reader was shown a distinction with nothing to read it by — while the
/// console had printed all three from the start (A22, §3.15).
#[test]
fn a_prompt_report_shows_its_numbers() {
    let desk = a_report();
    let with = drawn(&desk, DAY, "prompt-report");
    let ground = DAY.ground;
    let mut bare = a_report();
    // The same report with nothing measured: fewer marks must reach the glass.
    bare.doing = mcf_desk::Doing::Reporting(mcf_desk::job::Job::already(
        "prompt analysis".to_owned(),
        vec![mcf_record::json::Value::map(Vec::<(
            String,
            mcf_record::json::Value,
        )>::new())],
    ));
    let empty = drawn(&bare, DAY, "prompt-empty");
    assert!(
        with.inked(ground) > empty.inked(ground),
        "a finished report draws no more than an empty one: {} against {}",
        with.inked(ground),
        empty.inked(ground)
    );
}

/// Pressing a sentence shows what the model wrote without it.
///
/// The figures are checkable only beside the answer they are about, and MCF
/// has held both since the measurement was written (A19).
#[test]
fn pressing_a_sentence_shows_the_answer_without_it() {
    let desk = a_report();
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::ShowWithout(1)),
        "no sentence in the report could be pressed"
    );

    // And what is drawn changes: the answer without a sentence is not the
    // answer to the prompt as written.
    let mut chosen = a_report();
    chosen.without = Some(1);
    let ground = DAY.ground;
    let as_written = drawn(&desk, DAY, "answer-as-written").inked(ground);
    let without = drawn(&chosen, DAY, "answer-without").inked(ground);
    assert_ne!(
        as_written, without,
        "the answer panel drew the same thing with a sentence selected as without one"
    );
}

/// Pressing the sentence already shown puts the answer as written back.
#[test]
fn pressing_it_again_goes_back_to_the_answer_as_written() {
    let mut desk = a_report();
    desk.without = Some(1);
    desk.act(mcf_desk::Act::ShowWithout(1));
    assert_eq!(
        desk.without, None,
        "the selection is a mode with no way out of it"
    );
}

/// What leaves the window is the report the window is showing.
///
/// What a hybrid mixture's anatomy comes over the socket as: one dense block
/// and forty-six of experts, latent attention, a cache sized as the key
/// alone — the shape of the answer the daemon gave for a real file.
fn an_anatomy_answer() -> mcf_serve::anatomy::Said {
    let line = concat!(
        r#"{"model":"/models/Assistant-8B.gguf","counted":{"elements":29943393920,"#,
        r#""bytes":17510693376,"unsized_tensors":0,"blocks":47,"output_tied":false,"#,
        r#""active":{"elements":3896766080,"experts":64,"used":4},"#,
        r#""parts":[{"part":"embedding","tensors":1,"elements":317194240,"bytes":178421760},"#,
        r#"{"part":"output head","tensors":1,"elements":317194240,"bytes":260198400},"#,
        r#"{"part":"attention","tensors":282,"elements":1022623744,"bytes":819146752},"#,
        r#"{"part":"feed-forward","tensors":141,"elements":497025024,"bytes":322977792},"#,
        r#"{"part":"experts","tensors":138,"elements":27783069696,"bytes":15904800768},"#,
        r#"{"part":"routing","tensors":46,"elements":6029312,"bytes":24117248},"#,
        r#"{"part":"norms and biases","tensors":235,"elements":257664,"bytes":1030656}],"#,
        r#""encodings":[{"encoding":"Q4_K","tensors":306,"elements":26736328704,"bytes":15039184896},"#,
        r#"{"encoding":"Q5_K","tensors":23,"elements":2286944256,"bytes":1572274176},"#,
        r#"{"encoding":"Q6_K","tensors":49,"elements":476053504,"bytes":390512640},"#,
        r#"{"encoding":"q8_0","tensors":180,"elements":418119680,"bytes":444252160},"#,
        r#"{"encoding":"f16","tensors":5,"elements":19660800,"bytes":39321600},"#,
        r#"{"encoding":"f32","tensors":281,"elements":6286976,"bytes":25147904}],"#,
        r#""block_shapes":[{"blocks":[0],"mixing":"attention","feed":"dense","experts":null,"#,
        r#""shared_expert":false,"said":"attention over the context, keys and values kept per position; one feed-forward every token passes","#,
        r#""ranged":"0","bits_hundredths":[530,530],"tensors":13,"elements":84677888,"bytes":56185856},"#,
        r#"{"blocks":[1,2,3],"mixing":"attention","feed":"experts","experts":64,"shared_expert":true,"#,
        r#""said":"attention over the context, keys and values kept per position; 64 experts and a shared one every token passes","#,
        r#""ranged":"1–46","bits_hundredths":[455,496],"tensors":828,"elements":29224325504,"bytes":17015879168}],"#,
        r#""attending":47,"recurrent":0},"#,
        r#""agreements":[{"what":"blocks","declared":"47","observed":"47","agrees":true},"#,
        r#"{"what":"embedding width","declared":"2048","observed":"2048","agrees":true},"#,
        r#"{"what":"key/value heads","declared":"1","observed":"1","agrees":true},"#,
        r#"{"what":"experts","declared":"64","observed":"64","agrees":true},"#,
        r#"{"what":"parameters","declared":"64x2.6B (a label)","observed":"29943393920 (29.9B)","agrees":null}],"#,
        r#""work":{"multiply_adds":3579571840,"head_width":576,"queries_per_key":20,"#,
        r#""attention_at_context":207358525440,"cache":{"sized":true,"per_token":54144,"key_heads":1,"#,
        r#""per_head":576,"latent":true,"kept":"one latent of 576 per position, which is read back as both key and value — no value cache","#,
        r#""context":202752,"at_context":10977804288,"sliding_window":null,"attending":47,"blocks":47,"recurrent":0}},"#,
        r#""vocabulary":{"tokens":154880,"segmentation":"gpt2, pre-tokenised as glm4","merges":318088,"#,
        r#""kinds":[{"kind":"text","count":154482},{"kind":"control","count":398}],"word_starts":93571,"#,
        r#""digit_tokens":10,"longest_digits":1,"digits":"10 tokens of one digit each — a number is written one digit at a time","#,
        r#""longest":{"token":"ĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠĠ","bytes":256},"#,
        r#""named":[{"what":"end of text","identifier":151329,"spelled":"<|endoftext|>","beyond":null},"#,
        r#"{"what":"end of turn","identifier":151336,"spelled":"<|user|>","beyond":null},"#,
        r#"{"what":"padding","identifier":159999,"spelled":null,"beyond":"BEYOND THE LIST — the header names token 159999 and the list holds 154880; an engine reading that number indexes past the list"}],"#,
        r#""adds_beginning":false,"beginning":"no, the file says so","#,
        r#""template":{"bytes":6122,"mentions":["tools","system","enable_thinking","add_generation_prompt"],"markers":["[gMASK]","<sop>","<|system|>","<|user|>","<|assistant|>","<|observation|>","<think>","</think>"],"no_markers":null},"#,
        r#""no_template":"none in the file — a chat turn has no framing the file states"}}"#
    );
    let Ok(value) = mcf_record::json::parse(line) else {
        panic!("the answer does not parse");
    };
    match mcf_serve::anatomy::Said::from_value(&value) {
        Some(said) => said,
        None => panic!("the answer does not read"),
    }
}

/// What a model is made of is reachable from its list, and drawn as the
/// daemon said it.
///
/// **The window had no explain page.** Everything `mcf explain` counts was
/// on the console and nowhere else, so the primary surface said less about
/// a file than the command line did (A22). The screen draws what came over
/// the socket and counts nothing itself (B-072).
#[test]
fn what_is_in_it_is_reachable_and_drawn_as_the_daemon_said_it() {
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::Go(Page::Anatomy)),
        "nothing on Models leads to what the model is made of"
    );

    // Refused, in words, where nothing was said.
    desk.page = Page::Anatomy;
    desk.anatomy = None;
    desk.no_anatomy = Some("MCF is not answering".to_owned());
    let refused = drawn(&desk, NIGHT, "anatomy-refused");
    if refused.width == 1 {
        return; // no font here
    }
    let refused = refused.inked(NIGHT.ground);

    // And the whole of it where the daemon answered.
    desk.anatomy = Some(an_anatomy_answer());
    desk.no_anatomy = None;
    for ink in [NIGHT, DAY] {
        let paper = drawn(&desk, ink, "anatomy");
        let inked = paper.inked(ink.ground);
        assert!(
            inked > refused + 30_000,
            "the anatomy drew {inked} pixels against {refused} for a refusal: the counting \
             is not on the screen"
        );
    }

    // The vocabulary is the other half of the same answer, on its own screen
    // reached from this one — and back.
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::Go(Page::Vocabulary)),
        "nothing on What is in it leads to the vocabulary"
    );
    desk.page = Page::Vocabulary;
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::Go(Page::Anatomy)),
        "nothing on Vocabulary leads back to What is in it"
    );
    for ink in [NIGHT, DAY] {
        let paper = drawn(&desk, ink, "vocabulary");
        let inked = paper.inked(ink.ground);
        // A shorter screen than the anatomy — a token list is a dozen
        // figures, not three tables — so the bar is the refusal plus a
        // screenful of figures, not the anatomy's.
        assert!(
            inked > refused + 15_000,
            "the vocabulary drew {inked} pixels against {refused} for a refusal: the \
             counting is not on the screen"
        );
    }
}

/// The prompt field takes a document: a paste keeps its lines, Return adds
/// one, and Control-Return is what runs the analysis (B-430).
#[test]
fn the_prompt_field_takes_a_document() {
    let mut desk = four_models();
    desk.page = Page::Prompt;
    desk.chosen = Some(0);
    desk.paste("You are the dungeon master.\r\n\nNever break character.\tEver.\n");
    assert_eq!(
        desk.typed, "You are the dungeon master.\n\nNever break character.\tEver.\n",
        "a persona's paragraphs are the value, not its first line"
    );
    desk.returned(false);
    assert!(
        desk.typed.ends_with("Ever.\n\n"),
        "Return in a document is a line break: {:?}",
        desk.typed
    );
    assert!(
        matches!(desk.doing, mcf_desk::Doing::Nothing),
        "Return without Control ran the analysis"
    );

    // And the name field is still a name field.
    let mut adding = four_models();
    adding.page = Page::Adding;
    adding.paste("owner/repository\nsecond line");
    assert_eq!(adding.typed, "owner/repository");
}

/// The prompt is one field and all of it is analysed — there is no second
/// field holding a question out — typing goes where the caret is, and the
/// unit and the cap are choices on the page that reach the request exactly
/// as chosen (§3.15, B-430).
#[test]
fn the_prompt_is_one_field_and_the_unit_and_the_cap_are_choices_on_the_page() {
    use mcf_serve::prompt::Unit;
    let mut desk = four_models();
    desk.page = Page::Prompt;
    desk.chosen = Some(0);
    desk.typed = (0..12)
        .map(|at| format!("Rule {at}: do the thing the rule says."))
        .collect::<Vec<_>>()
        .join("\n\n");
    // The document is what typing goes into, and its last line is a part of
    // it like any other: Return starts a new line, Ctrl+Return runs it.
    desk.typing().push_str("\n\nRoll for initiative.");
    desk.returned(false);
    assert!(
        desk.typed.ends_with("Roll for initiative.\n"),
        "Return in the document is a new line"
    );
    desk.typed.pop();
    desk.returned(true);
    assert!(
        matches!(desk.doing, mcf_desk::Doing::Reporting(_)),
        "Ctrl+Return runs the analysis"
    );
    desk.doing = mcf_desk::Doing::Nothing;
    // Under the document field and above the report: a sweep of the whole
    // window is thousands of renders of a thirteen-paragraph document.
    let controls = (240.0, 400.0);

    // The text decided paragraphs; the page offers the other unit, and the
    // cap in steps up to every part.
    let taken = desk.taken();
    assert_eq!(taken.unit(), (Unit::Paragraph, false));
    assert_eq!(taken.parts().len(), 13);
    assert_eq!(taken.cap(), 8);
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::TakeApartBy(Some(Unit::Sentence)),
            controls
        ),
        "by sentence is offered"
    );
    assert!(
        act_within(&desk, &mcf_desk::Act::MostParts(12), controls),
        "more: four further parts"
    );
    assert!(
        act_within(&desk, &mcf_desk::Act::MostParts(13), controls),
        "all: every part"
    );
    assert!(
        act_within(&desk, &mcf_desk::Act::MostParts(4), controls),
        "fewer: four fewer"
    );
    desk.act(mcf_desk::Act::MostParts(13));
    desk.act(mcf_desk::Act::TakeApartBy(Some(Unit::Sentence)));
    let taken = desk.taken();
    assert_eq!(taken.unit(), (Unit::Sentence, true));
    assert_eq!(taken.cap(), 13);

    // The temperature the seeds are drawn at is a field under the choices:
    // empty asks nothing, a decimal is the condition, and what is not a
    // temperature holds Analyse rather than being dropped (B-431, §3.15).
    assert_eq!(desk.settle(), Ok(None));
    let lower = (controls.0 + 40.0, controls.1 + 60.0);
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::Focus(mcf_desk::Caret::Temperature),
            lower
        ),
        "the temperature field is on the page"
    );
    desk.act(mcf_desk::Act::Focus(mcf_desk::Caret::Temperature));
    desk.typing().push_str("warm");
    assert_eq!(desk.settle(), Err("warm"));
    desk.report_prompt();
    assert!(
        matches!(desk.doing, mcf_desk::Doing::Nothing),
        "not a temperature is not run as no temperature"
    );
    desk.temperature = "0.7".to_owned();
    assert_eq!(
        desk.settle(),
        Ok(Some(mcf_core::configuration::Thousandths(700)))
    );
    assert!(
        desk.typed.ends_with("Roll for initiative."),
        "the document was not typed into"
    );
    let _looked = drawn(&desk, DAY, "prompt-choices");
}

/// A document is drawn as its lines, and a long one shows its tail.
#[test]
fn a_long_prompt_is_drawn_as_lines_and_the_tail_is_what_shows() {
    let mut short = four_models();
    short.page = Page::Prompt;
    short.chosen = Some(0);
    short.typed = "One line.".to_owned();
    let mut long = four_models();
    long.page = Page::Prompt;
    long.chosen = Some(0);
    long.typed = (0..40)
        .map(|at| {
            format!("Paragraph {at} of the persona, which says something the model is to do.")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let ground = DAY.ground;
    let one = drawn(&short, DAY, "prompt-one-line").inked(ground);
    let many = drawn(&long, DAY, "prompt-many-lines").inked(ground);
    assert!(
        many > one,
        "forty lines put no more on the glass than one: {many} against {one}"
    );
}
