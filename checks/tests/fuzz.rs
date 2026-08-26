//! The fuzz tier: the three places untrusted bytes enter MCF (B-191, D10,
//! §3.7).
//!
//! Scheduled rather than gating — `scripts/ci.sh --with-fuzz` — because it
//! examines twenty thousand cases per target by default and B38 keeps the gate
//! fast. Every test here is `#[ignore]`d for that reason and for no other.
//!
//! **The three targets are the parsers, because a parser is the only thing at
//! M0 that reads bytes MCF did not write.** The record's codec reads a line
//! that may have been damaged on the medium; the journal replay reads a file a
//! crash was part-way through writing; the zone reader reads a file the
//! platform publishes and MCF has no say over. A bundle reader is the fourth,
//! and it reads a file that arrived from somewhere else, which is the most
//! untrusted of them all.
//!
//! **What is asserted is weak on purpose.** A parser may refuse anything: the
//! invariants are that it does not panic, that what it refuses it refuses by
//! name, and that what it says about what it read is true. A stronger claim
//! would be a claim about which damaged inputs *should* parse, and nobody has
//! made one.
//!
//! **A find is reproducible from the output.** Each test prints the seed base
//! it explored from, and a falsified case prints the seed and the bytes.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::cell::Cell;

use mcf_checks::fuzz::{self, mutate, render};
use mcf_checks::property::{Rng, Verdict, check_from};
use mcf_checks::scratch::Scratch;

use mcf_core::attested::Attested;
use mcf_core::digest::Sha256;
use mcf_core::time::{Timestamp, Zone};
use mcf_hub::{http, reference};
use mcf_record::export;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::{self, Value};
use mcf_standin::gguf;

/// Announces the campaign, so that anything it finds can be reproduced.
fn announce(target: &str) -> (u64, u32) {
    let (seed, cases) = (fuzz::seed(), fuzz::cases());
    println!(
        "  {target}: {cases} cases from seed base {seed:#018x} \
         (MCF_FUZZ_SEED, MCF_FUZZ_CASES to move it)"
    );
    (seed, cases)
}

fn assert_held(verdict: &Verdict) {
    assert!(verdict.held(), "{verdict}");
}

/// What a campaign reached, printed with its result.
///
/// A fuzz run that refused every input examined only the refusal path, and
/// would report the same green as one that exercised the parser thoroughly.
/// The counts are the difference, and the assertion on `accepted` is the
/// vacuous-green guard: a target that never once accepted a damaged input is
/// reporting on a code path it did not reach (A19).
struct Reached {
    target: &'static str,
    accepted: Cell<u32>,
    refused: Cell<u32>,
}

impl Reached {
    const fn new(target: &'static str) -> Self {
        Self {
            target,
            accepted: Cell::new(0),
            refused: Cell::new(0),
        }
    }

    fn accepted(&self) {
        self.accepted.set(self.accepted.get() + 1);
    }

    fn refused(&self) {
        self.refused.set(self.refused.get() + 1);
    }

    fn report(&self) {
        println!(
            "  {}: {} damaged inputs accepted, {} refused",
            self.target,
            self.accepted.get(),
            self.refused.get()
        );
        assert!(
            self.accepted.get() > 0,
            "{} never accepted a damaged input, so this campaign examined only the \
             refusal path",
            self.target
        );
    }
}

/// A line the record itself would write, as a starting point to damage.
fn valid_line(rng: &mut Rng) -> String {
    Value::map([
        ("id", Value::text(rng.text(8))),
        ("kind", Value::text("self_cost")),
        (
            "sequence",
            Value::Integer(rng.integer_between(0, 1_000_000)),
        ),
        (
            "body",
            Value::map([
                ("text", Value::text(rng.text(20))),
                (
                    "number",
                    Value::Integer(rng.integer_between(i64::MIN, i64::MAX)),
                ),
                ("nothing", Value::Null),
                ("list", Value::List(vec![Value::Bool(true), Value::Null])),
                // A number this format does not carry, which the reader keeps
                // as written so that a document MCF did not write can be read
                // at all (F16). It is in the corpus because the round-trip
                // property below is exactly what would break if it were ever
                // re-encoded rather than kept: a record with two spellings of
                // one value is the thing §XV's promise rests on not happening.
                (
                    "foreign",
                    mcf_record::json::parse(&format!(
                        "{}.{}e-0{}",
                        rng.integer_between(0, 9),
                        rng.integer_between(0, 999_999),
                        rng.integer_between(1, 9)
                    ))
                    .unwrap_or(Value::Null),
                ),
            ]),
        ),
    ])
    .to_line()
}

/// The codec refuses or accepts, and never panics; and what it accepts, it
/// re-encodes to something that parses back to the same value.
///
/// The second half is the one worth having. A parser that accepted a damaged
/// line and produced a value the encoder then wrote differently would give the
/// record two spellings of one entry, and §XV's promise that a configuration
/// found elsewhere reproduces here rests on there being one.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_record_codec_survives_damaged_lines() {
    let (seed, cases) = announce("json::parse");
    let reached = Reached::new("json::parse");
    let verdict = check_from(seed, cases, |rng| {
        let original = valid_line(rng);
        let damaged = mutate(rng, original.as_bytes());
        let Ok(text) = std::str::from_utf8(&damaged) else {
            // Not a case: the codec's input is a line of a file MCF wrote as
            // UTF-8, and a decoder above it has already refused what is not.
            return Ok(());
        };
        let Ok(value) = json::parse(text) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();
        let rewritten = value.to_line();
        match json::parse(&rewritten) {
            Ok(again) if again == value => Ok(()),
            Ok(again) => Err(format!(
                "accepted {}, re-encoded to {rewritten}, which reads as {again:?}",
                render(&damaged)
            )),
            Err(error) => Err(format!(
                "accepted {}, re-encoded to {rewritten}, which will not parse: {error}",
                render(&damaged)
            )),
        }
    });
    reached.report();
    assert_held(&verdict);
}

/// Whatever the medium did to a journal, replay says what it read and what it
/// did not, and the two account for the file.
///
/// B62 is the claim: *where a replay cannot complete, MCF reports what was lost
/// and how much*. A file damaged anywhere is the case that claim exists for.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_journal_replay_survives_a_damaged_file() {
    let (seed, _) = announce("journal::replay");
    // Fewer cases: every one of these writes a file and reads it back, which is
    // three orders of magnitude more expensive than a case that only parses.
    let cases = fuzz::filesystem_cases();
    let reached = Reached::new("journal::replay");
    let verdict = check_from(seed, cases.max(1), |rng| {
        let scratch = Scratch::new("fuzz-journal");
        {
            let mut journal =
                Journal::open(&scratch.journal()).map_err(|failure| failure.to_string())?;
            for _entry in 0..=rng.index(4) {
                journal
                    .append(&Entry::new(
                        EntryKind::SelfCost,
                        Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
                        Value::map([("n", Value::Integer(rng.integer_between(0, 1_000)))]),
                    ))
                    .map_err(|failure| failure.to_string())?;
            }
        }

        let whole = std::fs::read(scratch.journal()).map_err(|error| error.to_string())?;
        let damaged = mutate(rng, &whole);
        std::fs::write(scratch.journal(), &damaged).map_err(|error| error.to_string())?;

        match replay(&scratch.journal()) {
            // A refusal is a classified failure carrying its context. That is
            // all A2 asks of a file MCF cannot read.
            Err(failure) => {
                reached.refused();
                if failure.detail().is_empty() {
                    return Err(format!("a refusal said nothing: {}", render(&damaged)));
                }
                Ok(())
            }
            Ok(replayed) => match replayed.loss {
                None => {
                    reached.accepted();
                    Ok(())
                }
                Some(loss) => {
                    reached.accepted();
                    if loss.byte_offset + loss.bytes_unread != damaged.len() {
                        return Err(format!(
                            "a loss says byte {} plus {} unread, and the file is {} bytes: {}",
                            loss.byte_offset,
                            loss.bytes_unread,
                            damaged.len(),
                            render(&damaged)
                        ));
                    }
                    if loss.line == 0 {
                        return Err(format!("a loss named line 0: {}", render(&damaged)));
                    }
                    Ok(())
                }
            },
        }
    });
    reached.report();
    assert_held(&verdict);
}

/// The zone file is published by the platform, and MCF has no say over it.
///
/// A7 is what is being examined: what the reader cannot read it declines to
/// approximate. So the invariant is that it either declines or returns an
/// offset that is a real one — never a panic on a file somebody's distribution
/// wrote differently, and never `+00:00` standing in for *I could not tell*.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_zone_reader_survives_a_damaged_zone_file() {
    let (seed, cases) = announce("time::Zone::parse");
    let corpus = zone_corpus();
    assert!(
        !corpus.is_empty(),
        "there is no zone file to damage on this machine, so this target examined nothing"
    );
    let reached = Reached::new("time::Zone::parse");
    let verdict = check_from(seed, cases, |rng| {
        let original = rng.pick(&corpus).ok_or("the corpus is not empty")?.clone();
        let damaged = mutate(rng, &original);
        let Some(zone) = Zone::parse(&damaged) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();
        // Every moment the reader will be asked about, and two it will not:
        // the extremes are written out rather than divided, because the
        // workspace denies integer division and a literal says what it is.
        for seconds in [
            -4_611_686_018_427_387_904_i64,
            -2_208_988_800,
            0,
            1_700_000_000,
            2_000_000_000,
            4_611_686_018_427_387_903,
        ] {
            let moment =
                Timestamp::from_utc_nanos(i128::from(seconds) * 1_000_000_000, Attested::Unknown);
            if let Some(offset) = zone.offset_at(moment) {
                // ±26 hours is the range the format itself permits; anything
                // outside it is a value nobody could have meant.
                if offset.seconds_east().abs() > 93_600 {
                    return Err(format!(
                        "a damaged zone file yielded {} seconds east at {seconds}: {}",
                        offset.seconds_east(),
                        render(&damaged)
                    ));
                }
            }
        }
        Ok(())
    });
    reached.report();
    assert_held(&verdict);
}

/// The zone files this machine publishes, as the corpus to damage.
fn zone_corpus() -> Vec<Vec<u8>> {
    let mut corpus = Vec::new();
    for path in ["/etc/localtime", "/usr/share/zoneinfo/UTC"] {
        if let Ok(bytes) = std::fs::read(path) {
            corpus.push(bytes);
        }
    }
    corpus
}

/// A reference is the *first* untrusted input MCF meets: a string a person
/// typed or a script produced, which becomes paths and network requests
/// (B-020, §3.7, B7).
///
/// What is asserted is B7's commitment exactly: every input reaches a defined
/// outcome. No panic, and nothing accepted that would escape a directory when
/// joined to a path — which is the whole reason the syntax is where a hostile
/// reference is stopped.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_reference_parser_reaches_an_outcome_for_every_string() {
    let (seed, cases) = announce("reference::parse");
    let reached = Reached::new("reference::parse");
    let corpus: Vec<Vec<u8>> = [
        "owner/name",
        "owner/name@main:file.gguf",
        "https://huggingface.co/owner/name/resolve/main/model.gguf",
        "hf.co/owner/name",
    ]
    .iter()
    .map(|written| written.as_bytes().to_vec())
    .collect();

    let verdict = check_from(seed, cases, |rng| {
        let original = rng.pick(&corpus).ok_or("the corpus is not empty")?.clone();
        let damaged = mutate(rng, &original);
        let Ok(written) = std::str::from_utf8(&damaged) else {
            // Not a case: a reference arrives as text, and what is not text was
            // refused before it got here.
            return Ok(());
        };
        let Ok(parsed) = reference::parse(written) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();

        // Nothing accepted may escape a directory, because everything accepted
        // is about to be joined to one.
        for piece in [
            parsed.owner.as_str(),
            parsed.name.as_str(),
            parsed.revision.as_deref().unwrap_or("x"),
        ] {
            if piece.is_empty() || piece.contains('/') || piece.contains('\\') || piece == ".." {
                return Err(format!(
                    "accepted {written:?}, whose {piece:?} would escape a directory"
                ));
            }
        }
        if let Some(file) = parsed.file.as_deref()
            && (file.starts_with('/')
                || file.split('/').any(|part| part == ".." || part.is_empty()))
        {
            return Err(format!("accepted {written:?} naming the file {file:?}"));
        }

        // And what was accepted renders back to itself, so a reference in a
        // record is one somebody can type again (C5's habit).
        let rendered = parsed.to_string();
        match reference::parse(&rendered) {
            Ok(again) if again == parsed => Ok(()),
            Ok(again) => Err(format!(
                "{written:?} read as {parsed}, rendered as {rendered}, and read back as {again}"
            )),
            Err(failure) => Err(format!(
                "{written:?} read as {parsed}, rendered as {rendered}, which will not read: {failure}"
            )),
        }
    });
    reached.report();
    assert_held(&verdict);
}

/// A response is the first thing a hostile source can say to MCF, and it is
/// read before anything about it is known (B-322, §3.7).
///
/// The reference parser is fed strings a person typed; this is fed bytes a
/// network sent, which is a different threat: the header block has no length
/// prefix, the status line has no delimiter of its own, and every number in it
/// is a decimal somebody else chose. What is asserted is what a client owes —
/// no panic, a classified outcome for every input, and no claim read out of a
/// response that the response did not make.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_response_reader_survives_anything_a_source_sends() {
    let (seed, cases) = announce("http::Response::read");
    let reached = Reached::new("http::Response::read");
    let corpus: Vec<Vec<u8>> = [
        "HTTP/1.1 200 OK\r\nContent-Length: 8941\r\n\r\n{}",
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-15/396705472\r\n\r\n0123",
        "HTTP/1.1 302 Found\r\nLocation: https://us.aws.cdn.hf.co/xet-bridge-us/abc\r\n\r\n",
        "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Bearer\r\n\r\n",
    ]
    .iter()
    .map(|written| written.as_bytes().to_vec())
    .collect();

    let from = http::Url::parse("https://huggingface.co/owner/model/resolve/main/model.gguf")
        .expect("a URL");

    let verdict = check_from(seed, cases, |rng| {
        let original = rng.pick(&corpus).ok_or("the corpus is not empty")?.clone();
        let damaged = mutate(rng, &original);
        let Ok((response, consumed)) = http::Response::read(&damaged) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();

        // Whatever was accepted, the head ends inside what arrived: a caller
        // uses this to find the body, and a count past the end would have it
        // read somebody else's memory or panic.
        if consumed > damaged.len() {
            return Err(format!(
                "read {consumed} bytes of head out of {} that arrived",
                damaged.len()
            ));
        }
        // A status MCF accepted is one the protocol has.
        if !(100..600).contains(&response.status()) {
            return Err(format!("accepted the status {}", response.status()));
        }
        // Every number read back is one the source wrote, or a refusal. What
        // must not happen is a number nobody sent.
        if let Ok(Some(length)) = response.content_length() {
            // Against the bytes themselves rather than their rendering: the
            // rendering truncates for a report, and a property that read it
            // would fail on inputs the mutation grew past that limit — a defect
            // in the check reported as a defect in the code.
            let written = length.to_string();
            if !damaged
                .windows(written.len())
                .any(|window| window == written.as_bytes())
            {
                return Err(format!("read a content-length of {length} nobody sent"));
            }
        }
        if let Ok(Some(range)) = response.content_range()
            && let Some(total) = range.total
            && range.last >= total
        {
            return Err(format!(
                "accepted a range ending at {} of a file {total} long",
                range.last
            ));
        }
        // And a redirect never carries the credential off this origin, whatever
        // the source put in its location. This is the property B-322 exists
        // for, examined over what a source can actually say rather than over
        // the four locations a unit test writes.
        // A refusal and *no redirect* are both fine here: what is being
        // examined is the one case where something leaves this machine.
        if let Ok(Some((to, carried))) = http::redirect(&response, &from)
            && carried
            && !from.same_origin(&to)
        {
            return Err(format!("would have carried the credential to {to}"));
        }
        Ok(())
    });
    reached.report();
    assert_held(&verdict);
}

/// A model file is the largest untrusted input MCF will ever read, and the one
/// whose header is a set of lengths (B-360, §3.7).
///
/// A parser driven by lengths in the file is the classic way to turn a damaged
/// download into an allocation the machine cannot satisfy, so what is asserted
/// here is what a reader owes: no panic, a classified refusal, and — where it
/// accepts — a directory whose arithmetic holds together.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_model_reader_survives_a_damaged_model_file() {
    let (seed, cases) = announce("gguf::parse");
    let reached = Reached::new("gguf::parse");
    let corpus = model_corpus();
    let verdict = check_from(seed, cases, |rng| {
        let original = rng.pick(&corpus).ok_or("the corpus is not empty")?.clone();
        let damaged = mutate(rng, &original);
        let Ok(model) = gguf::parse(&damaged) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();

        // What it accepted has to hold together. A tensor whose size is
        // computable must not claim to start past the end of what the file
        // could hold, and every element count must be computable at all —
        // otherwise a later read of its bytes is a read of somebody else's.
        for tensor in &model.tensors {
            if let Some(bytes) = tensor.bytes()
                && tensor.offset.checked_add(bytes).is_none()
            {
                return Err(format!(
                    "accepted a tensor whose offset plus size overflows: {} at {} + {bytes}: {}",
                    tensor.name,
                    tensor.offset,
                    render(&damaged)
                ));
            }
        }
        if model.alignment == 0 || !model.alignment.is_power_of_two() {
            return Err(format!(
                "accepted an alignment of {}: {}",
                model.alignment,
                render(&damaged)
            ));
        }
        if model.data_offset % model.alignment != 0 {
            return Err(format!(
                "accepted a data offset of {} under an alignment of {}: {}",
                model.data_offset,
                model.alignment,
                render(&damaged)
            ));
        }
        Ok(())
    });
    reached.report();
    assert_held(&verdict);
}

/// Well-formed model files to damage: one minimal, one with metadata, a
/// vocabulary and tensors.
///
/// Written here rather than borrowed from the reader's own tests, for the
/// reason the laboratory's scenarios are: a corpus built by the code under test
/// is a corpus that shares its assumptions.
fn model_corpus() -> Vec<Vec<u8>> {
    let mut minimal = b"GGUF".to_vec();
    minimal.extend_from_slice(&3_u32.to_le_bytes());
    minimal.extend_from_slice(&0_u64.to_le_bytes());
    minimal.extend_from_slice(&0_u64.to_le_bytes());

    let mut whole = b"GGUF".to_vec();
    whole.extend_from_slice(&3_u32.to_le_bytes());
    whole.extend_from_slice(&1_u64.to_le_bytes());
    whole.extend_from_slice(&2_u64.to_le_bytes());
    // general.architecture = "llama"
    push_string(&mut whole, "general.architecture");
    whole.extend_from_slice(&8_u32.to_le_bytes());
    push_string(&mut whole, "llama");
    // llama.context_length = 4096
    push_string(&mut whole, "llama.context_length");
    whole.extend_from_slice(&5_u32.to_le_bytes());
    whole.extend_from_slice(&4096_u32.to_le_bytes());
    // one tensor: token_embd.weight, 8x3, f32, at 0
    push_string(&mut whole, "token_embd.weight");
    whole.extend_from_slice(&2_u32.to_le_bytes());
    whole.extend_from_slice(&8_u64.to_le_bytes());
    whole.extend_from_slice(&3_u64.to_le_bytes());
    whole.extend_from_slice(&0_u32.to_le_bytes());
    whole.extend_from_slice(&0_u64.to_le_bytes());

    vec![minimal, whole]
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&u64::try_from(value.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

/// A bundle arrived from somewhere else, and is the most untrusted input MCF
/// has. What it says about itself is checked against what it holds.
///
/// The invariant is the reader's own claim: it refuses a bundle whose entries
/// do not match its manifest, and what it returns matches. A reader that
/// accepted a damaged bundle as one with fewer rows would be the silent
/// shortening B62 forbids, arriving by post.
#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_bundle_reader_never_accepts_what_it_cannot_account_for() {
    let (seed, _) = announce("export::read");
    let cases = fuzz::filesystem_cases();
    let reached = Reached::new("export::read");
    let verdict = check_from(seed, cases.max(1), |rng| {
        let scratch = Scratch::new("fuzz-bundle");
        {
            let mut journal =
                Journal::open(&scratch.journal()).map_err(|failure| failure.to_string())?;
            for _entry in 0..=rng.index(4) {
                journal
                    .append(&Entry::new(
                        EntryKind::Trials,
                        Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
                        Value::map([("n", Value::Integer(rng.integer_between(0, 1_000)))]),
                    ))
                    .map_err(|failure| failure.to_string())?;
            }
        }
        let bundle = scratch.join("bundle.mcf");
        export::write(&scratch.journal(), &bundle, export::Kind::Export)
            .map_err(|failure| failure.to_string())?;

        let whole = std::fs::read(&bundle).map_err(|error| error.to_string())?;
        let damaged = mutate(rng, &whole);
        std::fs::write(&bundle, &damaged).map_err(|error| error.to_string())?;

        let Ok((_, manifest, entries)) = export::read(&bundle) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();
        if entries.len() != manifest.entries {
            return Err(format!(
                "accepted a bundle holding {} entries and stating {}: {}",
                entries.len(),
                manifest.entries,
                render(&damaged)
            ));
        }
        // Recomputed here rather than trusted: this is the check the reader
        // claims to have made, made independently (A19).
        let mut hasher = Sha256::new();
        for entry in &entries {
            hasher.update(entry.to_line().as_bytes());
            hasher.update(b"\n");
        }
        if hasher.finish().hex() != manifest.digest {
            return Err(format!(
                "accepted a bundle whose entries do not digest to its manifest: {}",
                render(&damaged)
            ));
        }
        Ok(())
    });
    reached.report();
    assert_held(&verdict);
}
