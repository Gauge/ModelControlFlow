//! Quantization fidelity: a file of a repository read against the
//! highest-precision file of the same repository here (B-491, D52).
//!
//! **What decides a quantization had no figure.** The library lists
//! every file a repository publishes with its size and whether it fits;
//! what a smaller file gives up was nowhere. Here the reference —
//! the most precise sibling on this machine — generates greedily from a
//! fixed prompt, and the file under measurement is made to read those
//! tokens and say, at every position, where it ranks the reference's
//! choice and how likely it found it. That is the cross-check's method
//! turned on two files of one model rather than two engines on one file
//! (B-362): texts cannot be compared once two generations part, and
//! ranks under teacher forcing can.
//!
//! **Two figures, both counts.** How many positions the file would have
//! chosen the same token at, and the bits it spent on the reference's
//! tokens — nought would be the reference itself. Neither says the file
//! is *worse*; a reader with the ordering can decide what a bit a token
//! is worth to them.

use std::path::{Path, PathBuf};

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, gigabytes, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "quantization-fidelity";

/// How many tokens the reference produces, and so how many positions are
/// read.
pub const POSITIONS: usize = 96;

/// How many ranked candidates the file is asked for at each position; a
/// reference token past them is bounded rather than lost.
pub const RANKED: usize = 64;

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: what it started, what it read, the rows it kept"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let siblings = siblings_of(site.model);
    let Some(reference) = reference_among(site.model, &siblings) else {
        let others: Vec<String> = siblings
            .iter()
            .filter(|held| held.as_path() != site.model)
            .filter_map(|held| {
                held.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        return Found::could_not_tell(&if others.is_empty() {
            "no other file of this repository is here to read against; the model's page \
             lists what the hub publishes, and a higher-precision one downloaded there is the \
             reference"
                .to_owned()
        } else {
            format!(
                "no file of this repository here is more precise than this one, so this is the \
                 reference the others read against ({})",
                others.join(", ")
            )
        });
    };
    let reference_bytes = std::fs::metadata(&reference).map_or(0, |about| about.len());
    let reference_name = reference
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    // The reference generates, then goes: two servers on one card at once
    // would be a fit nobody planned.
    let (prompt, reference_tokens) = match reference_words(site, &reference) {
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
    let Read {
        agreed,
        worst,
        bounded,
        spent,
        positions: read_positions,
    } = read;
    let mut rows = Vec::new();
    for (at, (rank, millibits, past)) in read_positions.iter().enumerate() {
        let dims = [("position", Value::Integer(as_integer(at)))];
        rows.push(Reading::new(
            &dims,
            "rank",
            Value::Integer(as_integer(*rank)).as_integer().unwrap_or(0),
            "count",
        ));
        rows.push(Reading::new(&dims, "millibits", *millibits, "millibits"));
        rows.push(Reading::new(&dims, "past_ranked", i64::from(*past), "bool"));
    }
    let positions = reference_tokens.len();
    let per_token = super::bits::per(spent, positions);
    let (worst_rank, worst_at) = worst.unwrap_or((1, 0));
    Found {
        lines: vec![
            format!(
                "  reference {reference_name} ({}); this file {}",
                gigabytes(reference_bytes),
                gigabytes(std::fs::metadata(site.model).map_or(0, |about| about.len()))
            ),
            format!(
                "  the reference produced {positions} token(s) from \"{}\"; this file read every one",
                crate::crosscheck::PROMPT
            ),
            format!(
                "  agreed at {agreed} of {positions} position(s) ({}); worst rank {} at position \
                 {worst_at}{}",
                super::per_cent(super::ppm(agreed as u64, positions as u64)),
                if worst_rank > RANKED {
                    format!("past {RANKED}")
                } else {
                    worst_rank.to_string()
                },
                if bounded > 0 {
                    format!(
                        "; {bounded} position(s) past the {RANKED} ranked, counted at the bound"
                    )
                } else {
                    String::new()
                }
            ),
            format!(
                "  {} bits spent a token on the reference's choices — nought would be the \
                 reference itself",
                millibits_said(per_token)
            ),
        ],
        fields: vec![
            ("reference", Value::text(reference_name)),
            ("reference_bytes", whole(reference_bytes)),
            ("prompt", Value::text(crate::crosscheck::PROMPT.to_owned())),
            ("positions", Value::Integer(as_integer(positions))),
            ("agreed", Value::Integer(as_integer(agreed))),
            ("worst_rank", Value::Integer(as_integer(worst_rank))),
            ("worst_at", Value::Integer(as_integer(worst_at))),
            ("bounded", Value::Integer(as_integer(bounded))),
            ("ranked_to", Value::Integer(as_integer(RANKED))),
            ("millibits_per_token", Value::Integer(per_token)),
        ],
        rows,
    }
}

/// What the reference said from the prompt: the prompt's identifiers and
/// the tokens it produced, greedily.
fn reference_words(site: &Site<'_>, reference: &Path) -> Result<(Vec<usize>, Vec<usize>), String> {
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let engine = site.server_for(
        reference,
        &Startup {
            projector: None,
            ..site.startup()
        },
    )?;
    let prompt: Vec<usize> = engine
        .tokenize(crate::crosscheck::PROMPT, true, false)
        .map_err(said)?
        .into_iter()
        .map(|token| token.id)
        .collect();
    let completed = engine
        .complete(
            Prompt::Identifiers(&prompt),
            POSITIONS,
            Draw::greedy(0),
            false,
            site.waiting,
        )
        .map_err(said)?;
    Ok((prompt, completed.words().to_vec()))
}

/// What the file under measurement made of the reference's tokens.
#[derive(Debug, Default)]
struct Read {
    /// Positions where its first choice was the reference's.
    agreed: usize,
    /// The worst rank it gave a reference token, and where.
    worst: Option<(usize, usize)>,
    /// Positions where the reference's token was past what was ranked.
    bounded: usize,
    /// Millibits spent on the reference's tokens, in all.
    spent: i64,
    /// Every position: the rank given the reference's token, the millibits
    /// spent on it, and whether it was past what was ranked (D16).
    positions: Vec<(usize, i64, bool)>,
}

/// Reads the reference's tokens with the file under measurement, position
/// by position.
fn read_against(
    site: &Site<'_>,
    prompt: Vec<usize>,
    reference_tokens: &[usize],
) -> Result<Read, String> {
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let engine = site.server(&Startup {
        projector: None,
        ..site.startup()
    })?;
    let mut read = Read::default();
    let mut prefix = prompt;
    for (at, wanted) in reference_tokens.iter().enumerate() {
        if site.asker_gone() {
            return Err(crate::served::CLIENT_LEFT.to_owned());
        }
        let ranked = engine.distribution_at(&prefix, RANKED).map_err(said)?;
        if let Some(place) = ranked.iter().position(|(id, _)| id == wanted) {
            let rank = place.saturating_add(1);
            if rank == 1 {
                read.agreed = read.agreed.saturating_add(1);
            }
            if read.worst.is_none_or(|(held, _)| rank > held) {
                read.worst = Some((rank, at));
            }
            let millibits = ranked.get(place).map_or(0, |(_, mb)| -mb);
            read.spent = read.spent.saturating_add(millibits);
            read.positions.push((rank, millibits, false));
        } else {
            read.bounded = read.bounded.saturating_add(1);
            if read.worst.is_none_or(|(held, _)| RANKED < held) {
                read.worst = Some((RANKED.saturating_add(1), at));
            }
            let millibits = ranked.last().map_or(0, |(_, mb)| -mb);
            read.spent = read.spent.saturating_add(millibits);
            read.positions
                .push((RANKED.saturating_add(1), millibits, true));
        }
        prefix.push(*wanted);
    }
    Ok(read)
}

/// Millibits as bits to three places.
pub(crate) fn millibits_said(millibits: i64) -> String {
    #[expect(clippy::integer_division, reason = "whole bits and thousandths")]
    let (whole, part) = (millibits / 1_000, (millibits % 1_000).abs());
    format!("{whole}.{part:03}")
}

/// The GGUF files in the model's directory that are models of it: not a
/// projector, and of a sharded file only its first part.
pub(crate) fn siblings_of(model: &Path) -> Vec<PathBuf> {
    let Some(directory) = model.parent() else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path
                .file_name()
                .map(|held| held.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let is_gguf = path
                .extension()
                .is_some_and(|held| held.eq_ignore_ascii_case("gguf"));
            let a_later_shard = name.contains("-of-") && !name.contains("00001-of-");
            is_gguf && !name.starts_with("mmproj") && !a_later_shard
        })
        .collect();
    found.sort();
    found
}

/// The most precise sibling that is more precise than the model itself.
pub(crate) fn reference_among(model: &Path, siblings: &[PathBuf]) -> Option<PathBuf> {
    let own = precision_of(model)?;
    siblings
        .iter()
        .filter(|held| held.as_path() != model)
        .filter_map(|held| precision_of(held).map(|bits| (bits, held)))
        .filter(|(bits, _)| *bits > own)
        .max_by_key(|(bits, held)| {
            (
                *bits,
                std::fs::metadata(held).map_or(0, |about| about.len()),
            )
        })
        .map(|(_, held)| held.clone())
}

/// Bits a weight, read off a file's name: `F32` is thirty-two, `Q4_K_M`
/// four, `IQ1_M` one. A name that says nothing is `None`.
pub(crate) fn precision_of(model: &Path) -> Option<u32> {
    let name = model.file_name()?.to_string_lossy().to_ascii_uppercase();
    name.split(['-', '_', '.', ' '])
        .filter_map(|part| match part {
            "F32" => Some(32),
            "BF16" | "F16" => Some(16),
            _ => {
                let digits = part
                    .strip_prefix("IQ")
                    .or_else(|| part.strip_prefix("TQ"))
                    .or_else(|| part.strip_prefix("Q"))?;
                digits.chars().next()?.to_digit(10)
            }
        })
        .next_back()
}
