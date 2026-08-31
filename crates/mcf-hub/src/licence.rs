//! Reading what a repository says its terms are, and saying no more than that
//! (B-023, §III).
//!
//! §III requires a licence be legible **before** use, and B-023 requires every
//! artifact report its licence *or report it as unknown* — never as a plausible
//! default. [`mcf_core::provenance::Licence`] is the type with those states;
//! this is the matching that produces one from what a hub declared.
//!
//! **Three states, and the middle one is the one that matters.** An identifier
//! MCF recognizes; terms that are present and unmatched; nothing at all. A
//! repository whose licence MCF cannot parse is *not* a repository with no
//! licence, and collapsing the two would let MCF proceed past terms nobody
//! read.
//!
//! **What MCF says about a licence it recognizes, and what it refuses to say.**
//! It names the identifier and the family that identifier puts itself in —
//! permissive, copyleft, non-commercial, bespoke. That is reading a label, not
//! reading terms. What MCF does **not** do is tell anybody whether a particular
//! use is allowed: that is a legal judgement about a specific person's specific
//! use, MCF has no standing to make it, and a tool that guessed would be worse
//! than one that stays quiet. §III's *a use the terms forbid is stated rather
//! than discovered* is honoured by surfacing the terms early enough to read,
//! which is what this is for.
//!
//! **Whether a model's licence constrains publishing *measurements* about the
//! model is DEC-036, and it is open.** Nothing here decides it. When it closes,
//! the answer belongs beside `mcf share` (B-335) rather than here, because it
//! is a question about what MCF may send rather than about what a repository
//! said.

use mcf_core::provenance::Licence;

/// What an identifier says about itself.
///
/// Not a legal characterization and not advice: it is the family the licence's
/// own name and publisher put it in, which is a fact about the label. A reader
/// deciding anything reads the terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Family {
    /// Few conditions beyond attribution — MIT, BSD, Apache.
    Permissive,
    /// Conditions that follow the work into derivatives — the GPL family.
    Copyleft,
    /// States a non-commercial restriction in its own name.
    NonCommercial,
    /// A licence written for one model or one publisher, with terms that have
    /// to be read rather than recognized: the Llama community licences, Gemma's
    /// terms, the `OpenRAIL` family's use restrictions.
    Bespoke,
}

impl core::fmt::Display for Family {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Permissive => "permissive",
            Self::Copyleft => "copyleft",
            Self::NonCommercial => "non-commercial",
            Self::Bespoke => "bespoke — read the terms",
        })
    }
}

/// The identifiers MCF recognizes, and the family each puts itself in.
///
/// The list is what appears on the hub rather than the whole of SPDX: an
/// identifier nobody publishes is a row that can only be wrong. Anything not
/// here is [`Licence::Stated`] — present and unmatched — which is a state a
/// reader can act on, unlike a guess.
///
/// Adding a row is a claim that the identifier means what the row says. That is
/// why the bespoke family exists: it is where an identifier goes when MCF can
/// recognize the *name* and has no business summarizing the terms.
const KNOWN: &[(&str, Family)] = &[
    ("mit", Family::Permissive),
    ("apache-2.0", Family::Permissive),
    ("bsd-2-clause", Family::Permissive),
    ("bsd-3-clause", Family::Permissive),
    ("isc", Family::Permissive),
    ("mpl-2.0", Family::Permissive),
    ("artistic-2.0", Family::Permissive),
    ("cc0-1.0", Family::Permissive),
    ("cc-by-4.0", Family::Permissive),
    ("gpl-2.0", Family::Copyleft),
    ("gpl-3.0", Family::Copyleft),
    ("lgpl-3.0", Family::Copyleft),
    ("agpl-3.0", Family::Copyleft),
    ("cc-by-sa-4.0", Family::Copyleft),
    ("cc-by-nc-4.0", Family::NonCommercial),
    ("cc-by-nc-sa-4.0", Family::NonCommercial),
    ("cc-by-nc-nd-4.0", Family::NonCommercial),
    ("llama2", Family::Bespoke),
    ("llama3", Family::Bespoke),
    ("llama3.1", Family::Bespoke),
    ("llama3.2", Family::Bespoke),
    ("llama3.3", Family::Bespoke),
    ("gemma", Family::Bespoke),
    ("openrail", Family::Bespoke),
    ("openrail++", Family::Bespoke),
    ("creativeml-openrail-m", Family::Bespoke),
    ("bigscience-openrail-m", Family::Bespoke),
    ("bigcode-openrail-m", Family::Bespoke),
    ("apple-ascl", Family::Bespoke),
    ("deepfloyd-if-license", Family::Bespoke),
];

/// The identifiers a hub uses to mean *there is a licence and it is not one of
/// ours*.
///
/// Hugging Face's own vocabulary. They are recognized as *unmatched terms*
/// rather than as identifiers, because that is what they say: the repository
/// has terms and did not name them in a way anybody can match.
const MEANS_UNMATCHED: &[&str] = &["other", "unknown", "unlicense-other", "proprietary"];

/// Matches what a repository declared.
///
/// Returns `None` when nothing was declared at all — which is
/// `hub.metadata.absent`, a different state from terms MCF could not read, and
/// the caller's to classify (see [`crate::inspect::terms_are_legible`]).
#[must_use]
pub fn recognize(declared: &str) -> Option<Licence> {
    let normalized = declared.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if MEANS_UNMATCHED.contains(&normalized.as_str()) {
        return Some(Licence::Stated);
    }
    if KNOWN
        .iter()
        .any(|(identifier, _)| *identifier == normalized)
    {
        // Kept as written, normalized only in case: an identifier is a name,
        // and a record that rewrote it would be a record of something the
        // repository did not say.
        return Some(Licence::spdx(normalized));
    }
    Some(Licence::Stated)
}

/// The family an identified licence puts itself in.
///
/// `None` for terms MCF could not match, which is the honest answer: an
/// unmatched licence has no family MCF can name, and inventing one would be the
/// summary this module exists not to write.
#[must_use]
pub fn family(licence: &Licence) -> Option<Family> {
    match licence {
        Licence::Spdx(identifier) => KNOWN
            .iter()
            .find(|(known, _)| *known == identifier.as_str())
            .map(|(_, family)| *family),
        // `Licence` is non-exhaustive, so a state added later lands here and
        // says *no family* rather than guessing one — which is the same answer
        // unmatched terms get, and for the same reason.
        _ => None,
    }
}

/// What a surface says about an artifact's terms, in one line.
///
/// Written here rather than at each surface so that every one of them says the
/// same thing — A6's habit applied to a licence: a surface that renders terms
/// differently from another is two answers to one question.
#[must_use]
pub fn describe(licence: Option<&Licence>) -> String {
    match licence {
        // The sentence is complete without a rule number: what a reader needs
        // is that nothing was declared and that MCF did not fill it in, and
        // both are said. A citation here would send somebody to a document
        // they have never seen to learn what they have just been told.
        None => {
            "licence: unknown — the repository declared none, and MCF has not guessed".to_owned()
        }
        Some(Licence::Stated) => {
            "licence: terms are present and MCF could not identify them — read them before use"
                .to_owned()
        }
        Some(identified @ Licence::Spdx(name)) => match family(identified) {
            Some(family) => format!("licence: {name} ({family})"),
            None => format!("licence: {name}"),
        },
        Some(other) => format!("licence: {other}"),
    }
}

#[cfg(test)]
mod tests;
