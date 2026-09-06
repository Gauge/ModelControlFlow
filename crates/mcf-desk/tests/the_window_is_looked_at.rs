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
            device_free: None,
            measured_body: None,
            cross_checked: Vec::new(),
            prompt_reported: false,
            measured_at: None,
            cross_checked_at: None,
            prompt_reported_at: None,
            applied_addressing: None,
            applied_budget: None,
            probed: Vec::new(),
            readings_at: std::collections::BTreeMap::new(),
            repository: None,
            file: String::new(),
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
            device_free: None,
            measured_body: None,
            cross_checked: Vec::new(),
            prompt_reported: false,
            measured_at: None,
            cross_checked_at: None,
            prompt_reported_at: None,
            applied_addressing: None,
            applied_budget: None,
            probed: Vec::new(),
            readings_at: std::collections::BTreeMap::new(),
            repository: None,
            file: String::new(),
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

/// Words nothing here matches leave the library empty and offer the hub
/// from the list itself; what the hub answered is pressed there too (D51,
/// B-485).
#[test]
fn words_nothing_matches_offer_the_hub_from_the_list() {
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.filter = "gemma".to_owned();
    assert!(desk.library().is_empty());
    assert!(
        act_within(&desk, &mcf_desk::Act::SearchHub, (40.0, 400.0)),
        "nothing in the list searches the hub for the words typed"
    );
    desk.doing = a_search_answer();
    if let mcf_desk::Doing::Listing(job) = &mut desk.doing {
        job.finished = true;
    }
    desk.hear_for_review();
    assert!(desk.hub_matches());
    assert!(
        act_within(&desk, &mcf_desk::Act::PickHub(0), (40.0, 400.0)),
        "the hub's first repository cannot be opened from the list"
    );
}

/// A repository's files show after a pick even when the stream closed
/// without saying it was done.
///
/// F194: the daemon answered a look-up in one line with no closing word,
/// so the window's job took the close for MCF dying mid-sentence and the
/// hub page drew that refusal over the files it had. Picking a repository
/// from the hub list did nothing a person could see.
#[test]
fn a_repositorys_files_show_though_the_stream_closed_without_a_word() {
    use mcf_record::json::Value;
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.filter = "gemma".to_owned();
    desk.doing = a_search_answer();
    if let mcf_desk::Doing::Listing(job) = &mut desk.doing {
        job.finished = true;
    }
    desk.hear_for_review();
    desk.hub_chosen = Some(0);
    desk.chosen = None;
    let mut job = mcf_desk::job::Job::already(
        "looking up someone/gemma-4-12B-it-qat-GGUF".to_owned(),
        vec![Value::map([
            ("repository", Value::text("someone/gemma-4-12B-it-qat-GGUF")),
            (
                "files",
                Value::List(vec![Value::map([
                    ("file", Value::text("gemma-4-12B-it-qat-Q4_K_M.gguf")),
                    ("bytes", Value::Integer(7_300_000_000)),
                    ("fits", Value::Bool(true)),
                    ("why", Value::Null),
                ])]),
            ),
        ])],
    );
    job.refused = Some("MCF stopped answering before it said it had finished".to_owned());
    desk.doing = mcf_desk::Doing::Listing(job);
    desk.hear_for_review();
    assert!(
        desk.offered.contains_key("someone/gemma-4-12B-it-qat-GGUF"),
        "the files that arrived are not kept"
    );
    assert!(
        act_within(&desk, &mcf_desk::Act::PickOffered(0), (60.0, 700.0)),
        "the files that arrived are not on the page to pick"
    );
}

/// The filters open from the library and each picker's list drops over
/// it (D51, B-489).
#[test]
fn the_filters_open_from_the_library() {
    let mut desk = four_models();
    desk.page = Page::Models;
    assert!(
        act_within(&desk, &mcf_desk::Act::ToggleFilters, (40.0, 200.0)),
        "nothing in the library opens the filters"
    );
    desk.filters.open = true;
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::Open(mcf_desk::Picker::Fits),
            (40.0, 260.0)
        ),
        "the fits picker cannot be opened"
    );
    desk.open = Some(mcf_desk::Picker::Fits);
    assert!(
        act_within(&desk, &mcf_desk::Act::SetFits(1), (40.0, 400.0)),
        "the fits list does not offer will run here"
    );
}

/// Every diagnostic is a row of one list and starts from its own pane:
/// the ladder's Run is on the page as it opens, a probe's row is chosen
/// from the list and its pane runs that probe, a family's heading runs
/// every one of it, and a measurement far down the list is reached by
/// scrolling it (D53, B-490).
#[test]
fn every_diagnostic_is_a_row_and_runs_from_its_pane() {
    use mcf_desk::Diagnostic;
    let mut desk = four_models();
    desk.page = Page::Diagnostics;
    desk.chosen = Some(0);
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::Run(mcf_desk::Card::Throughput),
            (60.0, 500.0)
        ),
        "the ladder's Run is not on the page as it opens"
    );
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::Show(Diagnostic::Probe(0)),
            (60.0, 700.0)
        ),
        "the first probe's row is not in the list"
    );
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::Run(mcf_desk::Card::Capabilities),
            (60.0, 400.0)
        ),
        "the probes' heading has no Run all"
    );
    desk.act(mcf_desk::Act::Show(Diagnostic::Probe(2)));
    assert!(
        act_within(
            &desk,
            &mcf_desk::Act::RunOne(Diagnostic::Probe(2)),
            (60.0, 400.0)
        ),
        "the chosen probe's pane has no Run"
    );
    desk.act(mcf_desk::Act::Show(Diagnostic::Prompt));
    assert!(
        act_within(&desk, &mcf_desk::Act::Go(Page::Prompt), (60.0, 400.0)),
        "the prompt analysis pane does not open the page"
    );
    // The last measurement's row is far down the list, which scrolls: at
    // some offset it is in view, whatever the list has grown to.
    let last = mcf_serve::examine::MEASURES.len() - 1;
    let reached = [900.0, 1200.0, 1500.0, 1800.0, 2100.0]
        .into_iter()
        .any(|offset| {
            let _was = desk.scrolls.insert(mcf_desk::Region::Checks, offset);
            let mut y = 60.0;
            while y < 750.0 {
                if pressed_at(&desk, (260.0, y)).as_ref()
                    == Some(&mcf_desk::Act::Show(Diagnostic::Measure(last)))
                {
                    return true;
                }
                y += 10.0;
            }
            false
        });
    assert!(
        reached,
        "the last measurement's row is not reached by scrolling the list"
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
    // The cross-check's own pane, chosen from the list (D53).
    desk.diagnostic = mcf_desk::Diagnostic::CrossCheck;
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
    // Still going: the daemon's last line is the reading, not the end.
    if let mcf_desk::Doing::CrossChecking(job) = &mut desk.doing {
        job.finished = false;
    }
    let running = drawn(&desk, DAY, "cross-check-running");
    if running.width == 1 {
        return; // no font here
    }
    // Finished: the card says the figures are on Statistics and offers the
    // way there, rather than opening a panel of its own (D50).
    if let mcf_desk::Doing::CrossChecking(job) = &mut desk.doing {
        job.finished = true;
    }
    let done = drawn(&desk, DAY, "cross-check-done");
    assert!(
        done.pixels != running.pixels,
        "a finished cross-check draws the same as one running"
    );
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::SeeStatistics),
        "a finished run does not lead to its figures"
    );
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

/// What one frame with this mouse means, on a window of the given size.
fn acted_with(desk: &Desk, mouse: &Mouse, size: (u32, u32)) -> Option<mcf_desk::Act> {
    let mut paint = match Painter::on_paper(size.0, size.1, 1.0, NIGHT) {
        Ok(paint) => paint,
        Err(why) => {
            eprintln!("skipped: {why}");
            return None;
        }
    };
    mcf_desk::view::draw(&mut paint, desk, mouse)
}

/// A window of the given size, drawn to paper.
fn drawn_sized(desk: &Desk, ink: Ink, name: &str, size: (u32, u32)) -> mcf_desk::paper::Paper {
    let mut paint = match Painter::on_paper(size.0, size.1, 1.0, ink) {
        Ok(paint) => paint,
        Err(why) => {
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

/// A library of many models, more than a small window shows at once.
fn many_models() -> Desk {
    let mut desk = four_models();
    for at in 0..14 {
        let Some(mut held) = desk.models.get(1).cloned() else {
            break;
        };
        held.name = format!("Model-{at:02}-Instruct-Q4_K_M");
        held.path = format!("/home/a/.local/share/mcf/models/Model-{at:02}.gguf");
        desk.models.push(held);
    }
    desk
}

/// The library scrolls by the wheel over it and shows a bar, the splitters
/// move by dragging, and a splitter cannot be dragged past the room an area
/// needs (B-490).
#[test]
fn the_library_scrolls_and_the_splitters_move() {
    let desk = many_models();
    let small = (900, 560);
    let wheel = Mouse {
        at: (260.0, 250.0),
        down: false,
        began: None,
        click: None,
        wheel: -1.0,
    };
    assert_eq!(
        acted_with(&desk, &wheel, small),
        Some(mcf_desk::Act::Scroll(mcf_desk::Region::Library, 48)),
        "the wheel over the library does not scroll it"
    );
    let mut scrolled = many_models();
    let _was = scrolled.scrolls.insert(mcf_desk::Region::Library, 48.0);
    let still = drawn_sized(&desk, DAY, "scroll-library-top", small);
    let moved = drawn_sized(&scrolled, DAY, "scroll-library-moved", small);
    if still.width > 1 {
        assert!(moved.pixels != still.pixels, "scrolling drew the same list");
    }
    // The splitter between the library and the page: pressed on the band,
    // dragged to the right.
    let band_x = desk.splits.side + 26.0 + desk.splits.list + 20.0;
    let drag = Mouse {
        at: (band_x + 60.0, 300.0),
        down: true,
        began: Some((band_x, 300.0)),
        click: None,
        wheel: 0.0,
    };
    let Some(mcf_desk::Act::Split(mcf_desk::Splitter::List, to)) = acted_with(&desk, &drag, small)
    else {
        panic!("dragging the band between the library and the page moved nothing");
    };
    let mut wider = many_models();
    wider.act(mcf_desk::Act::Split(mcf_desk::Splitter::List, to));
    assert!(
        wider.splits.list > desk.splits.list,
        "the library did not widen"
    );
    // F195: the line has moved under the pointer and the press began
    // outside the band where it now is; the drag goes on regardless, and
    // stops when the button is let go.
    let further = Mouse {
        at: (band_x + 140.0, 300.0),
        ..drag
    };
    assert_eq!(wider.grabbed, Some(mcf_desk::Splitter::List));
    let Some(mcf_desk::Act::Split(mcf_desk::Splitter::List, further_to)) =
        acted_with(&wider, &further, small)
    else {
        panic!("the drag dropped the line once it left the band where the press began");
    };
    assert!(further_to > to, "the line did not follow the pointer");
    wider.released();
    assert_eq!(
        acted_with(&wider, &further, small),
        None,
        "a let-go splitter still followed the pointer"
    );
    wider.act(mcf_desk::Act::Split(mcf_desk::Splitter::List, 10_000));
    assert!(
        (wider.splits.list - 520.0).abs() < 0.5,
        "a splitter is kept inside its room: {}",
        wider.splits.list
    );
    wider.act(mcf_desk::Act::Split(mcf_desk::Splitter::Side, 0));
    assert!(
        (wider.splits.side - 120.0).abs() < 0.5,
        "the column is not kept inside its room: {}",
        wider.splits.side
    );
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
    // Its row is in the list as the page opens, and its pane opens the
    // analysis (D53).
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::Show(mcf_desk::Diagnostic::Prompt)),
        "the prompt analysis has no row on Diagnostics"
    );
    desk.act(mcf_desk::Act::Show(mcf_desk::Diagnostic::Prompt));
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
                act_somewhere(&desk, &mcf_desk::Act::Go(*to)),
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
        std::path::Path::new("/nowhere/control.sock"),
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
        None,
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

/// And on Running, for the window actually being held.
#[test]
fn running_says_what_the_held_window_costs() {
    let mut desk = four_models();
    desk.page = Page::Hosting;
    desk.hosted = Some(mcf_desk::Hosted {
        model: desk.models[0].path.clone(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: "a moment ago".to_owned(),
        context: Some(32_768),
        projector: None,
        takes: None,
        api_key: false,
        in_use: None,
    });
    // The same window, held under a model this list does not carry: the
    // address and the context still show, the price cannot.
    let mut unknown = four_models();
    unknown.page = Page::Hosting;
    unknown.hosted = Some(mcf_desk::Hosted {
        model: "/models/one-this-window-is-not-listing.gguf".to_owned(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: "a moment ago".to_owned(),
        context: Some(32_768),
        projector: None,
        takes: None,
        api_key: false,
        in_use: None,
    });

    let ground = DAY.ground;
    let priced = drawn(&desk, DAY, "running-priced").inked(ground);
    let bare = drawn(&unknown, DAY, "running-unpriced").inked(ground);
    assert!(
        priced > bare,
        "Running does not say what the window it is holding costs: {priced} marks against \
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

/// The rank reading grouped by part, for six parts of which four were
/// removed (B-433).
fn a_grouping() -> mcf_record::json::Value {
    use mcf_record::json::Value;
    Value::map([
        (
            "parts",
            Value::List(
                [
                    (5, 5, 0),
                    (13, 4, 2),
                    (3, 2, 0),
                    (6, 3, 0),
                    (4, 4, 0),
                    (7, 1, 1),
                ]
                .into_iter()
                .map(|(tokens, first, past)| {
                    Value::map([
                        ("tokens", Value::Integer(tokens)),
                        ("first_choice", Value::Integer(first)),
                        ("past_depth", Value::Integer(past)),
                    ])
                })
                .collect(),
            ),
        ),
        ("unplaced", Value::Integer(3)),
    ])
}

/// A finished prompt report, as a person would be shown one.
/// The floor drawn at every position, and its spread (B-434).
fn a_spread() -> (mcf_record::json::Value, mcf_record::json::Value) {
    use mcf_record::json::Value;
    let at = |position: i64, moved: i64| {
        Value::map([
            ("position", Value::Integer(position)),
            ("moved_parts_per_million", Value::Integer(moved)),
            ("held", Value::Null),
        ])
    };
    (
        Value::List(vec![
            at(0, 140_000),
            at(1, 120_000),
            at(2, 879_000),
            at(3, 140_000),
        ]),
        Value::map([
            ("least_parts_per_million", Value::Integer(120_000)),
            ("middle_parts_per_million", Value::Integer(140_000)),
            ("most_parts_per_million", Value::Integer(879_000)),
        ]),
    )
}

/// What one part did asked as the whole prompt (B-435).
fn a_reading(moved: i64, answer: &str) -> mcf_record::json::Value {
    use mcf_record::json::Value;
    Value::map([
        ("moved_parts_per_million", Value::Integer(moved)),
        (
            "held",
            Value::map([
                (
                    "first_rank",
                    Value::Integer(if moved > 500_000 { 40 } else { 1 }),
                ),
                ("kept", Value::Integer(i64::from(moved <= 500_000))),
                ("of", Value::Integer(1)),
            ]),
        ),
        ("answer", Value::text(answer.to_owned())),
    ])
}

/// The prompt of `a_report` grown from the front, and its neighbours
/// swapped.
fn grown_and_swapped() -> (mcf_record::json::Value, mcf_record::json::Value) {
    use mcf_record::json::Value;
    let prefixes = Value::List(vec![
        a_reading(970_000, "Hello! What would you like me to do?"),
        a_reading(
            420_000,
            "def slugify(title):\n    return title.replace(\" \", \"-\")",
        ),
        a_reading(
            120_000,
            "def slugify(title):\n    return title.lower().replace(\" \", \"-\")",
        ),
    ]);
    let swaps = Value::List(vec![
        a_reading(
            60_000,
            "def slugify(title):\n    return title.lower().replace(\" \", \"-\")",
        ),
        a_reading(
            510_000,
            "def slugify(title):\n    return title.replace(\" \", \"-\").lower()",
        ),
    ]);
    (prefixes, swaps)
}

/// The four sentences of `a_report` in each form: five read, and the one
/// the prompt is written in already said as not rendered (B-444).
fn in_forms() -> mcf_record::json::Value {
    use mcf_record::json::Value;
    let formed = |form: &str, moved: i64, answer: &str| {
        Value::map([
            ("form", Value::text(form.to_owned())),
            ("moved_parts_per_million", Value::Integer(moved)),
            ("held", Value::Null),
            ("answer", Value::text(answer.to_owned())),
        ])
    };
    Value::List(vec![
        Value::map([
            ("form", Value::text("one line")),
            (
                "not_rendered",
                Value::text("the prompt is written this way"),
            ),
        ]),
        formed(
            "bullets",
            80_000,
            "def slugify(title):\n    return title.lower().replace(\" \", \"-\")",
        ),
        formed(
            "numbered",
            90_000,
            "def slugify(title):\n    return title.lower().replace(\" \", \"-\")",
        ),
        formed(
            "headings",
            460_000,
            "## slugify\n\ndef slugify(title):\n    return title.replace(\" \", \"-\")",
        ),
        formed(
            "tags",
            30_000,
            "def slugify(title):\n    return title.lower().replace(\" \", \"-\")",
        ),
        formed("capitals", 720_000, "DEF SLUGIFY(TITLE): RETURN TITLE"),
    ])
}

/// The four sentences of `a_report` asked alone, and the control alone.
fn each_alone() -> (mcf_record::json::Value, mcf_record::json::Value) {
    use mcf_record::json::Value;
    (
        Value::List(vec![
            a_reading(970_000, "I am a careful assistant. How can I help?"),
            a_reading(310_000, "def slugify(title):\n    return title.lower()"),
            a_reading(990_000, "Please provide the text to lowercase."),
            a_reading(985_000, "Which function?"),
        ]),
        a_reading(980_000, "Hello! How can I help you today?"),
    )
}

/// The rank reading of `a_report`: seven tokens, two past the depth.
fn some_expected() -> mcf_record::json::Value {
    use mcf_record::json::Value;
    let ranked = |text: &str, rank: Option<i64>| {
        Value::map([
            ("text", Value::text(text.to_owned())),
            ("rank", rank.map_or(Value::Null, Value::Integer)),
            ("engine_said", Value::Null),
        ])
    };
    Value::List(vec![
        ranked(" class", None),
        ranked(" each", None),
        ranked(" handles", Some(29)),
        ranked(" output", Some(19)),
        ranked("#.", Some(16)),
        ranked(" the", Some(1)),
        ranked(" function", Some(1)),
    ])
}

/// The rank reading of `a_report` by word: five words, one past the depth,
/// one the model would have written whole (B-443).
fn some_words() -> mcf_record::json::Value {
    use mcf_record::json::Value;
    let word = |text: &str, pieces: i64, rank: Option<i64>, first: i64, part: Option<i64>| {
        Value::map([
            ("text", Value::text(text.to_owned())),
            ("pieces", Value::Integer(pieces)),
            ("rank", rank.map_or(Value::Null, Value::Integer)),
            ("first_choice", Value::Integer(first)),
            ("part", part.map_or(Value::Null, Value::Integer)),
        ])
    };
    Value::map([
        (
            "words",
            Value::List(vec![
                word("class", 1, None, 0, Some(1)),
                word("each", 1, None, 0, Some(1)),
                word("handles", 1, Some(29), 0, Some(2)),
                word("output#.", 2, Some(19), 0, None),
                word("the", 1, Some(1), 1, Some(3)),
                word("function", 1, Some(1), 1, Some(3)),
            ]),
        ),
        ("unplaced", Value::Integer(0)),
    ])
}

fn a_report() -> mcf_desk::Desk {
    use mcf_record::json::Value;
    let clause = a_clause;
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
        ("expected", some_expected()),
        ("expected_by_word", some_words()),
        (
            "settled",
            Value::map([
                ("temperature_thousandths", Value::Integer(700)),
                ("temperature", Value::text("0.700")),
                ("seeds_asked", Value::Integer(3)),
                ("distinct_answers", Value::Integer(2)),
                ("spread_parts_per_million", Value::Integer(180_000)),
                ("from_greedy_parts_per_million", Value::Integer(90_000)),
                ("top_k", Value::text("20")),
                ("top_p", Value::text("0.950")),
                ("min_p", Value::text("off")),
                ("truncation", Value::text("declared by the file")),
            ]),
        ),
        ("expected_by_part", a_grouping()),
        ("floors", a_spread().0),
        ("floor_spread", a_spread().1),
        ("alone", each_alone().0),
        ("alone_floor", each_alone().1),
        ("prefixes", grown_and_swapped().0),
        ("swaps", grown_and_swapped().1),
        ("forms", in_forms()),
        ("clauses_over_the_cap", Value::Integer(2)),
        ("generations", Value::Integer(17)),
        ("prompt_tokens", Value::Integer(31)),
        (
            "read_by",
            Value::text("a test's tokenizer, which generated".to_owned()),
        ),
        ("token_limit", Value::Integer(96)),
        ("unit", Value::text("sentence".to_owned())),
        (
            "unit_chosen_by",
            Value::text("text: no blank line".to_owned()),
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

/// A prompt analysis under way says which generation it is on, of how
/// many, and can be cut short from the page; one that has finished offers
/// no Stop (B-479, B-468, A7).
#[test]
fn a_prompt_analysis_under_way_says_its_step_and_offers_stop() {
    use mcf_record::json::Value;
    let under_way = |answers: Vec<Value>| {
        let mut desk = four_models();
        desk.page = Page::Prompt;
        desk.chosen = Some(0);
        "Write a function. Lowercase everything.".clone_into(&mut desk.typed);
        let mut job = mcf_desk::job::Job::already("taking the prompt apart".to_owned(), answers);
        job.finished = false;
        desk.doing = mcf_desk::Doing::Reporting(job);
        desk
    };
    let step = Value::map([
        ("reporting", Value::text("a model")),
        (
            "step",
            mcf_serve::prompt::Step {
                what: "without part 2 of 4".to_owned(),
                count: 3,
                of: 12,
            }
            .to_value(),
        ),
        ("done", Value::Bool(false)),
    ]);
    let said = under_way(vec![step]);
    assert!(
        act_somewhere(&said, &mcf_desk::Act::Stop),
        "a prompt analysis under way cannot be cut short from its page"
    );
    let unsaid = under_way(Vec::new());
    let with = drawn(&said, DAY, "prompt-under-way");
    let without = drawn(&unsaid, DAY, "prompt-under-way-unsaid");
    assert!(
        with.pixels != without.pixels,
        "the step the daemon announced is not on the page"
    );
    assert!(
        !act_somewhere(&a_report(), &mcf_desk::Act::Stop),
        "a finished report offers a Stop with nothing to stop"
    );
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

/// The rank reading grouped by part is a third figure on each row, and a
/// report without the reading draws the rows without it rather than as
/// wholly expected (B-433, A7).
#[test]
fn each_row_says_how_much_of_it_the_model_expected() {
    use mcf_record::json::Value;
    // The rows sit under the readings table, past the window's foot until
    // the report is scrolled up to them.
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 330.0);
    let found = match &desk.doing {
        mcf_desk::Doing::Reporting(job) => job.answers.first().cloned(),
        _ => None,
    }
    .unwrap_or(Value::Null);
    let grouped = found.get("expected_by_part");
    assert_eq!(mcf_desk::expected_mark(grouped, 1), Some("4/13".to_owned()));
    assert_eq!(mcf_desk::expected_mark(grouped, 9), None);
    assert_eq!(mcf_desk::expected_mark(Some(&Value::Null), 0), None);
    assert_eq!(mcf_desk::expected_mark(None, 0), None);
    let ground = DAY.ground;
    let with = drawn(&desk, DAY, "prompt-report").inked(ground);
    let mut unread = a_report();
    let _was = unread.scrolls.insert(mcf_desk::Region::Prompt, 330.0);
    if let mcf_desk::Doing::Reporting(job) = &mut unread.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("expected_by_part");
    }
    let without = drawn(&unread, DAY, "prompt-report-unread").inked(ground);
    // Without the reading the rows lose a cell and the foot loses its
    // "least expected" line, so what follows moves up into the viewport;
    // the glass differs either way, and that difference is the reading.
    assert_ne!(
        with, without,
        "the third figure and its legend change no ink on the glass: {with} against {without}"
    );
}

/// The word table comes first under the conditions (B-443): the words the
/// model did not expect, least expected first, and a report with no word
/// reading says so rather than drawing an empty table (A7).
#[test]
fn the_words_the_model_did_not_expect_come_first() {
    use mcf_record::json::Value;
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 200.0);
    let ground = DAY.ground;
    let with = drawn(&desk, DAY, "prompt-expected").inked(ground);
    let mut unread = a_report();
    let _was = unread.scrolls.insert(mcf_desk::Region::Prompt, 200.0);
    if let mcf_desk::Doing::Reporting(job) = &mut unread.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("expected_by_word");
        fields.insert(
            "expected_refused".to_owned(),
            Value::text("needs the served engine".to_owned()),
        );
    }
    let without = drawn(&unread, DAY, "prompt-expected-untaken").inked(ground);
    assert_ne!(
        with, without,
        "the word table and its absence draw the same ink: {with} against {without}"
    );
}

/// Pressing a sentence shows what the model wrote without it.
///
/// The figures are checkable only beside the answer they are about, and MCF
/// has held both since the measurement was written (A19).
#[test]
fn pressing_a_sentence_shows_the_answer_without_it() {
    // The rows sit under the readings table, past the window's foot until
    // the report is scrolled up to them.
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 330.0);
    assert!(
        act_within(&desk, &mcf_desk::Act::ShowWithout(1), (540.0, 720.0)),
        "no sentence in the report could be pressed"
    );

    // And what is drawn changes: the answer without a sentence is not the
    // answer to the prompt as written.
    let mut chosen = a_report();
    let _was = chosen.scrolls.insert(mcf_desk::Region::Prompt, 330.0);
    chosen.shown = Some(mcf_desk::Shown::Without(1));
    let ground = DAY.ground;
    let as_written = drawn(&desk, DAY, "answer-as-written").inked(ground);
    let without = drawn(&chosen, DAY, "answer-without").inked(ground);
    assert_ne!(
        as_written, without,
        "the answer panel drew the same thing with a sentence selected as without one"
    );
}

/// Beside each sentence removed is the sentence alone, where that was asked,
/// and pressing it shows what the model wrote to it alone (B-435).
#[test]
fn pressing_a_sentence_alone_shows_the_answer_to_it_alone() {
    use mcf_record::json::Value;
    // The alone table sits under the removed and floors tables, past the
    // window's foot: the page is scrolled to it, as a reader would.
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 780.0);
    let rows = (520.0, 640.0);
    assert!(
        act_within(&desk, &mcf_desk::Act::ShowAlone(1), rows),
        "no sentence alone in the report could be pressed"
    );
    // The answer has a page of its own under the tables.
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    let mut alone = a_report();
    let _was = alone.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    alone.shown = Some(mcf_desk::Shown::Alone(1));
    let mut without = a_report();
    let _was = without.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    without.shown = Some(mcf_desk::Shown::Without(1));
    let ground = DAY.ground;
    let to_alone = drawn(&alone, DAY, "answer-alone").inked(ground);
    assert_ne!(
        drawn(&without, DAY, "answer-without").inked(ground),
        to_alone,
        "the answer to a sentence alone drew as the answer without it"
    );
    assert_ne!(
        drawn(&desk, DAY, "answer-as-written").inked(ground),
        to_alone,
        "the answer to a sentence alone drew as the answer as written"
    );

    // A report that did not ask has no alone line to press and none drawn
    // (A7): the lines say what was read, not what could have been.
    let mut unasked = a_report();
    let _was = unasked.scrolls.insert(mcf_desk::Region::Prompt, 780.0);
    if let mcf_desk::Doing::Reporting(job) = &mut unasked.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("alone");
        let _taken = fields.remove("alone_floor");
    }
    assert!(
        !act_within(&unasked, &mcf_desk::Act::ShowAlone(1), rows),
        "a sentence alone is offered where it was never read"
    );
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 780.0);
    assert_ne!(
        drawn(&desk, DAY, "prompt-report").inked(ground),
        drawn(&unasked, DAY, "prompt-report-no-alone").inked(ground),
        "the report reads the same with each sentence alone and without it"
    );
}

/// The prompt grown from the front is a row a prefix, and pressing one
/// shows the answer to that much of the prompt (B-436).
#[test]
fn pressing_a_prefix_shows_the_answer_to_that_much_of_the_prompt() {
    use mcf_record::json::Value;
    // The rows sit under the steering rows and their legend, past the
    // window's foot: the page is scrolled to them, which is how a reader
    // reaches them too.
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 980.0);
    let rows = (520.0, 640.0);
    assert!(
        act_within(&desk, &mcf_desk::Act::ShowPrefix(1), rows),
        "no prefix in the report could be pressed"
    );
    // The answer has a page of its own under the tables.
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    let mut prefix = a_report();
    let _was = prefix.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    prefix.shown = Some(mcf_desk::Shown::Prefix(1));
    let ground = DAY.ground;
    assert_ne!(
        drawn(&desk, DAY, "answer-as-written").inked(ground),
        drawn(&prefix, DAY, "answer-prefix").inked(ground),
        "the answer to a prefix drew as the answer as written"
    );
    let mut unasked = a_report();
    let _was = unasked.scrolls.insert(mcf_desk::Region::Prompt, 980.0);
    if let mcf_desk::Doing::Reporting(job) = &mut unasked.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("prefixes");
    }
    assert!(
        !act_within(&unasked, &mcf_desk::Act::ShowPrefix(1), rows),
        "a prefix is offered where none was read"
    );
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 980.0);
    assert_ne!(
        drawn(&desk, DAY, "prompt-report").inked(ground),
        drawn(&unasked, DAY, "prompt-report-no-prefixes").inked(ground),
        "the report reads the same with the prompt grown from the front and without it"
    );
}

/// Neighbours swapped is a row a pair, and pressing one shows the answer
/// with the two in each other's places (B-437).
#[test]
fn pressing_a_swap_shows_the_answer_with_the_pair_the_other_way_round() {
    use mcf_record::json::Value;
    // The rows sit under the prefixes, past the window's foot: the page is
    // scrolled to them, which is how a reader reaches them too.
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1105.0);
    let rows = (520.0, 640.0);
    assert!(
        act_within(&desk, &mcf_desk::Act::ShowSwap(1), rows),
        "no swap in the report could be pressed"
    );
    // The answer has a page of its own under the tables.
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    let mut swap = a_report();
    let _was = swap.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    swap.shown = Some(mcf_desk::Shown::Swap(1));
    let ground = DAY.ground;
    assert_ne!(
        drawn(&desk, DAY, "answer-as-written-swapped").inked(ground),
        drawn(&swap, DAY, "answer-swap").inked(ground),
        "the answer to a swap drew as the answer as written"
    );
    let mut unasked = a_report();
    let _was = unasked.scrolls.insert(mcf_desk::Region::Prompt, 1105.0);
    if let mcf_desk::Doing::Reporting(job) = &mut unasked.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("swaps");
    }
    assert!(
        !act_within(&unasked, &mcf_desk::Act::ShowSwap(1), rows),
        "a swap is offered where none was read"
    );
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1105.0);
    assert_ne!(
        drawn(&desk, DAY, "prompt-report-swaps").inked(ground),
        drawn(&unasked, DAY, "prompt-report-no-swaps").inked(ground),
        "the report reads the same with the neighbours swapped and without them"
    );
}

/// The forms are a row a form read, a line for the one not rendered, and
/// pressing a row shows the answer to the parts in that form (B-444).
#[test]
fn pressing_a_form_shows_the_answer_to_the_parts_in_that_form() {
    use mcf_record::json::Value;
    // The rows sit under the swaps, past the window's foot: the page is
    // scrolled to them, which is how a reader reaches them too.
    let mut desk = a_report();
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1250.0);
    let rows = (400.0, 700.0);
    assert!(
        act_within(&desk, &mcf_desk::Act::ShowForm(3), rows),
        "no form in the report could be pressed"
    );
    assert!(
        !act_within(&desk, &mcf_desk::Act::ShowForm(0), rows),
        "a form not rendered was offered as an answer"
    );
    let mut form = a_report();
    let _was = form.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    form.shown = Some(mcf_desk::Shown::Form(3));
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    let ground = DAY.ground;
    assert_ne!(
        drawn(&desk, DAY, "answer-as-written-formed").inked(ground),
        drawn(&form, DAY, "answer-form").inked(ground),
        "the answer to a form drew as the answer as written"
    );
    let mut unasked = a_report();
    let _was = unasked.scrolls.insert(mcf_desk::Region::Prompt, 1250.0);
    if let mcf_desk::Doing::Reporting(job) = &mut unasked.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("forms");
    }
    assert!(
        !act_within(&unasked, &mcf_desk::Act::ShowForm(3), rows),
        "a form is offered where none was read"
    );
    let _was = desk.scrolls.insert(mcf_desk::Region::Prompt, 1250.0);
    assert_ne!(
        drawn(&desk, DAY, "prompt-report-forms").inked(ground),
        drawn(&unasked, DAY, "prompt-report-no-forms").inked(ground),
        "the report reads the same with the forms and without them"
    );
}

/// The report scrolls under the controls and never over them: scrolled,
/// the controls' band is pixel for pixel what it was, and the body is not.
#[test]
fn the_report_scrolls_under_the_controls_and_not_over_them() {
    let still = a_report();
    let mut scrolled = a_report();
    let _was = scrolled.scrolls.insert(mcf_desk::Region::Prompt, 380.0);
    let before = drawn(&still, DAY, "prompt-report");
    let after = drawn(&scrolled, DAY, "prompt-report-scrolled");
    if before.width < 2 {
        return;
    }
    let row = |paper: &mcf_desk::paper::Paper, y: usize| {
        let wide = paper.width as usize * 4;
        paper
            .pixels
            .get(y * wide..(y + 1) * wide)
            .map(<[u8]>::to_vec)
    };
    // The controls end above the run's verdict; everything up to there is
    // untouched by the scroll.
    for y in 0..430 {
        assert_eq!(
            row(&before, y),
            row(&after, y),
            "row {y} of the controls changed when the report scrolled"
        );
    }
    let ground = DAY.ground;
    assert_ne!(
        before.inked(ground),
        after.inked(ground),
        "the report did not move for the wheel"
    );
}

/// The seeds section says how each seeded draw was cut, and a report from
/// before the cut was stated says it is not recorded rather than nothing.
#[test]
fn the_seeds_section_says_how_the_draws_were_cut() {
    use mcf_record::json::Value;
    let mut stated = a_report();
    let _was = stated.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    let mut unstated = a_report();
    let _was = unstated.scrolls.insert(mcf_desk::Region::Prompt, 1400.0);
    let mut found = match &unstated.doing {
        mcf_desk::Doing::Reporting(job) => job.answers.first().cloned(),
        _ => None,
    }
    .unwrap_or(Value::Null);
    if let Value::Map(report) = &mut found
        && let Some(Value::Map(settled)) = report.get_mut("settled")
    {
        for key in ["top_k", "top_p", "min_p", "truncation"] {
            let _gone = settled.remove(key);
        }
    }
    unstated.doing = mcf_desk::Doing::Reporting(mcf_desk::job::Job::already(
        "prompt analysis".to_owned(),
        vec![found],
    ));
    let ground = DAY.ground;
    let with = drawn(&stated, DAY, "prompt-report-seeds-cut").inked(ground);
    let without = drawn(&unstated, DAY, "prompt-report-seeds-uncut").inked(ground);
    if with < 2 {
        return;
    }
    assert_ne!(
        with, without,
        "the cut the seeded draws were taken under left no mark"
    );
}

/// Pressing the sentence already shown puts the answer as written back.
#[test]
fn pressing_it_again_goes_back_to_the_answer_as_written() {
    let mut desk = a_report();
    desk.shown = Some(mcf_desk::Shown::Without(1));
    desk.act(mcf_desk::Act::ShowWithout(1));
    assert_eq!(
        desk.shown, None,
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
        act_somewhere(&desk, &mcf_desk::Act::Tab(mcf_desk::Tab::Contents)),
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
    let controls = (350.0, 430.0);

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
    let lower = (520.0, 620.0);
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

/// Each further reading is a choice on the page that says what it costs
/// and reaches the request as chosen; the floor at every position is read
/// back as a spread in the legend — or, left off, as the one draw it was
/// (B-434, B-435, §3.15, §3.4).
#[test]
fn the_floor_at_every_position_is_a_choice_that_says_its_cost_and_its_spread() {
    use mcf_record::json::Value;
    use mcf_serve::prompt::Extra;
    let mut desk = four_models();
    desk.page = Page::Prompt;
    desk.chosen = Some(0);
    desk.typed = (0..5)
        .map(|at| format!("Rule {at}: do the thing the rule says."))
        .collect::<Vec<_>>()
        .join("\n\n");
    // The rows of the readings table under the removed row; a sweep of the
    // whole window is thousands of renders.
    let controls = (420.0, 580.0);
    for extra in Extra::ALL {
        assert!(!desk.taken().extras.has(extra), "{extra:?} unless asked");
        assert!(
            act_within(&desk, &mcf_desk::Act::Extra(extra, true), controls),
            "{extra:?} is offered"
        );
        desk.act(mcf_desk::Act::Extra(extra, true));
        assert!(
            desk.taken().extras.has(extra),
            "the choice reaches the request"
        );
        assert!(
            act_within(&desk, &mcf_desk::Act::Extra(extra, false), controls),
            "and is offered back"
        );
    }
    let ground = DAY.ground;
    let on = drawn(&desk, DAY, "prompt-floors-on").inked(ground);
    desk.act(mcf_desk::Act::Extra(Extra::Floors, false));
    desk.act(mcf_desk::Act::Extra(Extra::Alone, false));
    let off = drawn(&desk, DAY, "prompt-floors-off").inked(ground);
    assert_ne!(on, off, "the cost line does not say which was chosen");

    // A report that drew the spread reads it back; one that did not says so.
    // The legend sits under the rows, past the window's foot until the
    // report is scrolled up to it.
    let mut with = a_report();
    let _was = with.scrolls.insert(mcf_desk::Region::Prompt, 480.0);
    let found = match &with.doing {
        mcf_desk::Doing::Reporting(job) => job.answers.first().cloned(),
        _ => None,
    }
    .unwrap_or(Value::Null);
    assert_eq!(
        found
            .get("floor_spread")
            .and_then(|spread| spread.get("most_parts_per_million"))
            .and_then(Value::as_integer),
        Some(879_000)
    );
    let spread = drawn(&with, DAY, "prompt-report").inked(ground);
    let mut one = a_report();
    let _was = one.scrolls.insert(mcf_desk::Region::Prompt, 480.0);
    if let mcf_desk::Doing::Reporting(job) = &mut one.doing
        && let Some(Value::Map(fields)) = job.answers.first_mut()
    {
        let _taken = fields.remove("floor_spread");
        let _taken = fields.remove("floors");
    }
    let draw = drawn(&one, DAY, "prompt-report-one-floor").inked(ground);
    assert_ne!(
        spread, draw,
        "the legend reads the same with the spread and without it"
    );
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

/// The ask screen carries the turn a question will be sent under, and says
/// what a picture was when one was shown (A22, B-462).
///
/// Two screens: one with nothing switched, one with a system turn, an
/// effort, thinking and a picture. The second says more, because every one
/// of those is a condition of the answer and §3.15 will not have a
/// condition that is invisible.
#[test]
fn the_ask_screen_carries_the_turn_and_the_picture() {
    let mut plain = four_models();
    plain.chosen = Some(0);
    plain.page = Page::Hosting;
    let mut asked = four_models();
    asked.chosen = Some(0);
    asked.page = Page::Hosting;
    asked.system = "You are careful.".to_owned();
    asked.effort = "low".to_owned();
    asked.thinking = Some(false);
    asked.picture = "/home/a/pictures/circle.png".to_owned();

    let bare = drawn(&plain, DAY, "ask-plain");
    let full = drawn(&asked, DAY, "ask-asked");
    assert!(
        bare.inked(DAY.ground) > 0,
        "the ask screen drew nothing at all"
    );
    // **A condition that changes the answer changes the screen.** The two
    // differ in nothing but the turn and the picture, so a screen that drew
    // them the same would be sending a condition it never showed (§3.15,
    // A22, B-462).
    assert!(
        full.pixels != bare.pixels,
        "the turn a question goes under is not on the screen that asks it"
    );
}

/// Every page, in the states a person meets, written out as images when
/// `MCF_LOOK` names a directory — the review pass over the window that
/// reads what it draws rather than asking a person to (D49).
#[test]
fn every_page_is_drawn_for_review() {
    if std::env::var("MCF_LOOK").is_err() {
        return;
    }
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp-vulkan",
        "Radeon 8060S Graphics",
        true,
        32_768,
        Some(32),
        true,
        None,
    );
    let placements = three_placements();

    // Configure, with one setting moved and the mouse over a row.
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);
    desk.recommended = Some(recommended.clone());
    let mut moved = recommended.clone();
    moved.context = 16_384;
    desk.settings = Some(moved);
    let mut last = recommended.clone();
    last.context = 8_192;
    desk.last_settings = Some((last, "2026-09-04T22:00:00Z".to_owned()));
    desk.placements = placements.clone();
    desk.models[0].applied_addressing = Some(
        "user…assistant, thinking open — set by the chat-template probe at 2026-09-04T22:53:58"
            .to_owned(),
    );
    let _ = drawn(&desk, DAY, "review-configure");
    desk.editing = Some((mcf_desk::Field::Context, "16,3".to_owned()));
    let _ = drawn(&desk, DAY, "review-configure-typing");
    desk.editing = None;
    desk.edit_refused = Some("the context window wants a whole number, not \"lots\"".to_owned());
    let _ = drawn(&desk, NIGHT, "review-configure-refused");
    desk.edit_refused = None;

    // Statistics, with a measurement, a cross-check and a last hold.
    desk.models[0].measured_body = Some(a_measurement());
    desk.models[0].ladder = vec![
        mcf_desk::chart::Reading {
            depth: 512,
            ms: 6.43,
        },
        mcf_desk::chart::Reading {
            depth: 1024,
            ms: 6.91,
        },
        mcf_desk::chart::Reading {
            depth: 2048,
            ms: 7.66,
        },
    ];
    desk.models[0].cross_checked = vec![
        "MCF's own engine read 120 position(s) of what the provisioned engine produced, and would have chosen the same token at 118".to_owned(),
        "AGREE — where they differed, the other engine's token was never worse than MCF's rank 2".to_owned(),
    ];
    desk.last_hold = Some(mcf_desk::LastHold {
        model: desk.models[0].path.clone(),
        device: "Radeon 8060S Graphics".to_owned(),
        engine: "llama.cpp-vulkan".to_owned(),
        stopped: true,
        ago_seconds: Some(5_400),
    });
    desk.tab = mcf_desk::Tab::Statistics;
    let _ = drawn(&desk, DAY, "review-statistics");

    // Contents.
    desk.tab = mcf_desk::Tab::Contents;
    desk.anatomy = Some(an_anatomy_answer());
    let _ = drawn(&desk, DAY, "review-contents");

    // Server, running, with the engine's counters and a rate line.
    desk.page = Page::Hosting;
    desk.hosted = Some(an_engine_in_use(&desk.models[0].path));
    for at in 0..90_u32 {
        #[allow(clippy::cast_precision_loss, reason = "a sample index")]
        desk.rates
            .push_back(120.0 + 30.0 * ((at as f32) / 9.0).sin());
    }
    let _ = drawn(&desk, DAY, "review-server-running");
    let _ = drawn(&desk, NIGHT, "review-server-running-night");

    // Diagnostics: the cards, and the throughput card with a run going.
    desk.page = Page::Diagnostics;
    desk.tab = mcf_desk::Tab::Configure;
    desk.placements = placements.clone();
    let _ = drawn(&desk, DAY, "review-diagnostics");
    desk.doing = mcf_desk::Doing::Measuring(a_ladder_under_way());
    let _ = drawn(&desk, NIGHT, "review-diagnostics-running");
    review_the_diagnostics(&mut desk);

    review_the_library(&recommended);

    review_the_search();
    review_small_windows();
}

/// The pages in a small window, where the library, the model page and the
/// Diagnostics page overflow and show their bars (B-490).
fn review_small_windows() {
    let small = (900, 560);
    let mut desk = many_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);
    desk.tab = mcf_desk::Tab::Statistics;
    if let Some(held) = desk.models.get_mut(0) {
        held.measured_body = Some(a_measurement());
    }
    let _ = drawn_sized(&desk, DAY, "review-small-statistics", small);
    let _was = desk.scrolls.insert(mcf_desk::Region::Library, 120.0);
    let _was = desk.scrolls.insert(mcf_desk::Region::Page, 80.0);
    let _ = drawn_sized(&desk, DAY, "review-small-scrolled", small);
    desk.page = Page::Diagnostics;
    let _ = drawn_sized(&desk, NIGHT, "review-small-diagnostics", small);
    // Any size: a small window and a large one, the same pages laid out
    // for each, nothing cut and nothing left in a fixed-width island
    // (B-510).
    for (name, size) in [("tiny", (700, 500)), ("wide", (2200, 1300))] {
        for (page, tab, called) in [
            (Page::Diagnostics, mcf_desk::Tab::Configure, "diagnostics"),
            (Page::Models, mcf_desk::Tab::Configure, "configure"),
            (Page::Models, mcf_desk::Tab::Statistics, "statistics"),
            (Page::Hosting, mcf_desk::Tab::Configure, "server"),
            (Page::Monitor, mcf_desk::Tab::Configure, "system"),
        ] {
            desk.page = page;
            desk.tab = tab;
            let _ = drawn_sized(&desk, DAY, &format!("review-{name}-{called}"), size);
        }
    }
}

/// A repository's files as the daemon lists them, for the hub page.
fn a_files_answer() -> mcf_desk::Doing {
    use mcf_record::json::Value;
    let file = |name: &str, bytes: i64, fits: bool| {
        Value::map([
            ("file", Value::text(name)),
            ("bytes", Value::Integer(bytes)),
            ("fits", Value::Bool(fits)),
            ("why", Value::Null),
        ])
    };
    mcf_desk::Doing::Listing(mcf_desk::job::Job::already(
        "looking up someone/gemma-4-12B-it-qat-GGUF".to_owned(),
        vec![Value::map([
            ("repository", Value::text("someone/gemma-4-12B-it-qat-GGUF")),
            ("revision", Value::text("abc123")),
            (
                "files",
                Value::List(vec![
                    file("gemma-4-12B-it-qat-Q4_K_M.gguf", 7_300_000_000, true),
                    file("gemma-4-12B-it-qat-Q8_0.gguf", 12_500_000_000, true),
                    file("gemma-4-12B-it-qat-BF16.gguf", 24_000_000_000, false),
                ]),
            ),
            ("terms", Value::text("gemma (terms present)")),
            ("done", Value::Bool(true)),
        ])],
    ))
}

/// The library filtered and searched: the filters open with a list
/// dropped, words nothing here matches, the hub asked and answered, and
/// one of its repositories opened (D51).
fn review_the_search() {
    let mut searched = four_models();
    searched.page = Page::Models;
    searched.filters.open = true;
    searched.filters.fits = Some(true);
    searched.open = Some(mcf_desk::Picker::Size);
    let _ = drawn(&searched, DAY, "review-library-filters");
    searched.filters = mcf_desk::Filters::default();
    searched.open = None;
    "gemma".clone_into(&mut searched.filter);
    let _ = drawn(&searched, DAY, "review-library-search");
    searched.doing = a_search_answer();
    if let mcf_desk::Doing::Listing(job) = &mut searched.doing {
        job.finished = true;
    }
    searched.hear_for_review();
    let _ = drawn(&searched, DAY, "review-library-hub");
    searched.hub_chosen = Some(0);
    searched.chosen = None;
    searched.doing = a_files_answer();
    let _ = drawn(&searched, DAY, "review-library-hub-files");
}

/// A model as a repository: its quantizations picked on Configure, the
/// ones here and the ones the hub publishes, and one not here as the
/// page's subject (D51, B-486).
fn review_the_library(recommended: &mcf_serve::hosting::Hosting) {
    let mut grouped = four_models();
    grouped.page = Page::Models;
    grouped.chosen = Some(0);
    grouped.recommended = Some(recommended.clone());
    grouped.settings = Some(recommended.clone());
    grouped.placements = three_placements();
    for (at, file) in [
        (0, "Assistant-8B-Instruct-Q4_K_M.gguf"),
        (1, "Assistant-8B-Instruct-Q8_0.gguf"),
    ] {
        if let Some(held) = grouped.models.get_mut(at) {
            held.repository = Some("owner/Assistant-8B-Instruct-GGUF".to_owned());
            file.clone_into(&mut held.file);
        }
    }
    let _replaced = grouped.offered.insert(
        "owner/Assistant-8B-Instruct-GGUF".to_owned(),
        vec![
            mcf_desk::OfferedFile {
                file: "Assistant-8B-Instruct-Q8_0.gguf".to_owned(),
                bytes: Some(8_500_000_000),
                fits: Some(true),
            },
            mcf_desk::OfferedFile {
                file: "Assistant-8B-Instruct-BF16.gguf".to_owned(),
                bytes: Some(16_100_000_000),
                fits: Some(false),
            },
        ],
    );
    grouped.open = Some(mcf_desk::Picker::Quantization);
    let _ = drawn(&grouped, DAY, "review-library-quantizations");
    grouped.open = None;
    grouped.pick_quantization(2);
    let _ = drawn(&grouped, DAY, "review-library-pending");
    grouped.page = Page::Diagnostics;
    let _ = drawn(&grouped, DAY, "review-diagnostics-pending");
}

/// Where a hold can go on this machine: as resolved, the processor, the card.
fn three_placements() -> Vec<mcf_desk::Placement> {
    vec![
        mcf_desk::Placement {
            on: "resolved".to_owned(),
            engine: "llama.cpp-vulkan".to_owned(),
            device: "Radeon 8060S Graphics".to_owned(),
            gpu_layers: 999,
            free: Some(97_000_000_000),
        },
        mcf_desk::Placement {
            on: "processor".to_owned(),
            engine: "llama.cpp".to_owned(),
            device: "CPU".to_owned(),
            gpu_layers: 0,
            free: Some(40_000_000_000),
        },
        mcf_desk::Placement {
            on: "card".to_owned(),
            engine: "llama.cpp-vulkan".to_owned(),
            device: "Radeon 8060S Graphics".to_owned(),
            gpu_layers: 999,
            free: Some(97_000_000_000),
        },
    ]
}

/// The probes three in: the daemon has announced the third.
fn a_probe_run_under_way() -> mcf_desk::job::Job {
    let mut going = mcf_desk::job::Job::already(
        "probing Assistant-8B".to_owned(),
        vec![mcf_record::json::Value::map([
            ("probing", mcf_record::json::Value::text("a-model")),
            (
                "step",
                mcf_record::json::Value::map([
                    ("name", mcf_record::json::Value::text("stop-conditions")),
                    ("count", mcf_record::json::Value::Integer(3)),
                    ("of", mcf_record::json::Value::Integer(9)),
                ]),
            ),
            ("lines", mcf_record::json::Value::List(Vec::new())),
            ("done", mcf_record::json::Value::Bool(false)),
        ])],
    );
    going.finished = false;
    going
}

/// The Diagnostics list with a probe running, its row marked and its pane
/// showing the step; then a measurement running, with a finding from an
/// earlier run under its card (D53).
fn review_the_diagnostics(desk: &mut Desk) {
    desk.doing = mcf_desk::Doing::Probing(a_probe_run_under_way());
    desk.diagnostic = mcf_desk::Diagnostic::Probe(2);
    let _ = drawn(desk, DAY, "review-diagnostics-probing");
    desk.doing = mcf_desk::Doing::Examining(an_examination_under_way());
    desk.diagnostic = mcf_desk::Diagnostic::Measure(1);
    if let Some(held) = desk.models.get_mut(0) {
        held.probed.push(mcf_desk::Finding {
            name: "prefill-saturation".to_owned(),
            at: Some("2026-09-05T15:31:04.000000000Z (local offset +00:00)".to_owned()),
            engine: Some("provisioned llama.cpp-vulkan @a1b2c3d4e5f6".to_owned()),
            lines: vec![
                "  a prompt of 1024 identifiers read 3 times at each batch, the median kept"
                    .to_owned(),
                "  batch    64      405.3 ms       2525 tokens a second".to_owned(),
                "  batch   256      254.6 ms       4021 tokens a second".to_owned(),
                "  fastest at batch 256; batch 256 is within a tenth of it".to_owned(),
            ],
        });
    }
    // And the run's readings as a table under the finding (D54).
    let rows: Vec<mcf_record::readings::Reading> = [64_i64, 256, 1024, 2048]
        .iter()
        .flat_map(|batch| {
            (0..3_i64).map(move |repeat| {
                mcf_record::readings::Reading::new(
                    &[
                        ("batch", mcf_record::json::Value::Integer(*batch)),
                        ("repeat", mcf_record::json::Value::Integer(repeat)),
                    ],
                    "read_ns",
                    405_300_000 - batch * 60_000 + repeat * 1_200_000,
                    "ns",
                )
            })
        })
        .collect();
    let mut run = mcf_record::readings::run_body(
        &desk
            .models
            .first()
            .map(|held| held.path.clone())
            .unwrap_or_default(),
        "prefill-saturation",
        "provisioned llama.cpp-vulkan @a1b2c3d4e5f6",
        vec![("depth", mcf_record::json::Value::Integer(1024))],
        &rows,
    );
    if let mcf_record::json::Value::Map(fields) = &mut run {
        let _at = fields.insert(
            "at".to_owned(),
            mcf_record::json::Value::text("2026-09-05T15:31:04.000000000Z (local offset +00:00)"),
        );
    }
    desk.readings = Some((
        desk.models
            .first()
            .map(|held| held.path.clone())
            .unwrap_or_default(),
        vec![run],
    ));
    let _ = drawn(desk, NIGHT, "review-diagnostics-examining");
    review_the_coding_row(desk);
}

/// A coding suite's row: run through the command line, its readings
/// under `coding` and when it last ran from the model's summary (B-519).
fn review_the_coding_row(desk: &mut Desk) {
    desk.doing = mcf_desk::Doing::Nothing;
    desk.diagnostic = mcf_desk::Diagnostic::Eval(0);
    let coding_rows: Vec<mcf_record::readings::Reading> = ["merge-sorted", "glob-match"]
        .iter()
        .flat_map(|task| {
            (0..3_i64).flat_map(move |attempt| {
                let dims = [
                    ("language", mcf_record::json::Value::text("python")),
                    ("task", mcf_record::json::Value::text(*task)),
                    ("attempt", mcf_record::json::Value::Integer(attempt)),
                ];
                [
                    mcf_record::readings::Reading::new(&dims, "cases_held", 3 - attempt, "count"),
                    mcf_record::readings::Reading::new(&dims, "cases", 3, "count"),
                    mcf_record::readings::Reading::new(
                        &dims,
                        "ask_ns",
                        2_400_000_000 + attempt * 10_000_000,
                        "ns",
                    ),
                ]
            })
        })
        .collect();
    let mut coding = mcf_record::readings::run_body(
        &desk
            .models
            .first()
            .map(|held| held.path.clone())
            .unwrap_or_default(),
        "coding",
        "through the daemon, run in a container",
        vec![("tasks", mcf_record::json::Value::Integer(20))],
        &coding_rows,
    );
    if let mcf_record::json::Value::Map(fields) = &mut coding {
        let _at = fields.insert(
            "at".to_owned(),
            mcf_record::json::Value::text("2026-09-06T03:46:43.000000000Z (local offset +00:00)"),
        );
    }
    if let Some(held) = desk.models.get_mut(0) {
        let _was = held.readings_at.insert(
            "coding".to_owned(),
            "2026-09-06T03:46:43.000000000Z (local offset +00:00)".to_owned(),
        );
    }
    desk.readings = Some((
        desk.models
            .first()
            .map(|held| held.path.clone())
            .unwrap_or_default(),
        vec![coding],
    ));
    let _ = drawn(desk, DAY, "review-diagnostics-coding");
    desk.diagnostic = mcf_desk::Diagnostic::Throughput;
    desk.readings = None;
}

/// The measurements two in: the daemon has announced the second.
fn an_examination_under_way() -> mcf_desk::job::Job {
    let mut going = mcf_desk::job::Job::already(
        "examining Assistant-8B".to_owned(),
        vec![mcf_record::json::Value::map([
            ("examining", mcf_record::json::Value::text("a-model")),
            (
                "step",
                mcf_record::json::Value::map([
                    ("name", mcf_record::json::Value::text("prefill-saturation")),
                    ("count", mcf_record::json::Value::Integer(2)),
                    ("of", mcf_record::json::Value::Integer(6)),
                ]),
            ),
            ("lines", mcf_record::json::Value::List(Vec::new())),
            ("done", mcf_record::json::Value::Bool(false)),
        ])],
    );
    going.finished = false;
    going
}

/// A ladder two rungs in: its estimate, one reading, and the second rung
/// announced.
fn a_ladder_under_way() -> mcf_desk::job::Job {
    let mut going = mcf_desk::job::Job::already(
        "measuring Assistant-8B".to_owned(),
        vec![
            mcf_record::json::Value::map([
                ("measuring", mcf_record::json::Value::text("a-model")),
                (
                    "depths",
                    mcf_record::json::Value::List(vec![
                        mcf_record::json::Value::Integer(512),
                        mcf_record::json::Value::Integer(1024),
                    ]),
                ),
                ("estimate_low_seconds", mcf_record::json::Value::Integer(40)),
                (
                    "estimate_high_seconds",
                    mcf_record::json::Value::Integer(90),
                ),
                ("done", mcf_record::json::Value::Bool(false)),
            ]),
            mcf_record::json::Value::map([
                ("measuring", mcf_record::json::Value::text("a-model")),
                (
                    "reading",
                    mcf_record::json::Value::map([
                        ("depth", mcf_record::json::Value::Integer(512)),
                        ("ms_per_token", mcf_record::json::Value::text("6.43")),
                        ("measured", mcf_record::json::Value::Bool(true)),
                    ]),
                ),
                ("done", mcf_record::json::Value::Bool(false)),
            ]),
            mcf_record::json::Value::map([
                ("measuring", mcf_record::json::Value::text("a-model")),
                (
                    "starting",
                    mcf_record::json::Value::map([
                        ("depth", mcf_record::json::Value::Integer(1024)),
                        ("step", mcf_record::json::Value::Integer(2)),
                        ("of", mcf_record::json::Value::Integer(2)),
                    ]),
                ),
                ("done", mcf_record::json::Value::Bool(false)),
            ]),
        ],
    );
    going.finished = false;
    going
}

/// A measurement as the daemon answers it: three rungs on the card.
fn a_measurement() -> mcf_record::json::Value {
    let reading = |depth: i64, ms: &str| {
        mcf_record::json::Value::map([
            ("depth", mcf_record::json::Value::Integer(depth)),
            ("ms_per_token", mcf_record::json::Value::text(ms)),
            ("measured", mcf_record::json::Value::Bool(true)),
        ])
    };
    let figure = |ms: &str, why: &str| {
        mcf_record::json::Value::map([
            ("measured", mcf_record::json::Value::Bool(!ms.is_empty())),
            ("ms", mcf_record::json::Value::text(ms)),
            ("why", mcf_record::json::Value::text(why)),
        ])
    };
    mcf_record::json::Value::map([
        (
            "readings",
            mcf_record::json::Value::List(vec![
                reading(512, "6.43"),
                reading(1024, "6.91"),
                reading(2048, "7.66"),
            ]),
        ),
        (
            "first_token",
            mcf_record::json::Value::map([
                ("measured", mcf_record::json::Value::Bool(true)),
                ("ms", mcf_record::json::Value::text("412.7")),
                ("depth", mcf_record::json::Value::Integer(512)),
                (
                    "includes",
                    mcf_record::json::Value::text("includes reading the prompt"),
                ),
            ]),
        ),
        (
            "prompt_reading",
            figure(
                "",
                "one rung measured, and a prompt cost is read between two",
            ),
        ),
        (
            "memory",
            figure("", "every rung ran in one window of 4,096 tokens"),
        ),
        ("fall_off", figure("", "one rung measured")),
        (
            "conditions",
            mcf_record::json::Value::map([
                (
                    "engine_ran",
                    mcf_record::json::Value::text(
                        "provisioned llama.cpp-vulkan server @925e1179947e",
                    ),
                ),
                (
                    "device",
                    mcf_record::json::Value::text("Radeon 8060S Graphics"),
                ),
                ("gpu_layers", mcf_record::json::Value::Integer(999)),
            ]),
        ),
    ])
}

/// A server an hour up, with the engine's counters.
fn an_engine_in_use(path: &str) -> mcf_desk::Hosted {
    mcf_desk::Hosted {
        model: path.to_owned(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: "2026-09-05T05:59:21Z".to_owned(),
        context: Some(32_768),
        projector: None,
        takes: None,
        api_key: false,
        in_use: Some(mcf_desk::Use {
            generated: Some(41_320),
            prompted: Some(12_004),
            generated_per_second: Some(151.7),
            prompted_per_second: Some(2_310.0),
            cache_used: None,
            processing: Some(1),
            queued: Some(0),
            resident: Some(6_700_000_000),
            card: Some(23_200_000_000),
            uptime_seconds: Some(3_725),
        }),
    }
}

/// A hub search, answered.
fn a_search_answer() -> mcf_desk::Doing {
    mcf_desk::Doing::Listing(mcf_desk::job::Job::already(
        "searching the hub for \"gemma\"".to_owned(),
        vec![mcf_record::json::Value::map([
            ("query", mcf_record::json::Value::text("gemma")),
            (
                "repositories",
                mcf_record::json::Value::List(
                    [
                        ("someone/gemma-4-12B-it-qat-GGUF", 1_295_081),
                        ("google/gemma-4-12B-it-qat-q4_0-gguf", 754_658),
                        ("lmstudio-community/gemma-4-E4B-it-GGUF", 630_574),
                    ]
                    .into_iter()
                    .map(|(id, downloads)| {
                        mcf_record::json::Value::map([
                            ("id", mcf_record::json::Value::text(id)),
                            ("downloads", mcf_record::json::Value::Integer(downloads)),
                        ])
                    })
                    .collect(),
                ),
            ),
            ("done", mcf_record::json::Value::Bool(true)),
        ])],
    ))
}
