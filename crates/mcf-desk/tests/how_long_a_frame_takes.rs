//! How long the window takes to draw a frame.
//!
//! Ignored: it is a measurement, not a claim. Run it by name. `MCF_FRAMES` says how many
//! frames to draw, `MCF_FRAME_WIDTH` how wide.

use mcf_desk::paint::{NIGHT, Painter};
use mcf_desk::ui::Mouse;
use mcf_desk::{Desk, Hosted, Model, Page, Tally, Use};
use mcf_record::json::Value;

fn a_busy_desk(tallies: usize) -> Desk {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Hosting;
    desk.models = (0..40)
        .map(|at| Model {
            name: format!("Model-{at}-Q4_K_M"),
            path: format!("/store/owner/Model-{at}-GGUF/Model-{at}-Q4_K_M.gguf"),
            file: format!("Model-{at}-Q4_K_M.gguf"),
            repository: Some(format!("owner/Model-{at}-GGUF")),
            bytes: Some(5_020_000_000),
            ..Model::default()
        })
        .collect();
    desk.chosen = Some(0);
    desk.hosted = Some(Hosted {
        model: "/store/owner/Model-0-GGUF/Model-0-Q4_K_M.gguf".to_owned(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: "2026-09-17T20:01:23Z".to_owned(),
        context: Some(262_144),
        cache: mcf_core::configuration::CacheType::default(),
        projector: None,
        takes: None,
        api_key: true,
        open: true,
        network_address: Some("http://192.168.1.10:17817".to_owned()),
        in_use: Some(Use::from_value(&Value::map([
            ("prompted_tokens", Value::text("275698")),
            ("generated_tokens", Value::text("7844")),
            ("prompt_tokens_reused", Value::text("3648300")),
            ("engine_prompt_seconds", Value::text("781.316")),
            ("engine_generating_seconds", Value::text("198.664")),
            ("requests_served", Value::text("6")),
            ("decodes", Value::text("8011")),
            ("uptime_seconds", Value::Integer(3_352)),
        ]))),
    });
    let began = std::time::Instant::now();
    let (mut asked, mut processed, mut written) = (1_800_000_u64, 80_000_u64, 19_000_u64);
    for second in 0..tallies {
        // The engine advances these when a request finishes, not token by token, so the
        // shape is a staircase: level for a while, then a step.
        if second % 240 == 239 {
            asked += 653_999;
            processed += 45_949;
            written += 1_307;
        }
        desk.tallies.push_back(Tally {
            asked,
            processed,
            written,
            at: began
                .checked_add(std::time::Duration::from_secs(second as u64))
                .unwrap_or(began),
        });
    }
    desk
}

#[test]
#[ignore = "a measurement; run it by name"]
fn how_long_a_frame_takes() {
    let frames: usize = std::env::var("MCF_FRAMES")
        .ok()
        .and_then(|held| held.parse().ok())
        .unwrap_or(200);
    let wide: u32 = std::env::var("MCF_FRAME_WIDTH")
        .ok()
        .and_then(|held| held.parse().ok())
        .unwrap_or(1400);
    for (what, tallies) in [("a young hold", 60), ("an hour of readings", 3_600)] {
        for page in [Page::Hosting, Page::Host, Page::Downloads] {
            let mut desk = a_busy_desk(tallies);
            desk.page = page;
            let mut paint = match Painter::on_paper(wide, 1024, 1.0, NIGHT) {
                Ok(paint) => paint,
                Err(why) => {
                    println!("skipped: {why}");
                    return;
                }
            };
            // One frame first, so what is measured is drawing rather than whatever a
            // first frame sets up.
            let _warm = mcf_desk::view::draw(&mut paint, &desk, &Mouse::default());
            let at = std::time::Instant::now();
            for _frame in 0..frames {
                let _going = mcf_desk::view::draw(&mut paint, &desk, &Mouse::default());
            }
            #[expect(clippy::cast_precision_loss, reason = "a count of frames, for a rate")]
            let over = frames as f64;
            let each = at.elapsed().as_secs_f64() / over;
            println!(
                "{what:20} {:>10} {:7.2} ms a frame  ({:5.0} frames a second)",
                format!("{page:?}"),
                each * 1e3,
                1.0 / each
            );
        }
    }
}
