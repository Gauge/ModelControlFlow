use super::*;

const ROW: &str = "comparison · example-a.gguf quicker than example-b.gguf by 18.5%, over 6 \
                   pair(s) · workload a declared workload · under MCF 0.1.0-m0";
const ABSOLUTE: &str = "absolute · example-a.gguf took 12345 ns · workload a declared workload \
                        · under MCF 0.1.0-m0";
const COUNT: &str = "1 row(s): 1 comparison(s) and 0 absolute(s)";

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("mcf-site-{name}-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn the_arms_are_read_by_the_words_between_them() {
    assert_eq!(arms_in(ROW), vec!["example-a.gguf", "example-b.gguf"]);
    assert_eq!(arms_in(ABSOLUTE), vec!["example-a.gguf"]);
}

#[test]
fn a_row_of_another_shape_yields_nothing() {
    for held in [
        "",
        "not a row at all",
        "comparison · with no verb in it",
        "something · else entirely",
        "· ",
    ] {
        assert!(arms_in(held).is_empty(), "{held:?} produced an arm");
    }
}

#[test]
fn the_count_line_is_not_a_row() {
    let held = Held {
        digest: "x".to_owned(),
        at: 0,
        body: format!("{ROW}\n{COUNT}\n"),
    };
    assert_eq!(held.rows().len(), 1, "the count was counted as a row");
    assert!(!is_a_row(COUNT), "a summary of the rows is not a row");
    assert!(is_a_row(ROW));
    assert!(is_a_row(ABSOLUTE));
    for held in ["", "just some words", "comparison ·", "· something"] {
        assert!(!is_a_row(held), "{held:?} was taken for a row");
    }
}

#[test]
fn what_arrives_is_what_is_kept() {
    let archive = Archive::at(scratch("verbatim"));
    let body = format!("{ROW}\n{ABSOLUTE}\n{COUNT}\n");
    let digest = archive.keep(&body).expect("it is a contribution");
    let read = archive.one(&digest).expect("it was filed");
    assert_eq!(read.body, body, "the bytes changed on the way through");
    assert_eq!(read.rows().len(), 2);
    assert_eq!(read.arms(), vec!["example-a.gguf", "example-b.gguf"]);
    let _cleared = std::fs::remove_dir_all(root_of(&archive));
}

#[test]
fn the_same_file_twice_is_one_entry() {
    let archive = Archive::at(scratch("twice"));
    let body = format!("{ROW}\n{COUNT}\n");
    let first = archive.keep(&body).expect("filed");
    let second = archive.keep(&body).expect("filed again");
    assert_eq!(first, second);
    assert_eq!(archive.all().len(), 1);
    let _cleared = std::fs::remove_dir_all(root_of(&archive));
}

#[test]
fn a_file_with_no_rows_is_refused() {
    let archive = Archive::at(scratch("empty"));
    let refused = archive.keep("").expect_err("nothing is not a contribution");
    assert!(refused.contains("empty"), "{refused}");
    let refused = archive
        .keep("just some words\n")
        .expect_err("that is not a contribution either");
    assert!(refused.contains("mcf share"), "{refused}");
    let _cleared = std::fs::remove_dir_all(root_of(&archive));
}

#[test]
fn a_name_that_is_not_one_never_becomes_a_path() {
    let archive = Archive::at(scratch("traversal"));
    for held in [
        "../../../etc/passwd",
        "..",
        "/etc/passwd",
        "abc",
        "",
        "................................",
        "0123456789abcdef0123456789abcdeZ",
    ] {
        assert!(archive.one(held).is_none(), "{held:?} was looked up");
    }
    let _cleared = std::fs::remove_dir_all(root_of(&archive));
}

#[test]
fn a_name_follows_the_bytes() {
    assert_eq!(digest_of("a"), digest_of("a"));
    assert_ne!(digest_of("a"), digest_of("b"));
    assert_eq!(digest_of("a").len(), 32);
    assert!(digest_of("anything at all").chars().all(|c| c.is_ascii_hexdigit()));
}
