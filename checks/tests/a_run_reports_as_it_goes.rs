#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

fn bench() -> String {
    std::fs::read_to_string(mcf_checks::workspace::root().join("crates/mcf-cli/src/bench.rs"))
        .expect("bench.rs is readable")
}

#[test]
fn progress_does_not_reach_standard_output() {
    let source = bench();
    assert!(
        source.contains("fn so_far(") && source.contains("eprintln!"),
        "the run must report as it goes, on standard error (B-227)"
    );
    assert_eq!(
        source.matches("println!").count(),
        source.matches("eprintln!").count(),
        "and never on standard output: the result is what goes there, and a pipeline reading a \
         verdict must not have to filter progress out of it"
    );
}

#[test]
fn an_interim_line_says_how_far_it_got() {
    let source = bench();
    let (_, said) = source
        .split_once("fn so_far(")
        .expect("`so_far` is what reports as it goes");
    let (said, _) = said.split_once("\n}").expect("and it ends somewhere");
    assert!(
        said.contains("after {pairs} pair(s), so far"),
        "an interim line must say how many pairs it rests on and that it is interim — a reader \
         who scrolls back must not be able to read it as the answer (§3.1)"
    );
    assert!(
        said.contains("if pairs < 2"),
        "and must say nothing below two pairs, which is the floor at which there is a \
         comparison to report at all"
    );
}

#[test]
fn reporting_cannot_change_what_the_run_does() {
    let source = bench();
    let (_, said) = source
        .split_once("fn so_far(")
        .expect("`so_far` is what reports as it goes");
    let (said, _) = said.split_once("\n}").expect("and it ends somewhere");
    for forbidden in ["break", "std::process::exit", "panic!", "return Err"] {
        assert!(
            !said.contains(forbidden),
            "`so_far` contains `{forbidden}`: A18 gives a benchmark no pass condition, and a \
             progress line that could stop a run would be one"
        );
    }
    assert!(
        !said.contains("&mut "),
        "and it takes the comparison by reference: a reporter that could change what it \
         reports on is not a reporter"
    );
}
