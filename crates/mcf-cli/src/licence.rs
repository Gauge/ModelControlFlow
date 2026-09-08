use mcf_core::build_identity::{BuildIdentity, SourceRevision};

pub(crate) const SPDX: &str = "GPL-3.0-only";

const FULL_TEXT: &str = include_str!("../../../LICENSE");

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct Component {
    pub(crate) name: &'static str,
    pub(crate) terms: &'static str,
    pub(crate) revision: &'static str,
}

pub(crate) fn render(identity: BuildIdentity, full: bool) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
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
