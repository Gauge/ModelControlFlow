//! What the agreement type has to say correctly, without a model.

use super::{Agreement, FURTHEST_RANK};

fn an_agreement(furthest: usize) -> Agreement {
    Agreement {
        positions: 250,
        agreed: 247,
        furthest,
        furthest_at: 61,
        set_aside: 1,
        ranks: Vec::new(),
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

/// The sentences every surface prints say the verdict, the figures it rests
/// on, and what was set aside — and say DIVERGE where it is, rather than
/// which engine is wrong (A19, B-072).
#[test]
fn the_sentences_carry_the_verdict_and_its_figures() {
    let agreed = an_agreement(3).said();
    assert_eq!(agreed.len(), 4, "{agreed:?}");
    assert!(agreed[0].contains("250 position(s)") && agreed[0].contains("at 247"));
    assert!(agreed[1].contains("1 position(s) set aside"));
    assert!(agreed[2].starts_with("AGREE") && agreed[2].contains("rank 3"));
    assert!(agreed[3].contains("neither engine is the authority"));

    let mut apart = an_agreement(618);
    apart.set_aside = 0;
    let said = apart.said();
    assert_eq!(said.len(), 3, "nothing set aside, nothing said about it");
    assert!(
        said[1].starts_with("DIVERGE")
            && said[1].contains("position 61")
            && said[1].contains("618"),
        "{said:?}"
    );
    assert!(said[1].contains("does not say which"));
}
