#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

#[test]
fn letting_go_happens_in_one_place_and_that_place_records_it() {
    let source = read("crates/mcf-serve/src/daemon.rs");

    let mut taking = Vec::new();
    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.contains("holding.take()") {
            taking.push((number + 1, trimmed.to_owned()));
        }
    }
    assert_eq!(
        taking.len(),
        1,
        "a held model is taken out of its slot in more than one place, so one of them can let \
         go without writing it down (B-210, A26): {taking:#?}"
    );

    let (_, body) = source
        .split_once("fn let_go(")
        .expect("there is one place that lets go, and it is called let_go");
    let body = body.split_once("\n    }\n").map_or(body, |(held, _)| held);
    assert!(
        body.contains("holding.take()"),
        "the function named for letting go is not the one that lets go"
    );
    assert!(
        body.contains("EntryKind::ModelUnhosted"),
        "letting go does not write anything down, so the record can say a model is hosted \
         after it has stopped (A1, A26)"
    );
}

#[test]
fn a_daemon_lets_go_before_it_records_its_own_stop() {
    let source = read("crates/mcf-serve/src/daemon.rs");
    let (before, after) = source
        .split_once("EntryKind::DaemonStopped")
        .expect("the daemon records its own stop");
    let _ = after;
    let released = before
        .rfind("self.let_go(")
        .expect("the daemon lets go of a held model before recording that it stopped (B-210)");
    let answered = before
        .rfind("self.answer_one(")
        .expect("the daemon answers the request that stops it");
    assert!(
        released > answered,
        "the daemon records its stop without letting go of what it was holding first"
    );
}

#[test]
fn the_pair_is_a_pair() {
    let kinds = read("crates/mcf-record/src/journal/entry.rs");
    for half in ["ModelHosted", "ModelUnhosted"] {
        assert!(
            kinds.contains(half),
            "{half} is not a kind the record knows, so the pair is not a pair"
        );
    }
    let path: &Path = &mcf_checks::workspace::root().join("crates/mcf-serve/src/daemon.rs");
    assert!(
        path.is_file(),
        "this check looks at a file that is not there"
    );
}
