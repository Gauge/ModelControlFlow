#![allow(clippy::panic, clippy::expect_used)]

use mcf_desk::paint::{NIGHT, Painter};
use mcf_desk::ui::Mouse;
use mcf_desk::{Desk, Model, Page, Tab};
use mcf_optimize::dial::{Dial, Step};
use mcf_optimize::ledger::{At, Row, Under};
use mcf_optimize::reading::{Ending, Reading};

fn desk_with(verdicts: bool) -> Desk {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.refusal = None;
    desk.models = vec![Model {
        name: "A-Model-Q4_K_M".to_owned(),
        path: "/m/A-Model-Q4_K_M.gguf".to_owned(),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    desk.page = Page::Host;
    desk.tab = Tab::Optimize;
    for (at, value) in [200_u32, 500, 800].into_iter().enumerate() {
        for set in mcf_optimize::corpus::Set::short().into_iter().take(4) {
            let per_task: Vec<(String, bool)> = if verdicts {
                set.tasks
                    .iter()
                    .enumerate()
                    .map(|(number, task)| (task.name.clone(), (number + at) % 3 != 0))
                    .collect()
            } else {
                Vec::new()
            };
            let reading = Reading {
                dial: Dial::Temperature,
                step: Step::Thousandths(value),
                set: set.number,
                repeat: 1,
                passed: u32::try_from(per_task.iter().filter(|(_, right)| *right).count())
                    .unwrap_or(0),
                of: 25,
                produced: 4000,
                milliseconds: 40_000,
                ending: Ending::Answered,
                why: None,
                per_task,
            };
            desk.optimizing.report.record(reading.clone());
            desk.optimizing.rows.push(Row {
                recorded: "now".to_owned(),
                under: Under::default(),
                at: At {
                    dial: reading.dial,
                    step: reading.step,
                    set: reading.set,
                    repeat: reading.repeat,
                },
                reading,
            });
        }
    }
    desk
}

fn reach(desk: &Desk, name: &str) -> f32 {
    let mut paint = Painter::on_paper(1180, 1400, 1.0, NIGHT).expect("a paper to draw on");
    let _act = mcf_desk::view::draw(&mut paint, desk, &Mouse::default());
    if let (Ok(into), Some(paper)) = (std::env::var("MCF_LOOK"), paint.paper()) {
        let _written = std::fs::write(format!("{into}/{name}.ppm"), paper.as_pixmap());
    }
    paint.lowest()
}

#[test]
fn short_questions_marked_one_by_one_are_broken_down_by_kind_below_the_chart() {
    let without = reach(&desk_with(false), "no-breakdown");
    let with = reach(&desk_with(true), "breakdown");
    assert!(
        with > without + 200.0,
        "a row a kind of question has to take room on the page: {without} then {with}"
    );
}

#[test]
fn a_speed_sweep_is_not_broken_down_by_kind() {
    let mut desk = desk_with(true);
    let marked = reach(&desk, "breakdown");
    desk.optimizing.measure = mcf_optimize::reading::Measure::Speed;
    let timed = reach(&desk, "timed");
    assert!(timed < marked - 200.0, "{timed} against {marked}");
}
