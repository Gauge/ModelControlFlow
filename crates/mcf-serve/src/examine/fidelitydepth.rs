//! Fidelity over length: rank agreement with the reference at a hundred,
//! five hundred and a thousand tokens deep (B-537, D55, B-491).
//!
//! The fidelity measurement reads ninety-six positions. A quantization
//! that agrees with its reference for a paragraph may drift over a page:
//! every position's error is a token the next position is conditioned
//! on. The reference generates a thousand tokens once; the file reads
//! every one, and the agreement, the bits spent and the worst rank are
//! written at each depth over the positions up to it.

use mcf_record::json::Value;

use super::fidelity::{
    RANKED, millibits_said, read_against, reference_among, reference_words, siblings_of,
};
use super::{Found, Reading, Site, as_integer, per_cent, ppm};

/// The measurement's name.
pub const NAME: &str = "fidelity-over-length";

/// The depths written, in positions.
pub const DEPTHS: [usize; 3] = [100, 500, 1000];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: one long read, a row set a depth"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let siblings = siblings_of(site.model);
    let Some(reference) = reference_among(site.model, &siblings) else {
        return Found::could_not_tell(
            "no file of this repository here is more precise than this one, so there is no \
             reference to read against; the fidelity measurement says which would be",
        );
    };
    let reference_name = reference
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let deepest = DEPTHS.iter().copied().max().unwrap_or(0);
    let (prompt, reference_tokens) = match reference_words(site, &reference, deepest) {
        Ok(produced) => produced,
        Err(why) => {
            return Found::could_not_tell(&format!("the reference {reference_name}: {why}"));
        }
    };
    if reference_tokens.is_empty() {
        return Found::could_not_tell("the reference produced no tokens to read");
    }
    let read = match read_against(site, prompt, &reference_tokens) {
        Ok(read) => read,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  reference {reference_name} produced {} token(s) from \"{}\"; this file read every \
         one, and the agreement is written at each depth over the positions up to it",
        reference_tokens.len(),
        crate::crosscheck::PROMPT
    )];
    let mut fields = vec![
        ("reference", Value::text(reference_name)),
        (
            "produced",
            Value::Integer(as_integer(reference_tokens.len())),
        ),
        ("ranked_to", Value::Integer(as_integer(RANKED))),
    ];
    for depth in DEPTHS {
        let upto: Vec<&(usize, i64, bool)> = read.positions.iter().take(depth).collect();
        if upto.len() < depth {
            lines.push(format!(
                "  depth {depth:>5}   not reached: the reference stopped at {}",
                read.positions.len()
            ));
            continue;
        }
        let agreed = upto.iter().filter(|(rank, _, _)| *rank == 1).count();
        let spent: i64 = upto.iter().map(|(_, millibits, _)| *millibits).sum();
        let bounded = upto.iter().filter(|(_, _, past)| *past).count();
        let (worst_rank, worst_at) = upto
            .iter()
            .enumerate()
            .map(|(at, (rank, _, _))| (*rank, at))
            .max_by_key(|(rank, _)| *rank)
            .unwrap_or((1, 0));
        let per_token = super::bits::per(spent, depth);
        let dims = [("depth", Value::Integer(as_integer(depth)))];
        rows.push(Reading::new(&dims, "agreed", as_integer(agreed), "count"));
        rows.push(Reading::new(&dims, "positions", as_integer(depth), "count"));
        rows.push(Reading::new(&dims, "millibits", spent, "millibits"));
        rows.push(Reading::new(
            &dims,
            "millibits_per_token",
            per_token,
            "millibits",
        ));
        rows.push(Reading::new(
            &dims,
            "worst_rank",
            as_integer(worst_rank),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "worst_at",
            as_integer(worst_at),
            "count",
        ));
        rows.push(Reading::new(&dims, "bounded", as_integer(bounded), "count"));
        // The band since the last depth, so drift reads apart from the start.
        lines.push(format!(
            "  depth {depth:>5}   agreed at {agreed} ({}); {} bits a token; worst rank {} at \
             position {worst_at}",
            per_cent(ppm(
                u64::try_from(agreed).unwrap_or(0),
                u64::try_from(depth).unwrap_or(1)
            )),
            millibits_said(per_token),
            if worst_rank > RANKED {
                format!("past {RANKED}")
            } else {
                worst_rank.to_string()
            }
        ));
        fields.push((
            match depth {
                100 => "agreed_100",
                500 => "agreed_500",
                _ => "agreed_1000",
            },
            Value::Integer(as_integer(agreed)),
        ));
        fields.push((
            match depth {
                100 => "millibits_per_token_100",
                500 => "millibits_per_token_500",
                _ => "millibits_per_token_1000",
            },
            Value::Integer(per_token),
        ));
    }
    Found {
        lines,
        fields,
        rows,
    }
}
