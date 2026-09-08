#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "a measurement that cannot set up its own fixture has nothing to \
              report, and saying so loudly is the whole of its error handling"
)]

use std::io::Write as _;
use std::time::Instant;

use mcf_core::time::Timestamp;
use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;

const SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];

#[test]
#[ignore = "writes a million entries; run by scripts/ci.sh --with-budget"]
fn how_the_record_grows() {
    let root = std::env::temp_dir().join(format!("mcf-record-growth-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&root));
    std::fs::create_dir_all(&root).expect("a place to write");

    let (header, line, per_append) = one_real_entry(&root);
    println!("one append, durable:  {per_append:?}");
    println!(
        "one entry on disk:    {} bytes",
        line.len().saturating_add(1)
    );

    for count in SIZES {
        let journal = root.join(format!("record-{count}.jsonl"));
        write_journal(&journal, &header, &line, count);
        let bytes = std::fs::metadata(&journal).expect("it stats").len();

        let started = Instant::now();
        let replayed = replay(&journal).expect("it replays");
        let replaying = started.elapsed();
        assert_eq!(replayed.entries.len(), count);

        let index_path = index::default_path(&journal);
        let started = Instant::now();
        let built = Index::over(&journal, &index_path).expect("it indexes");
        let building = started.elapsed();

        let started = Instant::now();
        let opened = Index::over(&journal, &index_path).expect("it opens");
        let opening = started.elapsed();

        let started = Instant::now();
        let latest = opened.latest(Some(EntryKind::ArtifactAcquired), 20);
        let entries: Vec<Entry> = latest
            .iter()
            .map(|located| opened.read(located).expect("it reads"))
            .collect();
        let querying = started.elapsed();

        assert_eq!(built.entries().len(), count);
        assert_eq!(opened.entries().len(), count);
        assert_eq!(entries.len(), 20.min(count));

        let index_bytes = std::fs::metadata(&index_path).expect("it stats").len();
        println!(
            "{count:>9} entries: journal {:>6} MiB, index {:>4} MiB | replay {replaying:>12.2?} \
             | build {building:>12.2?} | open {opening:>12.2?} | last 20 {querying:>12.2?}",
            mib(bytes),
            mib(index_bytes),
        );
    }
    drop(std::fs::remove_dir_all(&root));
}

fn one_real_entry(root: &std::path::Path) -> (String, String, std::time::Duration) {
    let path = root.join("seed.jsonl");
    let mut journal = Journal::open(&path).expect("it opens");
    let body = Value::map([
        ("repository", Value::text("a-publisher/a-model-GGUF")),
        ("file", Value::text("a-model-Q4_K_M.gguf")),
        ("bytes", Value::Integer(4_920_000_000)),
        (
            "verification",
            Value::map([
                ("state", Value::text("verified")),
                (
                    "sha256",
                    Value::text("9f2c1e4a7b6d5c3f8e0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60"),
                ),
            ]),
        ),
    ]);

    let trials = 100;
    let started = Instant::now();
    for _trial in 0..trials {
        journal
            .append(&Entry::new(
                EntryKind::ArtifactAcquired,
                Timestamp::now(),
                body.clone(),
            ))
            .expect("it appends");
    }
    let per_append = started
        .elapsed()
        .checked_div(u32::try_from(trials).unwrap_or(1))
        .unwrap_or_default();

    let text = std::fs::read_to_string(&path).expect("it reads");
    let mut lines = text.lines();
    let header = lines.next().expect("a header").to_owned();
    let line = lines.next().expect("an entry").to_owned();
    (header, line, per_append)
}

fn write_journal(path: &std::path::Path, header: &str, line: &str, count: usize) {
    let file = std::fs::File::create(path).expect("it creates");
    let mut out = std::io::BufWriter::new(file);
    writeln!(out, "{header}").expect("the header");
    for _ in 0..count {
        writeln!(out, "{line}").expect("a line");
    }
    out.flush().expect("it flushes");
}

fn mib(bytes: u64) -> u64 {
    bytes.checked_div(1024 * 1024).unwrap_or(0)
}
