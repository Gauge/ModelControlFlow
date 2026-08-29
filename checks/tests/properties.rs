//! The property tier: invariants MCF claims universally, examined over
//! generated inputs (B-191, D10).
//!
//! Each test below states one thing MCF's documents claim for *every* input,
//! and tries to falsify it over [`GATING_CASES`] generated cases. The seeds are
//! fixed (see `mcf_checks::property`), so a failure here is reproducible on
//! another machine from the seed the verdict prints.
//!
//! What belongs here and what does not: a property is a claim with a
//! quantifier in it. "The spread is five values that were actually observed"
//! (A6) is one; "a torn journal reports the byte it stopped at and how much it
//! did not read" (B62) is one. "`mcf doctor` renders a heading" is not — that
//! is an example, and examples belong to the unit and functional tiers.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use mcf_checks::property::{FILESYSTEM_CASES, GATING_CASES, Rng, Verdict, check};
use mcf_checks::scratch::Scratch;

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::digest::{Sha256, sha256};
use mcf_core::measurement::{ConditionValue, Conditions, Count, Floor, Measurement, Percentile};
use mcf_core::provenance::{
    Checksum, Decay, Licence, Observation, Origin, Provenance, Repository, Revision, ToolIdentity,
    Transformation, TransformationKind,
};
use mcf_core::time::{Timestamp, UtcOffset};
use mcf_core::trial::{Series, Thinning};
use mcf_record::journal::index::{self, Built, Index};
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::{self, Value};
use mcf_record::{decode, encode};

/// Fails the test with the verdict's own account of what it found, seed
/// included.
fn assert_held(verdict: &Verdict) {
    assert!(verdict.held(), "{verdict}");
}

/// The conditions generated measurements are taken under. A6 requires them;
/// what they say is not what these properties are about.
fn conditions() -> Conditions {
    Conditions::new(
        BuildIdentity::current(),
        Floor {
            mcf_configuration: Attested::Known(ConditionValue::text("the property tier")),
            ..Floor::nothing_known()
        },
    )
}

/// A measurement over between two and forty generated samples.
fn measurement(rng: &mut Rng) -> (Measurement<Count>, Vec<u64>) {
    let extra = rng.index(39);
    let samples: Vec<u64> = (0..extra + 2)
        .map(|_| u64::try_from(rng.integer_between(0, 1_000_000)).unwrap_or(0))
        .collect();
    let first = Count(samples[0]);
    let second = Count(samples[1]);
    let rest: Vec<Count> = samples[2..].iter().copied().map(Count).collect();
    (Measurement::of(first, second, rest, conditions()), samples)
}

/// A6: the spread is five values that *were actually observed*, and they are
/// ordered. An interpolating percentile would break the first half; a sort bug
/// would break the second. Neither is visible from any single example.
#[test]
fn every_reported_statistic_is_a_sample_that_was_observed() {
    let verdict = check(GATING_CASES, |rng| {
        let (measured, samples) = measurement(rng);
        let spread = measured.spread();
        for (name, value) in [
            ("minimum", spread.minimum),
            ("p5", spread.p5),
            ("median", spread.median),
            ("p95", spread.p95),
            ("maximum", spread.maximum),
        ] {
            if !samples.contains(&value.0) {
                return Err(format!(
                    "{name} is {} which is not among the {} samples",
                    value.0,
                    samples.len()
                ));
            }
        }
        if !(spread.minimum <= spread.p5
            && spread.p5 <= spread.median
            && spread.median <= spread.p95
            && spread.p95 <= spread.maximum)
        {
            return Err(format!("the spread is not ordered: {spread:?}"));
        }
        let smallest = samples.iter().copied().min().unwrap_or(0);
        let largest = samples.iter().copied().max().unwrap_or(0);
        if spread.minimum.0 != smallest || spread.maximum.0 != largest {
            return Err(format!(
                "the extremes are {}..{} and the samples are {smallest}..{largest}",
                spread.minimum.0, spread.maximum.0
            ));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// A6 again, on the count: `n` is how many samples there were, not how many
/// the type happened to keep. The two are the same only while nothing summarizes
/// early, which is exactly what B56 forbids and what a property can watch for.
#[test]
fn the_sample_count_is_the_number_of_samples() {
    let verdict = check(GATING_CASES, |rng| {
        let (measured, samples) = measurement(rng);
        if measured.n() != samples.len() {
            return Err(format!(
                "n is {} over {} samples",
                measured.n(),
                samples.len()
            ));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// Every percentile between 0 and 100 lands on an observed sample, and the
/// function is monotone in the rank. Stated separately from the spread because
/// the spread only asks about five of the hundred and one ranks.
#[test]
fn percentiles_are_monotone_in_the_rank() {
    let verdict = check(GATING_CASES, |rng| {
        let (measured, samples) = measurement(rng);
        let mut previous = None;
        for rank in 0..=100u8 {
            let percentile = Percentile::new(rank).ok_or("a rank of 0..=100 is a percentile")?;
            let value = measured.at(percentile);
            if !samples.contains(&value.0) {
                return Err(format!("p{rank} is {} which was never observed", value.0));
            }
            if previous.is_some_and(|earlier| value < earlier) {
                return Err(format!("p{rank} is {value:?}, below the rank before it"));
            }
            previous = Some(value);
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// A generated JSON value, up to a bounded depth.
///
/// Depth is bounded rather than left to chance: an unbounded generator produces
/// a stack overflow eventually, which would be a finding about the generator
/// and not about the codec.
fn value(rng: &mut Rng, depth: u32) -> Value {
    match rng.below(if depth == 0 { 4 } else { 6 }) {
        0 => Value::Null,
        1 => Value::Bool(rng.boolean()),
        2 => Value::Integer(rng.integer_between(i64::MIN, i64::MAX)),
        3 => Value::text(rng.text(12)),
        4 => Value::List((0..rng.index(5)).map(|_| value(rng, depth - 1)).collect()),
        _ => Value::map(
            (0..rng.index(5))
                .map(|_| (rng.text(8), value(rng, depth - 1)))
                .collect::<Vec<_>>(),
        ),
    }
}

/// §7.30 makes the record's format an interface MCF keeps for ever, and a
/// codec is only that if it is exact. Written and read back is the same value —
/// for strings a uniform generator would never reach: quotes, backslashes, the
/// C0 controls, and the astral plane that travels as a surrogate pair.
#[test]
fn every_record_value_survives_the_round_trip() {
    let verdict = check(GATING_CASES, |rng| {
        let original = value(rng, 3);
        let line = original.to_line();
        match json::parse(&line) {
            Ok(read_back) if read_back == original => Ok(()),
            Ok(read_back) => Err(format!("wrote {line}, read back {read_back:?}")),
            Err(error) => Err(format!("wrote {line}, and it would not parse: {error}")),
        }
    });
    assert_held(&verdict);
}

/// B62's *one line per entry* is what makes a torn write a torn line, and a
/// torn line is what replay can bound and report. A value that encoded a raw
/// newline would silently split one entry into two, and the second would be
/// unreadable — the loss A2 forbids, arriving through the codec.
#[test]
fn an_encoded_value_is_always_one_line() {
    let verdict = check(GATING_CASES, |rng| {
        let line = value(rng, 3).to_line();
        if line.contains('\n') || line.contains('\r') {
            return Err(format!("the encoding holds a line break: {line:?}"));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// B-301 re-verifies a checksum by streaming a file, and `mcf export` digests
/// one in a single pass. If the two disagreed for any chunking, an artifact
/// would verify against itself and not against its record.
#[test]
fn a_streamed_digest_equals_a_single_pass_one() {
    let verdict = check(GATING_CASES, |rng| {
        let bytes = rng.bytes(4_000);
        let whole = sha256(&bytes);
        let mut streamed = Sha256::new();
        let mut offset = 0;
        while offset < bytes.len() {
            let chunk = rng.index(200) + 1;
            let end = (offset + chunk).min(bytes.len());
            streamed.update(&bytes[offset..end]);
            offset = end;
        }
        let streamed = streamed.finish();
        if streamed != whole {
            return Err(format!(
                "{} bytes digest to {} whole and {} streamed",
                bytes.len(),
                whole.hex(),
                streamed.hex()
            ));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// A7 is the property: what MCF did not know comes back unknown, and what it
/// knew comes back as what it wrote. A decoder that turned an unknown into the
/// word "unknown" would compare equal to a floor that had read something.
#[test]
fn a_condition_floor_survives_the_record_including_its_unknowns() {
    let verdict = check(GATING_CASES, |rng| {
        let mut floor = Floor::nothing_known();
        for (index, slot) in [
            &mut floor.hardware_state,
            &mut floor.thermal_state,
            &mut floor.driver_versions,
            &mut floor.runtime_versions,
            &mut floor.quantization,
            &mut floor.context_length,
            &mut floor.batch_shape,
            &mut floor.mcf_configuration,
            &mut floor.realized_placement,
            &mut floor.instrumentation,
        ]
        .into_iter()
        .enumerate()
        {
            *slot = match rng.below(3) {
                0 => Attested::Unknown,
                1 => Attested::Known(ConditionValue::text(rng.text(10))),
                _ => Attested::Known(ConditionValue::integer(
                    rng.integer_between(-1_000, 1_000)
                        .saturating_add(i64::try_from(index).unwrap_or(0)),
                )),
            };
        }
        let conditions = Conditions::new(BuildIdentity::current(), floor.clone());
        let encoded = encode::conditions(&conditions);
        let read_back = decode::floor(&encoded).ok_or("a floor MCF wrote is a floor it reads")?;
        if read_back != floor {
            return Err(format!("wrote {floor:?}, read back {read_back:?}"));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// A provenance of any shape survives the record (§3.6, B-029).
///
/// The unit tests write the chain §XII names as the hard case. This writes
/// chains nobody would think of: eleven links deep, unknowns in every optional
/// field, transformation kinds with no name, revisions present and absent. An
/// artifact's provenance travels with it only as far as the decoder can carry
/// it, and *as far as* is the quantifier this tier is for.
#[test]
fn a_provenance_of_any_shape_survives_the_record() {
    let verdict = check(GATING_CASES, |rng| {
        let depth = rng.index(4);
        let original = a_provenance(rng, depth);
        let written = encode::provenance(&original);
        // Through the bytes, because that is what goes to the disk: an encoder
        // and a decoder that agree in memory and disagree about JSON are two
        // halves of nothing.
        let line = written.to_line();
        let parsed =
            json::parse(&line).map_err(|error| format!("MCF wrote unreadable JSON: {error}"))?;
        let read_back = decode::provenance(&parsed).map_err(|failure| failure.to_string())?;
        if read_back != original {
            return Err(format!("wrote {original:?}, read back {read_back:?}"));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// What MCF later found upstream, appended rather than written over (D37,
/// B-331).
///
/// Every finding shape, because each carries different fields and a round trip
/// that only ever saw *unchanged* would not exercise them.
fn an_observation(rng: &mut Rng) -> Observation {
    let found = match rng.below(6) {
        0 => Decay::Unchanged,
        1 => Decay::RevisionGone {
            revision: rng.text(12),
        },
        2 => Decay::Relicensed {
            was: rng.text(10),
            now: rng.text(10),
        },
        3 => Decay::Gated { how: rng.text(8) },
        4 => Decay::Replaced {
            file: rng.text(10),
            was: rng.text(64),
            now: rng.text(64),
        },
        _ => Decay::Unreachable { said: rng.text(20) },
    };
    Observation::new(
        Timestamp::from_utc_nanos(
            i128::from(rng.integer_between(0, 2_000_000_000_000_000_000)),
            Attested::Unknown,
        ),
        found,
    )
}

/// A provenance with an arbitrary chain under it.
fn a_provenance(rng: &mut Rng, depth: usize) -> Provenance {
    let origin = match rng.below(3) {
        0 => Origin::hub(
            Repository::new(format!("owner/{}", rng.text(8))),
            if rng.boolean() {
                Some(Revision::new(rng.text(12)))
            } else {
                None
            },
        ),
        1 => Origin::LocalFile {
            path: std::path::PathBuf::from(format!("/models/{}", rng.text(8))),
        },
        _ => Origin::Unattributed,
    };
    // A link MCF never fetched is a state of its own (B-019), and a round trip
    // that only ever saw acquired artifacts would not exercise it.
    if rng.below(4) == 0 {
        let mut never_fetched = Provenance::known_of(origin);
        if depth > 0 {
            never_fetched = never_fetched.derived_from(a_provenance(rng, depth - 1));
        }
        return never_fetched;
    }
    let mut provenance = Provenance::acquired(
        origin,
        Timestamp::from_utc_nanos(
            i128::from(rng.integer_between(-1_000_000_000_000, 1_000_000_000_000)),
            match rng.below(3) {
                0 => Attested::Unknown,
                _ => Attested::Known(
                    UtcOffset::from_seconds_east(
                        i32::try_from(rng.integer_between(-50_400, 50_400)).unwrap_or(0),
                    )
                    .unwrap_or_else(|| UtcOffset::from_seconds_east(0).expect("zero is an offset")),
                ),
            },
        ),
    );
    if rng.boolean() {
        let hex: String = (0..64)
            .map(|_| char::from(b"0123456789abcdef"[rng.index(16)]))
            .collect();
        if let Some(checksum) = Checksum::sha256(&hex) {
            provenance = provenance.with_integrity(checksum);
        }
    }
    match rng.below(3) {
        0 => {}
        1 => provenance = provenance.with_licence(Licence::Stated),
        _ => provenance = provenance.with_licence(Licence::spdx(rng.text(10))),
    }
    for _ in 0..rng.index(3) {
        provenance = provenance.observed(an_observation(rng));
    }
    for _ in 0..rng.index(3) {
        let kind = match rng.below(4) {
            0 => TransformationKind::Quantization,
            1 => TransformationKind::Requantization,
            2 => TransformationKind::FormatConversion,
            _ => TransformationKind::Other(rng.text(12)),
        };
        provenance = provenance.transformed(Transformation::new(
            kind,
            if rng.boolean() {
                Attested::Known(rng.text(20))
            } else {
                Attested::Unknown
            },
            if rng.boolean() {
                Attested::Known(ToolIdentity::new(
                    rng.text(8),
                    if rng.boolean() {
                        Some(rng.text(6))
                    } else {
                        None
                    },
                ))
            } else {
                Attested::Unknown
            },
            if rng.boolean() {
                Attested::Known(Timestamp::from_utc_nanos(
                    i128::from(rng.integer_between(0, 1_000_000_000)),
                    Attested::Unknown,
                ))
            } else {
                Attested::Unknown
            },
        ));
    }
    if depth > 0 {
        provenance = provenance.derived_from(a_provenance(rng, depth - 1));
    }
    provenance
}

/// The journal is the record (D20, B62), so what a journal holds is what was
/// appended to it — for any entry, not for the handful the unit tests write.
#[test]
fn every_entry_survives_the_journal() {
    let verdict = check(FILESYSTEM_CASES, |rng| {
        let scratch = Scratch::new("property-journal");
        let count = rng.index(6) + 1;
        let entries: Vec<Entry> = (0..count)
            .map(|_which| {
                let kind = *rng.pick(&EntryKind::ALL).unwrap_or(&EntryKind::Failure);
                Entry::new(
                    kind,
                    Timestamp::from_utc_nanos(
                        i128::from(rng.integer_between(0, 2_000_000_000_000_000_000)),
                        Attested::Unknown,
                    ),
                    value(rng, 2),
                )
            })
            .collect();

        {
            let mut journal =
                Journal::open(&scratch.journal()).map_err(|failure| failure.to_string())?;
            for entry in &entries {
                journal
                    .append(entry)
                    .map_err(|failure| failure.to_string())?;
            }
        }

        let replayed = replay(&scratch.journal()).map_err(|failure| failure.to_string())?;
        if let Some(loss) = &replayed.loss {
            return Err(format!("an undamaged journal reported a loss: {loss}"));
        }
        let written: Vec<&Value> = entries.iter().map(Entry::body).collect();
        let read: Vec<&Value> = replayed.entries.iter().map(Entry::body).collect();
        if written != read {
            return Err(format!(
                "appended {} entries and replayed {}",
                written.len(),
                read.len()
            ));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// The index is a pointer and the journal is the record (D20, B-300), so for
/// any journal the index must agree with a replay about what is in it — the
/// same entries, in the same order, at offsets that read back to the same
/// bytes.
///
/// This is the invariant the whole of B-300 rests on. The unit tests write six
/// entries of two kinds; a record has whatever somebody's machine put in it,
/// and an index that agreed on those and not on these would be a faster way to
/// be wrong.
#[test]
fn the_index_agrees_with_a_replay_about_every_journal() {
    let verdict = check(FILESYSTEM_CASES, |rng| {
        let scratch = Scratch::new("property-index");
        let count = rng.index(8) + 1;
        {
            let mut journal =
                Journal::open(&scratch.journal()).map_err(|failure| failure.to_string())?;
            for _which in 0..count {
                let kind = *rng.pick(&EntryKind::ALL).unwrap_or(&EntryKind::Failure);
                journal
                    .append(&Entry::new(
                        kind,
                        Timestamp::from_utc_nanos(
                            i128::from(rng.integer_between(0, 2_000_000_000_000_000_000)),
                            Attested::Unknown,
                        ),
                        value(rng, 2),
                    ))
                    .map_err(|failure| failure.to_string())?;
            }
        }

        let replayed = replay(&scratch.journal()).map_err(|failure| failure.to_string())?;
        // Opened twice on purpose: the first builds it, the second reads what
        // the first wrote. Both must agree with the journal, and the second
        // must say it *loaded* — an index that rebuilt itself every time would
        // still be correct and would have thrown away the only reason it
        // exists, so a property that did not assert this would pass on an
        // index that was never read back at all.
        for attempt in 0..2 {
            let index = Index::over(&scratch.journal(), &index::default_path(&scratch.journal()))
                .map_err(|failure| failure.to_string())?;
            if attempt == 1 && !matches!(index.built(), Built::Loaded { .. }) {
                return Err(format!(
                    "the second open did not read what the first wrote: {}",
                    index.built()
                ));
            }
            if index.entries().len() != replayed.entries.len() {
                return Err(format!(
                    "attempt {attempt}: the index has {} entries and the journal {}",
                    index.entries().len(),
                    replayed.entries.len()
                ));
            }
            for (located, entry) in index.entries().iter().zip(&replayed.entries) {
                if located.kind() != entry.kind() {
                    return Err(format!(
                        "attempt {attempt}: the index says {} and the journal {}",
                        located.kind(),
                        entry.kind()
                    ));
                }
                let read = index.read(located).map_err(|failure| failure.to_string())?;
                if read.body() != entry.body() {
                    return Err(format!(
                        "attempt {attempt}: the offset the index gave holds a different entry"
                    ));
                }
                if read.id() != entry.id() {
                    return Err(format!(
                        "attempt {attempt}: the entry at that offset has another identifier"
                    ));
                }
            }
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// B62: a replay that cannot complete says which line, which byte, and how
/// much it did not read. The property is that those three agree with the file
/// for *every* place a crash could have torn it — which is the whole point,
/// since a crash does not choose convenient offsets.
#[test]
fn a_journal_torn_anywhere_reports_exactly_what_it_lost() {
    let verdict = check(FILESYSTEM_CASES, |rng| {
        let scratch = Scratch::new("property-torn");
        let count = rng.index(5) + 2;
        {
            let mut journal =
                Journal::open(&scratch.journal()).map_err(|failure| failure.to_string())?;
            for sequence in 0..count {
                journal
                    .append(&Entry::new(
                        EntryKind::SelfCost,
                        Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
                        Value::map([("n", Value::Integer(i64::try_from(sequence).unwrap_or(0)))]),
                    ))
                    .map_err(|failure| failure.to_string())?;
            }
        }

        let whole = std::fs::read(scratch.journal()).map_err(|error| error.to_string())?;
        let cut = rng.index(whole.len());
        std::fs::write(scratch.journal(), &whole[..cut]).map_err(|error| error.to_string())?;

        let replayed = match replay(&scratch.journal()) {
            Ok(replayed) => replayed,
            // A journal cut inside its header is refused rather than replayed,
            // which is a different honest answer and not a loss report.
            Err(_) if cut < first_line_length(&whole) => return Ok(()),
            Err(failure) => return Err(format!("cut at {cut} of {}: {failure}", whole.len())),
        };
        match replayed.loss {
            None => {
                if cut != whole.len() && !ends_at_a_line_boundary(&whole, cut) {
                    return Err(format!(
                        "cut at {cut} of {} mid-line, and the replay reported no loss",
                        whole.len()
                    ));
                }
                Ok(())
            }
            Some(loss) => {
                if loss.byte_offset + loss.bytes_unread != cut {
                    return Err(format!(
                        "cut at {cut}: the loss says byte {} plus {} unread, which is {}",
                        loss.byte_offset,
                        loss.bytes_unread,
                        loss.byte_offset + loss.bytes_unread
                    ));
                }
                if loss.line == 0 {
                    return Err("a loss names a one-based line, and named zero".to_owned());
                }
                Ok(())
            }
        }
    });
    assert_held(&verdict);
}

/// How long the header line is, including its newline.
fn first_line_length(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |index| index + 1)
}

/// Whether a cut at this offset landed on a line boundary.
fn ends_at_a_line_boundary(bytes: &[u8], cut: usize) -> bool {
    cut == 0 || bytes.get(cut - 1) == Some(&b'\n')
}

/// B-271: thinning declares what it dropped. The property is that a thinned
/// series keeps the points the factor names and says so — a series that kept a
/// different number than its factor implies would make interior detail
/// uninterpretable.
#[test]
fn thinning_keeps_what_its_factor_names() {
    let verdict = check(GATING_CASES, |rng| {
        let length = rng.index(200) + 1;
        let points: Vec<Count> = (0..length)
            .map(|index| Count(u64::try_from(index).unwrap_or(0)))
            .collect();
        let full = Series::new(points.clone(), Thinning::FULL);
        if full.kept() != length {
            return Err(format!(
                "a full-resolution series of {length} kept {}",
                full.kept()
            ));
        }
        let factor = u32::try_from(rng.index(9) + 1).unwrap_or(1);
        let thinned = full
            .thinned(factor)
            .ok_or_else(|| format!("thinning by {factor} is a factor"))?;
        let expected = length.div_ceil(usize::try_from(factor).unwrap_or(1));
        if thinned.kept() != expected {
            return Err(format!(
                "thinning {length} points by {factor} kept {} rather than {expected}",
                thinned.kept()
            ));
        }
        if thinned.thinning().factor() != factor {
            return Err(format!(
                "a series thinned by {factor} says {}",
                thinned.thinning().factor()
            ));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// A19: anything MCF reports is tested against an independently known value.
/// The calendar is MCF's own arithmetic (Hinnant's `civil_from_days`), and the
/// system's `date` is a separate implementation of the same question — so
/// agreement over generated moments is evidence and disagreement is a defect
/// here.
///
/// Skipped, loudly, where `date` is not the one this reads: a check that cannot
/// run says so rather than passing (B38).
#[test]
fn the_civil_calendar_agrees_with_an_independent_one() {
    let Some(oracle) = date_oracle() else {
        println!(
            "  not checked: `date -u -d @0 +%Y-%m-%dT%H:%M:%S` is unavailable here, \
             so this property has no independent oracle on this machine"
        );
        return;
    };
    let verdict = check(64, |rng| {
        // 1901-12-13 to 2038-01-19: the range every implementation of this
        // question agrees is representable, so a disagreement is about the
        // calendar and not about somebody's 32-bit boundary.
        let seconds = rng.integer_between(-2_100_000_000, 2_100_000_000);
        let moment =
            Timestamp::from_utc_nanos(i128::from(seconds) * 1_000_000_000, Attested::Unknown);
        let civil = moment.civil_utc();
        let mine = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            civil.year, civil.month, civil.day, civil.hour, civil.minute, civil.second
        );
        let theirs =
            oracle(seconds).ok_or_else(|| format!("`date` would not answer for {seconds}"))?;
        if mine != theirs {
            return Err(format!(
                "MCF says {mine} for {seconds}s; `date` says {theirs}"
            ));
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// The system's own answer to the same question, or `None` where there is not
/// one to ask.
fn date_oracle() -> Option<impl Fn(i64) -> Option<String>> {
    let probe = ask_date(0)?;
    if probe != "1970-01-01T00:00:00" {
        return None;
    }
    Some(|seconds: i64| ask_date(seconds))
}

fn ask_date(seconds: i64) -> Option<String> {
    let output = std::process::Command::new("date")
        .args(["-u", &format!("-d@{seconds}"), "+%Y-%m-%dT%H:%M:%S"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
}

/// A product is the same bytes however many threads compute it (B-366, D38).
///
/// **The quantifier is what makes this a property rather than an example.**
/// D38 requires bit-identical output *whatever the thread count*, over every
/// shape and every set of weights — and the failure it is against is invisible
/// by construction: a reduction split across workers gives an answer that
/// differs in the last bits and depends on how busy the machine was. Fixed
/// shapes could only ever say that these shapes are safe.
///
/// The values span six orders of magnitude on purpose. Adding a million to a
/// thousandth and then to another thousandth is not the same number as adding
/// the two thousandths first, so a generator producing values of one size would
/// make this property pass without exercising the thing it is about.
#[test]
fn a_product_is_the_same_bytes_at_every_thread_count() {
    let verdict = check(GATING_CASES, |rng| {
        // Big enough to be partitioned. The engine hands a product only as many
        // workers as its size earns (F99), so a shape below that threshold runs
        // serially and would compare the serial path against itself — a case
        // that cannot fail. The assertion below is what says these did not.
        let columns = rng.index(192) + 64;
        let wanted = mcf_standin::threads::WORTH_A_WORKER * (rng.index(6) + 2);
        let rows = wanted.div_ceil(columns) + rng.index(8);
        let matrix: Vec<f32> = (0..rows * columns).map(|_| weight(rng)).collect();
        let vector: Vec<f32> = (0..columns).map(|_| weight(rng)).collect();

        let definition = mcf_standin::ops::matmul_vec(&matrix, &vector, rows, columns);
        // Zero is a thread count a caller can ask for and is not one; the
        // partition must survive it as the definition does.
        let count = rng.index(40);
        let threads = mcf_standin::threads::Threads::stated(count);
        if count > 1 && threads.worth_starting(rows * columns) < 2 {
            return Err(format!(
                "{rows}×{columns} earns {} worker(s) at {count} thread(s), so this case \
                 compared the serial path with itself and could not have failed",
                threads.worth_starting(rows * columns)
            ));
        }
        let produced =
            mcf_standin::ops::matmul_vec_across(&matrix, &vector, rows, columns, threads);
        if definition.len() != produced.len() {
            return Err(format!(
                "{rows}×{columns} at {count} thread(s) produced {} rows against {}",
                produced.len(),
                definition.len()
            ));
        }
        for (index, (one, other)) in definition.iter().zip(produced.iter()).enumerate() {
            if one.to_bits() != other.to_bits() {
                return Err(format!(
                    "{rows}×{columns} at {count} thread(s): row {index} is {other} and the \
                     serial definition says {one}"
                ));
            }
        }
        Ok(())
    });
    assert_held(&verdict);
}

/// One weight: mixed sign, and a magnitude drawn from six orders.
///
/// Never a `NaN` and never an infinity. Both would compare equal to themselves
/// by bits and neither says anything about summation order, so admitting them
/// would spend cases on inputs the property is not about.
fn weight(rng: &mut Rng) -> f32 {
    let unit = f32::from(u16::try_from(rng.below(65_536)).unwrap_or(0));
    let signed = (unit - 32_768.0) / 32_768.0;
    let decade = match rng.below(4) {
        0 => 0.001,
        1 => 1.0,
        2 => 1_000.0,
        _ => 1_000_000.0,
    };
    signed * decade
}
