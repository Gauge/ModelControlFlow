#![allow(clippy::panic, clippy::expect_used)]

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

#[test]
fn the_only_rendering_of_a_contribution_carries_every_row() {
    let source = read("crates/mcf-core/src/contribution.rs");
    let (_, after) = source
        .split_once("impl fmt::Display for Contribution {")
        .expect("a contribution renders itself");
    let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
    assert!(
        body.contains("for row in &self.rows"),
        "a contribution's rendering must write every row: a count is the description A24 \
         forbids being shown instead of them"
    );
    assert_eq!(
        source.matches("impl fmt::Display for Contribution").count(),
        1,
        "a second rendering is a rendering that can drop the rows"
    );
    for held in [
        "impl fmt::Display for Row",
        "impl fmt::Display for Comparison",
        "impl fmt::Display for Absolute",
    ] {
        assert!(source.contains(held), "{held} is missing");
    }
}

#[test]
fn the_surface_states_the_terms_and_that_nothing_left() {
    let source = read("crates/mcf-cli/src/share.rs");
    assert!(
        source.contains("TERMS"),
        "the share surface must state the terms before anything is written (D21, §3.20)"
    );
    assert!(
        source.contains("NOTHING HAS LEFT THIS MACHINE"),
        "and must say plainly that producing is not sending (A24)"
    );
    assert!(
        source.contains("cannot be undone")
            || read("crates/mcf-core/src/contribution.rs").contains("cannot be undone"),
        "and that publication cannot be undone (D21, B63)"
    );
    for suggesting in ["fn retract", "fn unsend", "fn recall", "--undo"] {
        assert!(
            !source.contains(suggesting),
            "the share surface offers {suggesting}, and there is no such act (D21, B63)"
        );
    }
}

#[test]
fn what_cannot_travel_is_named_rather_than_omitted() {
    let source = read("crates/mcf-cli/src/share.rs");
    assert!(
        source.contains("what cannot travel, and why"),
        "a share that silently dropped what it could not carry would leave the operator \
         believing they had shared something they had not (A1, A4)"
    );
    assert!(
        source.contains("could not be read"),
        "and a row the record could not read is neither included nor counted, which is said"
    );
}

#[test]
fn the_file_is_what_was_confirmed() {
    let source = read("crates/mcf-cli/src/share.rs");
    assert!(
        source.contains("format!(\"{held}\\n\")"),
        "the file must be written from the same rendering the confirmation showed: a file \
         whose contents differ from the confirmation is a confirmation of something else (A24)"
    );
}
