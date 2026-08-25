//! What MCF is licensed under, stated by the artifact itself (B-330, D28).
//!
//! B-330's condition has two halves. The first — no vendored component ships
//! without a recorded compatibility finding — is [vendored.md] and the check
//! that reads it. This is the second: **the licence is stated in the artifact
//! and surfaced to a redistributor**.
//!
//! **Why the artifact and not the repository.** A redistributor has a binary.
//! GPL-3.0 §4 asks that whoever conveys a copy of the Program give its
//! recipients a copy of the License, and §6 that they be told how to get the
//! source. A licence that lives only in a file next to the code is a licence
//! the person with the obligation does not have, so the whole text is compiled
//! in and `mcf licence --full` prints it. It costs about 35 KiB of a 40 MiB
//! ceiling (D24), which is the cheapest obligation MCF has.
//!
//! **What it says about source is what MCF knows.** A24 and A7 both apply: the
//! source is the tree the binary was built from, named by `MCF_BUILD_COMMIT`
//! where the builder set one and stated as unknown where nobody did. MCF does
//! not invent a repository URL it cannot verify — an offer nobody can act on is
//! worse than a stated absence.
//!
//! **The vendored list is empty and says so.** When D23 admits a component, it
//! appears here *and* in [vendored.md], and a check requires the two agree:
//! a component whose terms a redistributor cannot read is exactly the failure
//! B-330 exists to prevent.
//!
//! [vendored.md]: ../../../doc/vendored.md

use mcf_core::build_identity::{BuildIdentity, SourceRevision};

/// The licence MCF is under, as SPDX names it (D28).
pub(crate) const SPDX: &str = "GPL-3.0-only";

/// The whole licence, compiled in.
const FULL_TEXT: &str = include_str!("../../../LICENSE");

/// A component MCF ships that it did not write.
///
/// Empty, and the emptiness is checked: `checks/tests/the_licence_it_says.rs`
/// requires this list and [vendored.md]'s admitted section to name the same
/// components, so a vendored tree cannot arrive without the artifact learning
/// to say what its terms are.
///
/// [vendored.md]: ../../../doc/vendored.md
pub(crate) const VENDORED: &[Component] = &[];

/// What a redistributor needs to know about one shipped component.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Component {
    /// What it is called.
    pub(crate) name: &'static str,
    /// The terms it is under, as SPDX names them.
    pub(crate) terms: &'static str,
    /// The revision MCF ships, so the terms can be checked against the tree
    /// they came from rather than against the project's current one.
    pub(crate) revision: &'static str,
}

/// The licence statement, rendered.
///
/// `full` prints the entire text after it — what GPL-3.0 §4 asks a conveyor to
/// hand on.
pub(crate) fn render(identity: BuildIdentity, full: bool) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    // The writes below cannot fail: the target is a `String`. The results are
    // bound rather than discarded because A2's habit does not have exceptions
    // for the cases where the author is sure.
    let _written = write!(
        out,
        "MCF is free software under the GNU General Public License, version 3 only \
         ({SPDX}).\n\n\
         There is NO WARRANTY, to the extent permitted by law. See sections 15 and 16 \
         of the\nlicence for the full disclaimer.\n\n"
    );

    out.push_str("THE SOURCE\n");
    match identity.revision {
        SourceRevision::Known(revision) => {
            let _written = write!(
                out,
                "  This binary was built from revision {revision}. Conveying it obliges \
                 you to\n\x20 convey that source, or an offer of it, under section 6.\n"
            );
        }
        SourceRevision::Unknown => out.push_str(
            "  The revision this binary was built from is not recorded: whoever built it\n\
             \x20 did not set MCF_BUILD_COMMIT, and MCF does not invent one (A7). The source\n\
             \x20 is the tree it was built from, and conveying the binary obliges you to\n\
             \x20 convey that source, or an offer of it, under section 6.\n",
        ),
    }

    out.push_str("\nWHAT MCF SHIPS THAT IT DID NOT WRITE\n");
    if VENDORED.is_empty() {
        out.push_str(
            "  Nothing. No component has been admitted, so there are no other terms to\n\
             \x20 honour. When one is, it is listed here with its terms and the revision\n\
             \x20 shipped (doc/vendored.md holds the compatibility finding).\n",
        );
    } else {
        for component in VENDORED {
            let _written = writeln!(
                out,
                "  {} at {} — {}",
                component.name, component.revision, component.terms
            );
        }
    }

    if full {
        out.push('\n');
        out.push_str(FULL_TEXT);
    } else {
        out.push_str(
            "\nRun `mcf licence --full` for the whole text, which is compiled into this\n",
        );
        out.push_str("binary rather than referred to.\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{FULL_TEXT, SPDX, VENDORED, render};
    use mcf_core::build_identity::BuildIdentity;

    /// The text compiled in is the licence MCF declares, not a placeholder.
    #[test]
    fn the_text_in_the_artifact_is_the_gpl_version_three() {
        assert!(FULL_TEXT.contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(FULL_TEXT.contains("Version 3, 29 June 2007"));
        assert!(
            !FULL_TEXT.contains("GNU AFFERO"),
            "{SPDX} is not the Affero variant"
        );
        assert!(FULL_TEXT.lines().count() > 600);
    }

    /// The short form names the licence, the warranty position and the source
    /// obligation — the three things a redistributor has to act on.
    #[test]
    fn the_statement_names_what_a_redistributor_must_act_on() {
        let text = render(BuildIdentity::current(), false);
        assert!(text.contains(SPDX), "{text}");
        assert!(text.contains("NO WARRANTY"), "{text}");
        assert!(text.contains("section 6"), "{text}");
        assert!(
            !text.contains("GNU GENERAL PUBLIC LICENSE\n"),
            "the short form printed the whole text"
        );
    }

    /// The long form is the short form and then the licence itself.
    #[test]
    fn the_full_form_carries_the_licence_with_it() {
        let text = render(BuildIdentity::current(), true);
        assert!(text.contains(SPDX));
        assert!(
            text.contains("TERMS AND CONDITIONS"),
            "the full text is not there"
        );
        assert!(text.len() > FULL_TEXT.len());
    }

    /// An empty vendored list says so rather than printing nothing, because a
    /// blank section reads as an omission and this one is a fact.
    #[test]
    fn nothing_vendored_is_stated_rather_than_left_blank() {
        assert!(VENDORED.is_empty(), "the list grew and this test did not");
        let text = render(BuildIdentity::current(), false);
        assert!(
            text.contains("Nothing. No component has been admitted"),
            "{text}"
        );
    }
}
