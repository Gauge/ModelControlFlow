#![allow(clippy::panic, clippy::expect_used)]

use mcf_desk::paint::{Box, NIGHT, Painter};
use mcf_desk::ui::Mouse;
use mcf_desk::{Desk, Model, Page, Region, Tab};
use mcf_optimize::dial::{Dial, Step};
use mcf_optimize::ledger::{At, Row, Under};
use mcf_optimize::reading::{Ending, Reading};

fn model() -> Model {
    Model {
        name: "A-Model-Q4_K_M".to_owned(),
        path: "/m/A-Model-Q4_K_M.gguf".to_owned(),
        bytes: Some(16_000_000_000),
        parts: None,
        architecture: Some("an-architecture".to_owned()),
        trained: Some(131_072),
        context: Some(131_072),
        engine: Some("llama.cpp-vulkan".to_owned()),
        device: Some("a card".to_owned()),
        device_free: None,
        repository: None,
        file: String::new(),
        on_a_card: true,
        cache_per_token: Some(59_392),
        cache_elements_per_token: Some(29_696),
        refused: None,
        does_not_fit: None,
    }
}

fn desk_with(rows: usize) -> Desk {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.refusal = None;
    desk.models = vec![model()];
    desk.chosen = Some(0);
    desk.page = Page::Host;
    desk.tab = Tab::Optimize;
    for at in 0..rows {
        let spot = At {
            dial: Dial::ThinkingBudget,
            step: Step::Whole(4096),
            set: at % 8 + 1,
            repeat: u8::try_from(at.checked_div(8).unwrap_or(0) + 1).unwrap_or(1),
        };
        let reading = Reading {
            dial: spot.dial,
            step: spot.step,
            set: spot.set,
            repeat: spot.repeat,
            passed: 4,
            of: 8,
            produced: 8000,
            milliseconds: 140_000,
            ending: Ending::Answered,
            why: None,
            per_task: Vec::new(),
        };
        desk.optimizing.report.record(reading.clone());
        desk.optimizing.rows.push(Row {
            recorded: "now".to_owned(),
            under: Under::default(),
            at: spot,
            reading,
        });
    }
    desk
}

fn how_far_the_page_reaches(desk: &Desk) -> f32 {
    let mut paint = Painter::on_paper(1000, 700, 1.0, NIGHT).expect("a paper to draw on");
    let mouse = Mouse::default();
    let _act = mcf_desk::view::draw(&mut paint, desk, &mouse);
    paint.lowest()
}

#[test]
fn a_report_with_more_rows_reaches_further_down_the_page() {
    let few = how_far_the_page_reaches(&desk_with(4));
    let many = how_far_the_page_reaches(&desk_with(400));
    assert!(
        many > few,
        "a table of four hundred rows has to measure taller than one of four, or the page \
         thinks there is nothing below the fold: {few} then {many}"
    );
}

#[test]
fn a_long_report_reaches_well_past_the_window_it_is_drawn_in() {
    let reach = how_far_the_page_reaches(&desk_with(400));
    assert!(
        reach > 700.0 * 4.0,
        "four hundred rows is thousands of pixels of table, and the page must say so for \
         the scrollbar to have anywhere to go: it reached {reach}"
    );
}

#[test]
fn scrolling_down_shows_rows_the_top_of_the_page_did_not() {
    let desk = desk_with(400);
    let drawn = |offset: f32| {
        let mut held = Desk::new(std::path::PathBuf::from("/nowhere"));
        held.refusal = None;
        held.models = vec![model()];
        held.chosen = Some(0);
        held.page = Page::Host;
        held.tab = Tab::Optimize;
        held.optimizing.report = desk.optimizing.report.clone();
        held.optimizing.rows.clone_from(&desk.optimizing.rows);
        let _was = held.scrolls.insert(Region::Page, offset);
        let mut paint = Painter::on_paper(1000, 700, 1.0, NIGHT).expect("paper");
        let mouse = Mouse::default();
        let _act = mcf_desk::view::draw(&mut paint, &held, &mouse);
        paint.paper().map(|paper| paper.pixels.clone())
    };
    let top = drawn(0.0);
    let down = drawn(2000.0);
    if let (Some(top), Some(down)) = (top, down) {
        assert!(
            top != down,
            "scrolling two thousand points down the page drew exactly the same thing"
        );
    }
}

#[test]
fn the_page_still_measures_something_when_the_report_is_empty() {
    let reach = how_far_the_page_reaches(&desk_with(0));
    assert!(reach > 0.0);
}

#[test]
fn a_row_off_the_bottom_of_the_window_is_not_drawn_but_is_still_counted() {
    let mut paint = Painter::on_paper(1000, 700, 1.0, NIGHT).expect("paper");
    let area = Box::new(0.0, 0.0, 1000.0, 200.0);
    paint.clip(area);
    assert_eq!(paint.clipped(), Some(area));
    paint.reaches(9000.0);
    assert!(
        paint.lowest() >= 9000.0,
        "what a page says it reaches is what the scrollbar has to work with"
    );
    paint.unclip();
    assert_eq!(paint.clipped(), None);
}
