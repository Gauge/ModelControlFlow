//! Every quantization of a repository in one run: the fidelity
//! measurement over every file of the model here, with each file's size
//! and speed beside it — one table a repository (B-538, D55, B-491, D51).
//!
//! A person choosing a quantization has the files' sizes from the hub
//! and nothing else. Here every file of the repository on this machine
//! reads the same reference tokens, and each is a row set: its bytes, the
//! positions it agreed at, the bits it spent, and the tokens a second it
//! generated at. The reference is the most precise file here and reads
//! against itself, which is the nought the others are measured from.

use mcf_record::json::Value;

use super::fidelity::{
    POSITIONS, RANKED, millibits_said, precision_of, read_with, reference_words, siblings_of,
};
use super::{Found, Reading, Site, as_integer, framed_ids, gigabytes, per_second, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "repository";

/// How many tokens each file generates for its speed.
const PRODUCE: usize = 128;

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: every file of the repository, a row set each"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let siblings = siblings_of(site.model);
    let Some(reference) = siblings
        .iter()
        .filter_map(|held| precision_of(held).map(|bits| (bits, held)))
        .max_by_key(|(bits, held)| {
            (
                *bits,
                std::fs::metadata(held).map_or(0, |about| about.len()),
            )
        })
        .map(|(_, held)| held.clone())
    else {
        return Found::could_not_tell(
            "no file of this repository here says its precision in its name",
        );
    };
    if siblings.len() < 2 {
        return Found::could_not_tell(
            "only one file of this repository is here; the model's page lists what the hub \
             publishes, and each one downloaded joins this table",
        );
    }
    let reference_name = reference
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let (prompt, reference_tokens) = match reference_words(site, &reference, POSITIONS) {
        Ok(produced) => produced,
        Err(why) => {
            return Found::could_not_tell(&format!("the reference {reference_name}: {why}"));
        }
    };
    if reference_tokens.is_empty() {
        return Found::could_not_tell("the reference produced no tokens to read");
    }
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} file(s) of this repository here; each reads the {} token(s) the reference \
         {reference_name} produced, then generates {PRODUCE} for its speed",
        siblings.len(),
        reference_tokens.len()
    )];
    let mut measured = 0_usize;
    for (at, file) in siblings.iter().enumerate() {
        site.progress(at, siblings.len(), &file.display().to_string());
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let name = file
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let bytes = std::fs::metadata(file).map_or(0, |about| about.len());
        let bits = precision_of(file);
        let dims = [("file", Value::text(name.clone()))];
        rows.push(Reading::new(
            &dims,
            "bytes",
            i64::try_from(bytes).unwrap_or(i64::MAX),
            "bytes",
        ));
        if let Some(bits) = bits {
            rows.push(Reading::new(&dims, "bits", i64::from(bits), "bits"));
        }
        let read = match read_with(site, file, prompt.clone(), &reference_tokens) {
            Ok(read) => read,
            Err(why) => {
                lines.push(format!("  {name}   {}   not read: {why}", gigabytes(bytes)));
                rows.push(Reading::new(&dims, "read", 0, "bool"));
                continue;
            }
        };
        let per_token = super::bits::per(read.spent, reference_tokens.len());
        let (worst_rank, worst_at) = read.worst.unwrap_or((1, 0));
        // Its speed: a short generation on its own server.
        let speed = site
            .server_for(
                file,
                &Startup {
                    projector: None,
                    ..site.startup()
                },
            )
            .and_then(|engine| {
                let ids = framed_ids(&engine, crate::crosscheck::PROMPT)?;
                let (done, ns) = timed(|| {
                    engine.complete(
                        Prompt::Identifiers(&ids),
                        PRODUCE,
                        Draw::greedy(0),
                        false,
                        site.timed(),
                    )
                });
                let completed = done.map_err(|failure| failure.detail().to_owned())?;
                Ok(per_second(
                    u64::try_from(completed.predicted).unwrap_or(0),
                    ns,
                ))
            });
        measured = measured.saturating_add(1);
        rows.push(Reading::new(&dims, "read", 1, "bool"));
        rows.push(Reading::new(
            &dims,
            "agreed",
            as_integer(read.agreed),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "positions",
            as_integer(reference_tokens.len()),
            "count",
        ));
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
        rows.push(Reading::new(
            &dims,
            "bounded",
            as_integer(read.bounded),
            "count",
        ));
        if let Ok(rate) = speed {
            rows.push(Reading::new(
                &dims,
                "per_second",
                i64::try_from(rate).unwrap_or(i64::MAX),
                "tokens/s",
            ));
        }
        lines.push(format!(
            "  {name}\n      {}   agreed at {} of {}   {} bits a token   worst rank {} at {}   {}",
            gigabytes(bytes),
            read.agreed,
            reference_tokens.len(),
            millibits_said(per_token),
            if worst_rank > RANKED {
                format!("past {RANKED}")
            } else {
                worst_rank.to_string()
            },
            worst_at,
            speed.map_or_else(
                |why| format!("speed not measured: {why}"),
                |rate| format!("{rate} tokens/s")
            )
        ));
    }
    Found {
        lines,
        fields: vec![
            ("reference", Value::text(reference_name)),
            ("files", Value::Integer(as_integer(siblings.len()))),
            ("measured", Value::Integer(as_integer(measured))),
            (
                "positions",
                Value::Integer(as_integer(reference_tokens.len())),
            ),
        ],
        rows,
    }
}
