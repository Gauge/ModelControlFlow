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

#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_record_codec_survives_damaged_lines() {
    let (seed, cases) = announce("json::parse");
    let reached = Reached::new("json::parse");
    let verdict = check_from(seed, cases, |rng| {
        let original = valid_line(rng);
        let damaged = mutate(rng, original.as_bytes());
        let Ok(text) = std::str::from_utf8(&damaged) else {
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

#[test]
#[ignore = "the fuzz tier is scheduled: scripts/ci.sh --with-fuzz (B38)"]
fn the_journal_replay_survives_a_damaged_file() {
    let (seed, _) = announce("journal::replay");
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
            #[allow(
                clippy::collapsible_if,
                reason = "the nested form is kept deliberately"
            )]
            if let Some(offset) = zone.offset_at(moment) {
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

fn zone_corpus() -> Vec<Vec<u8>> {
    let mut corpus = Vec::new();
    for path in ["/etc/localtime", "/usr/share/zoneinfo/UTC"] {
        if let Ok(bytes) = std::fs::read(path) {
            corpus.push(bytes);
        }
    }
    corpus
}

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
            return Ok(());
        };
        let Ok(parsed) = reference::parse(written) else {
            reached.refused();
            return Ok(());
        };
        reached.accepted();

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

        if consumed > damaged.len() {
            return Err(format!(
                "read {consumed} bytes of head out of {} that arrived",
                damaged.len()
            ));
        }
        if !(100..600).contains(&response.status()) {
            return Err(format!("accepted the status {}", response.status()));
        }
        if let Ok(Some(length)) = response.content_length() {
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

fn model_corpus() -> Vec<Vec<u8>> {
    let mut minimal = b"GGUF".to_vec();
    minimal.extend_from_slice(&3_u32.to_le_bytes());
    minimal.extend_from_slice(&0_u64.to_le_bytes());
    minimal.extend_from_slice(&0_u64.to_le_bytes());

    let mut whole = b"GGUF".to_vec();
    whole.extend_from_slice(&3_u32.to_le_bytes());
    whole.extend_from_slice(&1_u64.to_le_bytes());
    whole.extend_from_slice(&2_u64.to_le_bytes());
    push_string(&mut whole, "general.architecture");
    whole.extend_from_slice(&8_u32.to_le_bytes());
    push_string(&mut whole, "llama");
    push_string(&mut whole, "llama.context_length");
    whole.extend_from_slice(&5_u32.to_le_bytes());
    whole.extend_from_slice(&4096_u32.to_le_bytes());
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
