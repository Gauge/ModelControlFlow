//! Threads may divide the work and may never divide a sum (B-366, D38, §3.12,
//! D19).
//!
//! **What is being protected.** Floating-point addition is not associative, so
//! an engine that split one reduction across two threads would give an answer
//! that depended on how many threads it got — which is to say, on how busy the
//! machine was. `mcf-standin`'s property tests assert that the answer does not
//! move today, over generated inputs and over a real forward pass. They cannot
//! assert that the *shape* which makes it true stays in place, and that shape is
//! three facts:
//!
//! 1. the engine partitions in exactly one place, so there is one thing to read;
//! 2. every product's rows go through that place, so nothing hand-rolls a second
//!    partition beside it;
//! 3. the serial path and the partitioned path compute a row with the same
//!    function, so the two cannot drift apart.
//!
//! **Why a check rather than a comment.** The failure mode is specific and
//! foreseeable: somebody profiling the engine finds that the output projection
//! dominates, splits its *columns* across threads to use more of them, and
//! every number MCF produces silently becomes a function of the machine's load.
//! That change would pass every existing test on a quiet machine.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

/// Where the engine lives.
const ENGINE: &str = "crates/mcf-standin/src";

/// The one module allowed to start a thread.
const PARTITION: &str = "crates/mcf-standin/src/threads.rs";

/// Every module of the engine that is not itself a test.
///
/// A `tests.rs` beside a module is that module's tests: it may start a thread,
/// and it may call the partition directly — that is how the partition is
/// checked at all.
fn engine_sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&mcf_checks::workspace::root().join(ENGINE), &mut found);
    found.retain(|path| path.file_name().is_none_or(|name| name != "tests.rs"));
    found.sort();
    found
}

fn walk(directory: &std::path::Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|kind| kind == "rs") {
            into.push(path);
        }
    }
}

fn relative(path: &std::path::Path) -> String {
    path.strip_prefix(mcf_checks::workspace::root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Production lines only: a test may start a thread to prove something about
/// one.
fn code_of(path: &std::path::Path) -> String {
    let source = std::fs::read_to_string(path).expect("an engine source is readable");
    let mut kept = String::new();
    let mut in_tests = false;
    for line in source.lines() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            in_tests = true;
        }
        if in_tests || line.trim_start().starts_with("//") {
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    kept
}

/// One module starts threads, and it is the one whose whole subject is that
/// they change nothing.
#[test]
fn the_engine_partitions_in_exactly_one_place() {
    let mut offenders = Vec::new();
    for path in engine_sources() {
        if relative(&path) == PARTITION {
            continue;
        }
        let code = code_of(&path);
        for spelling in ["thread::scope", "thread::spawn", "spawn_scoped", "spawn("] {
            if code.contains(spelling) {
                offenders.push(format!("{} → {spelling}", relative(&path)));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a second place in the engine starts threads, so the partition is no longer one thing \
         a reader can check (B-366): {offenders:#?}"
    );

    let partition = code_of(&mcf_checks::workspace::root().join(PARTITION));
    assert!(
        partition.contains("thread::scope"),
        "the partition module no longer starts threads, so this check is reading the wrong file"
    );
}

/// Everything the partition divides is an *output index*, handed one row at a
/// time.
///
/// `each_row` is the only entry point, and it is called from one place. A
/// second caller is not wrong in itself — it is the point at which somebody
/// must think again about whether their loop is a reduction, and this check is
/// how that thinking is made to happen.
#[test]
fn every_partition_goes_through_the_one_entry_point() {
    let mut callers = Vec::new();
    for path in engine_sources() {
        if relative(&path) == PARTITION {
            continue;
        }
        let code = code_of(&path);
        // The name of a *test* is not a call; the check for the call is the
        // parenthesis, and a test called `…of_each_row()` has one too. What
        // separates them is the module path, which a call outside `threads`
        // must write.
        if code.contains("threads::each_row(") || code.contains("crate::threads::each_row(") {
            callers.push(relative(&path));
        }
    }
    assert_eq!(
        callers,
        vec!["crates/mcf-standin/src/ops.rs".to_owned()],
        "the set of places that partition work has changed. Each one must divide by output \
         index and never split a sum (B-366); if the new one does, add it here with the \
         reasoning"
    );
}

/// The serial path and the partitioned path compute a row with the same
/// function.
///
/// This is the fact the property rests on. If `matmul_vec` grew its own loop
/// back, the two could be edited apart — and the first symptom would be a
/// benchmark that disagreed with itself on a busy afternoon.
#[test]
fn the_serial_path_and_the_partitioned_path_share_their_arithmetic() {
    let ops = code_of(&mcf_checks::workspace::root().join("crates/mcf-standin/src/ops.rs"));

    let serial = body(&ops, "pub fn matmul_vec(");
    assert!(
        serial.contains("matmul_vec_across("),
        "`matmul_vec` no longer delegates to the partitioned form, so the two paths can be \
         edited apart (B-366)"
    );
    assert!(
        !serial.contains("for "),
        "`matmul_vec` has grown a loop of its own, which is a second implementation of the \
         arithmetic the partitioned path uses (B-366)"
    );

    let partitioned = body(&ops, "pub fn matmul_vec_across(");
    assert!(
        partitioned.contains("row_of(matrix, vector, row, columns)"),
        "the partitioned path no longer computes a row with `row_of`, so a thread count can \
         reach the order a sum is taken in (B-366)"
    );
    assert_eq!(
        ops.matches("fn row_of(").count(),
        1,
        "there is more than one `row_of`, so the two paths may not be summing the same way"
    );
}

/// The property this file protects is asserted somewhere, and on bits.
///
/// A structural check that outlived the test it protects would be a check
/// asserting the shape of a claim nobody makes any more.
#[test]
fn the_property_itself_is_asserted_on_bits() {
    for (path, what) in [
        (
            "crates/mcf-standin/tests/threads_do_not_change_the_answer.rs",
            "the product",
        ),
        (
            "crates/mcf-lab/tests/threads_do_not_change_the_answer.rs",
            "a whole forward pass",
        ),
    ] {
        let source = std::fs::read_to_string(mcf_checks::workspace::root().join(path))
            .unwrap_or_else(|_| panic!("the property test for {what} is in the tree: {path}"));
        assert!(
            source.contains("to_bits()"),
            "the property test for {what} no longer compares bits, and a comparison with a \
             tolerance would pass on exactly the divergence this exists to catch (B-366)"
        );
    }
}

/// The body of a function, from its signature to the closing brace at column
/// zero.
fn body<'a>(source: &'a str, signature: &str) -> &'a str {
    let after = source
        .split_once(signature)
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("`{signature}` is where this check looks and it is gone"));
    after.split_once("\n}").map_or(after, |(body, _)| body)
}
