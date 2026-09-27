#![allow(clippy::panic, clippy::expect_used)]

use mcf_desk::paint::{DAY, Ink, NIGHT, Painter};
use mcf_desk::ui::Mouse;
use mcf_desk::{Desk, Model, Page, Tab};
use mcf_optimize::dial::{Dial, Step};
use mcf_optimize::running::{Seen, Verdict};

/// A desk part way through a focus sweep: the first run of the set held every step, the
/// second slipped once, the third lost its place, and the fourth is still being written.
fn part_way(scratch: &std::path::Path, second: Verdict) -> Desk {
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
    let course = mcf_optimize::course::Course::laid_out(
        mcf_optimize::ledger::Under::default(),
        mcf_optimize::hunt::Way::ByHand,
        Dial::Temperature,
        &[],
        &[],
        1,
        mcf_optimize::reading::Measure::Correctness,
    );
    let mut run = mcf_optimize::running::Running::begun(
        mcf_optimize::running::Orders {
            endpoint: mcf_optimize::trial::Endpoint::default(),
            under: mcf_optimize::ledger::Under::default(),
            dial: Dial::Temperature,
            ceiling: None,
            named: Vec::new(),
            switch: false,
            mark: true,
            at_once: 4,
            room: std::env::temp_dir(),
            ready_within: std::time::Duration::from_secs(1),
        },
        course,
        mcf_optimize::ledger::Ledger::open(scratch).expect("ledger"),
        Box::new(|_, _| Err("no".to_owned())),
        || "now".to_owned(),
    );
    let set = mcf_optimize::corpus::Set::focus().remove(0);
    run.kind = Some(set.kind);
    run.doing = Some(mcf_optimize::ledger::At {
        dial: Dial::Temperature,
        step: Step::Thousandths(700),
        set: set.number,
        repeat: 1,
    });
    let verdicts = [
        Some(Verdict::Steps {
            held: 150,
            of: 150,
            first_slip: None,
        }),
        Some(second),
        Some(Verdict::Steps {
            held: 91,
            of: 150,
            first_slip: Some("first slip at step 12: wrote 3 1 0 9 4, wanted 3 1 9 0 4".to_owned()),
        }),
        None,
    ];
    run.questions = set
        .tasks
        .iter()
        .zip(verdicts)
        .map(|(task, verdict)| Seen {
            asked: task.asked.clone(),
            sent: true,
            answer: "1: e += 3 -> a=4 b=0 c=7 d=2 e=1\n2: swap a c -> a=7 b=0 c=4 d=2 e=1"
                .to_owned(),
            verdict,
            ..Seen::default()
        })
        .collect();
    desk.optimizing.run = Some(run);
    desk.act(mcf_desk::Act::LookAt(1));
    desk
}

fn drawn(desk: &Desk, ink: Ink, name: &str) -> mcf_desk::paper::Paper {
    let mut paint = match Painter::on_paper(1400, 900, 1.0, ink) {
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
fn a_run_that_slipped_is_drawn_apart_from_one_that_held_every_step() {
    let scratch = std::env::temp_dir().join(format!("mcf-focus-look-{}.jsonl", std::process::id()));
    let slipped = part_way(
        &scratch,
        Verdict::Steps {
            held: 149,
            of: 150,
            first_slip: Some("first slip at step 23: wrote 5 2 9 5 5, wanted 5 2 9 1 5".to_owned()),
        },
    );
    let held = part_way(
        &scratch,
        Verdict::Steps {
            held: 150,
            of: 150,
            first_slip: None,
        },
    );
    for (ink, named) in [(NIGHT, "night"), (DAY, "day")] {
        let one = drawn(&slipped, ink, &format!("focus-slipped-{named}"));
        let other = drawn(&held, ink, &format!("focus-held-{named}"));
        if one.width < 2 {
            return;
        }
        let mut differ = 0_usize;
        for y in 0..one.height {
            for x in 0..one.width {
                if one.at(x, y) != other.at(x, y) {
                    differ += 1;
                }
            }
        }
        assert!(
            differ > 500,
            "a run with a slip in it looks like one without: {differ} pixels differ"
        );
    }
    let _removed = std::fs::remove_file(&scratch);
}
