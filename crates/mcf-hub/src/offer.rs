//! What a repository publishes, and which of it will run on this machine.
//!
//! **The question behind naming a repository and no file.** A person who types
//! a repository is not asking what exists — they are asking which of these is
//! for their machine (PR3, B-213). That judgement is arithmetic over the
//! published sizes and the model's own shape, and it is made here.
//!
//! **It lives here because two surfaces ask it.** The command line asks when
//! somebody runs `mcf pull` with no file; the daemon asks on behalf of the
//! window. A judgement made twice is a judgement that can differ between the
//! places it is made, and *this will run* differing between two of MCF's own
//! surfaces would be the worst kind of disagreement (B-072).

use crate::client::Hub;
use crate::fitment::{self, Requirement, Shape, Verdict};
use crate::source::Listing;
use mcf_core::measurement::Bytes;

/// How much context a plan is made at when nobody has said.
///
/// Four thousand and ninety-six tokens: the length most engines default to, and
/// a number stated here rather than buried, because the answer *this fits*
/// means nothing without the context it fits at (A6, §3.4).
pub const PLANNING_CONTEXT: u64 = 4096;

/// How many bytes one cached element takes.
///
/// Two, for the half-precision caches engines use by default. A parameter of
/// the run rather than a fact about the model, and the reason [`Shape`] takes
/// it rather than reading it.
const CACHE_ELEMENT: u64 = 2;

/// What MCF judged about a repository's variants, kept rather than rendered.
///
/// The verdicts themselves rather than the sentences they print, because the
/// same plan is written to the record (B-086) and shown to an operator, and a
/// plan that existed only as prose could be written to one of those and not
/// the other.
#[derive(Debug, Clone)]
pub struct Plan {
    /// How much memory this machine had free when the plan was made — a
    /// condition of every verdict in it (A6).
    pub available: Bytes,
    /// The context the verdicts are at.
    pub context: u64,
    /// One verdict per published file MCF can read.
    pub verdicts: Vec<(String, Verdict)>,
}

/// Which of the variants a repository publishes will run on this machine.
///
/// # Errors
///
/// Carries *why there is no plan*, in a sentence an operator can act on. A7
/// asks that an unknown stay unknown; A2 asks that the reason not be
/// swallowed. Before this said which, a repository whose configuration MCF
/// could not parse and one that publishes none read identically — and the
/// first is a defect in MCF while the second is a fact about the repository
/// ([findings.md](../../../doc/findings.md) F16).
/// **The machine is not read here, it is handed in.** B4 forbids MCF sampling
/// hardware from anywhere that runs without being asked, and this is called
/// from the daemon as well as from the command line. So how much memory is
/// free is a *parameter*: the command line reads the machine because somebody
/// ran a command, and the daemon passes what it already knows. A plan is
/// arithmetic over a number somebody else established, which is also why the
/// number it used is in the answer (A6).
pub fn plan_for(hub: &Hub, listing: &Listing, available: Bytes) -> Result<Plan, String> {
    let configuration = match hub.configuration(listing) {
        Ok(Some(configuration)) => configuration,
        Ok(None) => {
            return Err(
                "this repository publishes no configuration, and a plan needs one".to_owned(),
            );
        }
        Err(failure) => {
            return Err(format!("its configuration could not be read — {failure}"));
        }
    };
    let Some(shape) = Shape::from_configuration(&configuration, CACHE_ELEMENT) else {
        return Err(
            "its configuration does not say how many blocks, key/value heads and head \
             dimensions the model has, and MCF will not guess at a shape (A7)"
                .to_owned(),
        );
    };
    let requirements: Vec<Requirement> = listing
        .entries
        .iter()
        // Case-insensitively, because a repository's file names are its own:
        // `.GGUF` is the same format and a plan that skipped it would leave a
        // variant out of the list without saying so (A1).
        .filter(|entry| {
            std::path::Path::new(&entry.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
        })
        .map(|entry| Requirement {
            name: entry.path.clone(),
            weights: Bytes(entry.size),
            shape,
        })
        .collect();
    if requirements.is_empty() {
        return Err("this repository publishes nothing in a format MCF reads".to_owned());
    }

    let verdicts = fitment::plan(&requirements, PLANNING_CONTEXT, available)
        .map_err(|failure| format!("the arithmetic would not add up — {failure}"))?;
    Ok(Plan {
        available,
        context: PLANNING_CONTEXT,
        verdicts,
    })
}
