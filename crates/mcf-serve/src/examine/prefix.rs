//! Prefix reuse: what a conversation pays for its history every turn
//! (B-494, D52).
//!
//! Every other request MCF makes turns the engine's prompt cache off, so
//! that a trial measures nothing the previous trial left behind (§3.12).
//! A chat is the opposite case: every turn shares its prefix with the
//! last, and an engine that keeps what it read answers the next turn
//! after reading only what is new. Three timings on one prompt say how
//! much that is worth here.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, as_ms, filler, median, ppm, timed, whole};
use crate::generation::Draw;
use crate::served::{Extras, Prompt};

/// The measurement's name.
pub const NAME: &str = "prefix-reuse";

/// How deep the shared prefix is, in identifiers.
const DEPTH: usize = 1024;

/// How many identifiers the next turn adds.
const TAIL: usize = 8;

/// How many times each timing is taken.
const REPEATS: usize = 3;

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&site.startup()) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let prefix = filler(DEPTH);
    let mut longer = prefix.clone();
    longer.extend(std::iter::repeat_n(2_usize, TAIL));
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let ask = |ids: &[usize], cached: bool| {
        engine.complete_with(
            Prompt::Identifiers(ids),
            1,
            Draw::greedy(0),
            true,
            &Extras {
                cached,
                ..Extras::default()
            },
            site.timed(),
        )
    };
    let mut off = Vec::with_capacity(REPEATS);
    let mut kept = Vec::with_capacity(REPEATS);
    for _ in 0..REPEATS {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        // The cache off: the whole prompt read, and nothing kept.
        let (done, ns) = timed(|| ask(&longer, false));
        if let Err(failure) = done {
            return Found::could_not_tell(&said(failure));
        }
        off.push(ns);
        // The prefix read and kept, then the longer prompt: what a next
        // turn costs when its history is already in the cache.
        if let Err(failure) = ask(&prefix, true) {
            return Found::could_not_tell(&said(failure));
        }
        let (done, ns) = timed(|| ask(&longer, true));
        if let Err(failure) = done {
            return Found::could_not_tell(&said(failure));
        }
        kept.push(ns);
    }
    let mut rows = Vec::new();
    for (repeat, (off_ns, kept_ns)) in off.iter().zip(&kept).enumerate() {
        let at = [("repeat", Value::Integer(as_integer(repeat)))];
        rows.push(Reading::new(
            &at,
            "cache_off_ns",
            i64::try_from(*off_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &at,
            "prefix_kept_ns",
            i64::try_from(*kept_ns).unwrap_or(i64::MAX),
            "ns",
        ));
    }
    let (Some(off), Some(kept)) = (median(&mut off), median(&mut kept)) else {
        return Found::could_not_tell("no timing was taken");
    };
    let saving = ppm(off.saturating_sub(kept), off);
    Found {
        lines: vec![
            format!(
                "  a prompt of {DEPTH} identifiers and {TAIL} new ones, to the first token, the \
                 median of {REPEATS}"
            ),
            format!("  cache off         {:>8} ms", as_ms(off)),
            format!("  prefix kept       {:>8} ms", as_ms(kept)),
            format!(
                "  a turn after {DEPTH} kept identifiers pays {} of what it pays reading them again",
                super::per_cent(1_000_000_i64.saturating_sub(saving))
            ),
        ],
        fields: vec![
            ("depth", Value::Integer(as_integer(DEPTH))),
            ("tail", Value::Integer(as_integer(TAIL))),
            ("cache_off_ns", whole(off)),
            ("prefix_kept_ns", whole(kept)),
            ("saving_ppm", Value::Integer(saving)),
        ],
        rows,
    }
}
