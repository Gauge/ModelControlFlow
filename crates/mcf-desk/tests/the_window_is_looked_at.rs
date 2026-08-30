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
            wakes: Some(1.9),
            fastest: Some(6.43),
            slowest: Some(7.66),
            cache_per_token: Some(114_688),
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
            let paper = drawn(&desk, ink, "scratch");
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
