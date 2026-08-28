//! What the agreement type has to say correctly, without a model.

use super::{Agreement, FURTHEST_RANK};

fn an_agreement(furthest: usize) -> Agreement {
    Agreement {
        positions: 250,
        agreed: 247,
        furthest,
        furthest_at: 61,
        set_aside: 1,
    }
}

/// A top-two swap is arithmetic and a token ranked hundredth is not, and the
/// line between them is the one F41 measured rather than one chosen here.
#[test]
fn the_line_is_where_it_was_measured() {
    assert!(an_agreement(1).within_arithmetic());
    assert!(an_agreement(3).within_arithmetic(), "F40's clean corpus");
    assert!(an_agreement(FURTHEST_RANK).within_arithmetic(), "the line");
    assert!(!an_agreement(FURTHEST_RANK + 1).within_arithmetic());
    assert!(
        !an_agreement(618).within_arithmetic(),
        "F41's broken attention mechanism"
    );
}

/// The verdict travels with the numbers behind it, so that a reader can
/// disagree with the threshold rather than only with the answer (A19, §3.15).
#[test]
fn the_record_carries_what_the_verdict_rests_on() {
    let said = an_agreement(3).to_value().to_line();
    for wanted in [
        "positions",
        "agreed",
        "furthest_rank",
        "furthest_rank_allowed",
        "set_aside",
        "within_arithmetic",
    ] {
        assert!(said.contains(wanted), "{wanted} is missing from {said}");
    }
}
