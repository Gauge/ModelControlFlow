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
//! **The vendored list is the TLS stack.** It appears here *and* in
//! [vendored.md], and a check requires the two agree: a component whose terms a
//! redistributor cannot read is exactly the failure B-330 exists to prevent.
//! What is listed is what is *compiled in* — the tree also holds crates cargo
//! requires to be present and nothing builds, and a conveyor is not conveying
//! those.
//!
//! [vendored.md]: ../../../doc/vendored.md

use mcf_core::build_identity::{BuildIdentity, SourceRevision};

/// The licence MCF is under, as SPDX names it (D28).
pub(crate) const SPDX: &str = "GPL-3.0-only";

/// The whole licence, compiled in.
const FULL_TEXT: &str = include_str!("../../../LICENSE");

/// A component MCF ships that it did not write.
///
/// The TLS stack, and nothing else: fourteen crates that are *compiled into
/// this binary*, which is what a redistributor conveys and therefore what they
/// need the terms of. The vendored tree also holds crates no target compiles —
/// cargo requires them present — and those are in [vendored.md] rather than
/// here, because a list of what somebody is conveying should not include what
/// they are not.
///
/// `checks/tests/the_licence_is_what_it_says.rs` requires this list and the
/// register to agree, so a vendored tree cannot arrive without the artifact
/// learning to say what its terms are.
///
/// [vendored.md]: ../../../doc/vendored.md
pub(crate) const VENDORED: &[Component] = &[
    Component {
        name: "cfg-if",
        terms: "MIT OR Apache-2.0",
        revision: "1.0.4",
    },
    Component {
        name: "getrandom",
        terms: "MIT OR Apache-2.0",
        revision: "0.2.17",
    },
    Component {
        name: "getrandom",
        terms: "MIT OR Apache-2.0",
        revision: "0.3.4",
    },
    Component {
        name: "graviola",
        terms: "Apache-2.0 OR ISC OR MIT-0",
        revision: "0.4.1",
    },
    Component {
        name: "libc",
        terms: "MIT OR Apache-2.0",
        revision: "0.2.189",
    },
    Component {
        name: "once_cell",
        terms: "MIT OR Apache-2.0",
        revision: "1.21.4",
    },
    Component {
        name: "rustls",
        terms: "Apache-2.0 OR ISC OR MIT",
        revision: "0.23.43",
    },
    Component {
        name: "rustls-graviola",
        terms: "Apache-2.0 OR ISC OR MIT-0",
        revision: "0.4.0",
    },
    Component {
        name: "rustls-pki-types",
        terms: "MIT OR Apache-2.0",
        revision: "1.15.1",
    },
    Component {
        name: "rustls-webpki",
        terms: "ISC",
        revision: "0.103.15",
    },
    Component {
        name: "subtle",
        terms: "BSD-3-Clause",
        revision: "2.6.1",
    },
    Component {
        name: "untrusted",
        terms: "ISC",
        revision: "0.9.0",
    },
    Component {
        name: "webpki-roots",
        terms: "CDLA-Permissive-2.0",
        revision: "1.0.9",
    },
    Component {
        name: "zeroize",
        terms: "Apache-2.0 OR MIT",
        revision: "1.9.0",
    },
];

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
        out.push_str(
            "  Each is under terms compatible with GPL-3.0-only, and conveying this binary
               conveys them too: doc/vendored.md records what each declares, what MCF
               found in its tree, and the three that state their terms and ship no copy.
",
        );
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

    /// What MCF ships that it did not write is named, with its terms and the
    /// revision — which is what a person conveying the binary is obliged to
    /// pass on, and what they cannot get from a repository they do not have
    /// (GPL-3.0 §4, B-330).
    #[test]
    fn what_is_shipped_is_named_with_its_terms() {
        assert!(
            !VENDORED.is_empty(),
            "the vendored list emptied and this test did not: if nothing is shipped, say so \
             the way the render does"
        );
        let text = render(BuildIdentity::current(), false);
        for component in VENDORED {
            assert!(
                text.contains(component.name),
                "{} is shipped and unnamed",
                component.name
            );
            assert!(
                text.contains(component.terms),
                "{} is named without its terms",
                component.name
            );
            assert!(
                text.contains(component.revision),
                "{} is named without the revision shipped, so its terms cannot be checked \
                 against the tree they came from",
                component.name
            );
        }
    }

    /// The TLS stack is what is shipped, and the reader is pointed at the
    /// register for what MCF verified about each of them (A21).
    #[test]
    fn the_shipped_list_is_the_stack_and_points_at_the_register() {
        let text = render(BuildIdentity::current(), false);
        for expected in ["rustls", "rustls-graviola", "webpki-roots"] {
            assert!(text.contains(expected), "{expected} is not named: {text}");
        }
        assert!(text.contains("doc/vendored.md"), "{text}");
        assert!(text.contains("GPL-3.0-only"), "{text}");
    }
}
