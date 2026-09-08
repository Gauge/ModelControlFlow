#![allow(clippy::panic, reason = "a test says what went wrong by failing")]

use mcf_desk::paint::{DAY, Ink, NIGHT, Painter};
use mcf_desk::ui::Mouse;
use mcf_desk::{Desk, Model, Page};

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
            repository: None,
            file: String::new(),
            on_a_card: true,
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
            device_free: None,
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

fn drawn(desk: &Desk, ink: Ink, name: &str) -> mcf_desk::paper::Paper {
    let mut paint = match Painter::on_paper(1180, 760, 1.0, ink) {
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

#[test]
fn a_refusal_does_not_look_like_anything_else() {
    let mut desk = four_models();
    desk.page = Page::Host;
    for ink in [NIGHT, DAY] {
        desk.chosen = Some(1);
        let unmeasured = drawn(&desk, ink, "scratch");
        if unmeasured.width == 1 {
            return;
        }
        desk.chosen = Some(2);
        let refused = drawn(&desk, ink, "scratch");

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

#[test]
fn every_colour_can_be_seen_against_the_one_behind_it() {
    for (name, ink) in [("night", NIGHT), ("day", DAY)] {
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

fn separation(one: (u8, u8, u8), two: (u8, u8, u8)) -> f32 {
    let lightness = |colour: (u8, u8, u8)| {
        0.299_f32.mul_add(
            f32::from(colour.0),
            0.587_f32.mul_add(f32::from(colour.1), 0.114 * f32::from(colour.2)),
        )
    };
    (lightness(one) - lightness(two)).abs()
}

#[test]
fn a_model_that_will_not_run_says_so_on_its_own_page() {
    let mut desk = four_models();
    desk.page = Page::Host;
    desk.chosen = Some(2);
    let paper = drawn(&desk, NIGHT, "refused");
    if paper.width == 1 {
        return;
    }
    assert!(
        paper.inked(NIGHT.ground) > 20_000,
        "the refused model's page is blank"
    );
}

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
    let (word, said) = desk.state_line();
    assert_eq!(word, "NOT UP");
    assert!(said.contains("not answering"), "{said}");
}

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

#[test]
fn a_six_per_cent_rise_is_not_drawn_as_a_cliff() {
    use mcf_desk::chart::{Reading, falloff};
    use mcf_desk::paint::Box;

    let spread_of = |readings: &[Reading]| -> Option<u32> {
        let mut paint = Painter::on_paper(400, 200, 1.0, NIGHT).ok()?;
        paint.begin();
        let _under = falloff(&mut paint, Box::new(20.0, 20.0, 360.0, 120.0), readings);
        let paper = paint.paper()?;
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
        return;
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

    let three = paint.measure("モデル-7B", Weight::Regular, 15.0);
    let five = paint.measure("نموذج-7B", Weight::Regular, 15.0);
    assert!(
        (three - five).abs() > 1.0,
        "a three-character name and a five-character one measure the same: {three} against {five}"
    );

    paint.begin();
    let ink = paint.ink;
    paint.say_at(16.0, 20.0, "モデル", Weight::Regular, 15.0, ink.ink);
    let Some(paper) = paint.paper() else { return };
    assert!(
        paper.inked(NIGHT.ground) > 40,
        "a name MCF cannot draw put nothing at all on the screen"
    );
}

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

fn act_somewhere(desk: &Desk, wanted: &mcf_desk::Act) -> bool {
    act_within(desk, wanted, (0.0, 750.0))
}

fn act_within(desk: &Desk, wanted: &mcf_desk::Act, band: (f32, f32)) -> bool {
    let ((), asked) = mcf_desk::ui::boxes_asked(|| {
        let Ok(mut paint) = Painter::on_paper(1180, 760, 1.0, NIGHT) else {
            return;
        };
        let _act = mcf_desk::view::draw(&mut paint, desk, &Mouse::default());
    });
    let mut seen: Vec<(i32, i32)> = Vec::new();
    for area in asked {
        let at = (area.x + area.w / 2.0, area.y + area.h / 2.0);
        #[allow(clippy::cast_possible_truncation, reason = "a pixel position")]
        let key = (at.0 as i32, at.1 as i32);
        if at.1 < band.0 || at.1 >= band.1 || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        if pressed_at(desk, at).as_ref() == Some(wanted) {
            return true;
        }
    }
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

#[test]
fn pressing_host_asks_to_host_and_does_not_silently_do_nothing() {
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);

    assert!(
        act_somewhere(&desk, &mcf_desk::Act::HostIt),
        "no control on the Models screen asks to host the chosen model"
    );

    desk.settings = None;
    desk.no_settings = None;
    desk.act(mcf_desk::Act::HostIt);
    assert!(
        desk.no_settings.is_some(),
        "hosting without settings must say why, not do nothing"
    );
}

#[test]
fn every_menu_entry_can_be_pressed_from_every_screen() {
    let mut desk = four_models();
    for (from, _) in Page::MENU {
        desk.page = *from;
        for (to, label) in Page::MENU {
            let entry = mcf_desk::view::menu_box(*to, 760.0, desk.splits.side);
            let at = (entry.x + entry.w / 2.0, entry.y + entry.h / 2.0);
            assert_eq!(
                pressed_at(&desk, at),
                Some(mcf_desk::Act::Go(*to)),
                "{label} cannot be reached from {from:?} at {at:?}"
            );
        }
    }
}

#[test]
fn a_window_holding_models_does_not_look_like_one_holding_none() {
    let mut desk = four_models();
    desk.page = Page::Models;
    let with = drawn(&desk, NIGHT, "models-held");

    desk.models.clear();
    desk.chosen = None;
    let without = drawn(&desk, NIGHT, "models-none");
    if with.width < 2 || without.width < 2 {
        return;
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

    desk.busy = true;
    let (word, said) = desk.state_line();
    assert_eq!(word, "HOLDING");
    assert!(said.contains("so far"), "{said}");
}

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
    let (twice, _) = mcf_desk::view::reserve_of(held, 65_536).expect("still priced");
    assert_eq!(twice, cache * 2);
}

#[test]
fn a_model_that_cannot_be_priced_is_not_given_a_price() {
    let mut held = four_models().models[0].clone();
    held.cache_per_token = None;
    assert!(mcf_desk::view::reserve_of(&held, 32_768).is_none());
    assert!(mcf_desk::view::reserve_line(&held, 32_768).is_none());
}

#[test]
fn the_price_of_a_window_is_shown_where_it_is_chosen() {
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

#[test]
fn what_is_in_it_is_reachable_and_drawn_as_the_daemon_said_it() {
    let mut desk = four_models();
    desk.page = Page::Models;
    desk.chosen = Some(0);
    assert!(
        act_somewhere(&desk, &mcf_desk::Act::Tab(mcf_desk::Tab::Contents)),
        "nothing on Models leads to what the model is made of"
    );

    desk.page = Page::Anatomy;
    desk.anatomy = None;
    desk.no_anatomy = Some("MCF is not answering".to_owned());
    let refused = drawn(&desk, NIGHT, "anatomy-refused");
    if refused.width == 1 {
        return;
    }
    let refused = refused.inked(NIGHT.ground);

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
        assert!(
            inked > refused + 15_000,
            "the vocabulary drew {inked} pixels against {refused} for a refusal: the \
             counting is not on the screen"
        );
    }
}

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
    assert!(
        full.pixels != bare.pixels,
        "the turn a question goes under is not on the screen that asks it"
    );
}

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
