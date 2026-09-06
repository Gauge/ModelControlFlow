//! Stop latency: the time from a stop being raised to the engine being
//! idle again, at several depths of context in flight (B-536, D55,
//! B-459).
//!
//! A request the daemon abandons is closed at the engine, and the
//! engine takes the closed connection as a cancellation. How long it
//! takes to notice — from the stop to the moment it can answer again —
//! is a cost a person pressing Stop feels, and it may grow with the
//! context the engine was working through. A generation is started
//! under a stop flag, the flag is raised after a moment, and two clocks
//! run: to the request coming back closed, and to a one-token probe
//! answered afterwards, less what that probe takes on an idle engine.

use std::sync::atomic::{AtomicBool, Ordering};

use mcf_record::json::Value;

use super::retrieval::FILLER;
use super::{Found, Reading, Site, as_integer, framed_ids, timed, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup, Waiting};

/// The measurement's name.
pub const NAME: &str = "stop-latency";

/// The depths of prompt in flight, in tokens.
const DEPTHS: [usize; 3] = [512, 4096, 16384];

/// How long the generation runs before the stop is raised, in ms.
const RUN_MS: u64 = 1500;

/// How many tokens the generation would produce, were it not stopped.
const PRODUCE: usize = 2048;

/// The room kept for the template and the ask.
const ROOM: usize = 200;

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each depth a stopped generation, two clocks a row each"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let deepest = DEPTHS.iter().copied().max().unwrap_or(0);
    let wanted =
        u64::try_from(deepest.saturating_add(ROOM).saturating_add(PRODUCE)).unwrap_or(u64::MAX);
    let engine = match site.server(&Startup {
        projector: None,
        context: site.context.min(wanted),
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let unit = match engine.tokenize(FILLER, false, false) {
        Ok(tokens) => tokens.len().max(1),
        Err(failure) => return Found::could_not_tell(failure.detail()),
    };
    let window = usize::try_from(engine.window).unwrap_or(usize::MAX);
    let probe_ids = match framed_ids(&engine, "Say one word.") {
        Ok(ids) => ids,
        Err(why) => return Found::could_not_tell(&why),
    };
    // What the probe takes on an idle engine, so that the idle clock can
    // be read net of it.
    let probe = |waiting: Waiting<'_>| {
        engine.complete(
            Prompt::Identifiers(&probe_ids),
            1,
            Draw::greedy(0),
            false,
            waiting,
        )
    };
    let (idle_probe, idle_probe_ns) = timed(|| probe(site.timed()));
    if let Err(failure) = idle_probe {
        return Found::could_not_tell(failure.detail());
    }
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  a generation started under a stop flag and stopped after {RUN_MS} ms, at each depth \
         of prompt; the window is {window} tokens; a one-token probe on the idle engine takes \
         {} ms",
        super::as_ms(idle_probe_ns)
    )];
    let mut depths_asked = 0_usize;
    for (at, depth) in DEPTHS.into_iter().enumerate() {
        site.progress(at, DEPTHS.len(), &format!("depth {depth}"));
        if depth.saturating_add(ROOM) > window {
            lines.push(format!("  depth {depth:>6}   not asked: past the window"));
            continue;
        }
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        #[expect(
            clippy::integer_division,
            reason = "how many repeats of the filler reach the depth"
        )]
        let repeats = (depth.saturating_sub(ROOM) / unit).max(1);
        let mut text =
            String::with_capacity(FILLER.len().saturating_mul(repeats.saturating_add(1)));
        for _ in 0..repeats {
            text.push_str(FILLER);
        }
        text.push_str("\n\nContinue this account at length, in the same voice.");
        let ids = match framed_ids(&engine, &text) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let stop = AtomicBool::new(false);
        let waiting = Waiting {
            stopping: Some(&stop),
            ..Waiting::NOBODY
        };
        let (closed_ns, produced, raised_at) = std::thread::scope(|scope| {
            let running = scope.spawn(|| {
                timed(|| {
                    engine.complete(
                        Prompt::Identifiers(&ids),
                        PRODUCE,
                        Draw::greedy(0),
                        false,
                        waiting,
                    )
                })
            });
            std::thread::sleep(std::time::Duration::from_millis(RUN_MS));
            let raised_at = std::time::Instant::now();
            stop.store(true, Ordering::SeqCst);
            let produced = running
                .join()
                .ok()
                .and_then(|(done, _)| done.ok())
                .map_or(0, |completed| completed.predicted);
            let closed_ns = u64::try_from(raised_at.elapsed().as_nanos()).unwrap_or(u64::MAX);
            (closed_ns, produced, raised_at)
        });
        let (after, probe_ns) = timed(|| probe(site.timed()));
        if let Err(failure) = after {
            return Found::could_not_tell(failure.detail());
        }
        let idle_ns = u64::try_from(raised_at.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let net_ns = idle_ns.saturating_sub(idle_probe_ns);
        depths_asked = depths_asked.saturating_add(1);
        let dims = [("depth", Value::Integer(as_integer(depth)))];
        rows.push(Reading::new(
            &dims,
            "prompt_tokens",
            as_integer(ids.len()),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "produced_before_stop",
            as_integer(produced),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "closed_ns",
            i64::try_from(closed_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "idle_ns",
            i64::try_from(idle_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "idle_net_ns",
            i64::try_from(net_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "probe_after_ns",
            i64::try_from(probe_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        lines.push(format!(
            "  depth {depth:>6}   closed after {} ms, idle after {} ms ({} ms net of the probe)",
            super::as_ms(closed_ns),
            super::as_ms(idle_ns),
            super::as_ms(net_ns)
        ));
    }
    if depths_asked == 0 {
        return Found::could_not_tell("no depth fits the window");
    }
    Found {
        lines,
        fields: vec![
            ("window", whole(engine.window)),
            (
                "run_ms",
                Value::Integer(as_integer(usize::try_from(RUN_MS).unwrap_or(0))),
            ),
            ("depths", Value::Integer(as_integer(depths_asked))),
            (
                "idle_probe_ns",
                Value::Integer(i64::try_from(idle_probe_ns).unwrap_or(i64::MAX)),
            ),
        ],
        rows,
    }
}
