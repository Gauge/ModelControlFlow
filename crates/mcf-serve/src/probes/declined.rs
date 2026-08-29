//! The modalities MCF does not probe, and why (B-057, D42, §7.24, A7, C7).
//!
//! **B-057's done-when has two halves and this is the second**: *each in-scope
//! modality has a probe; each out-of-scope one is recorded as declined.* A
//! modality MCF simply does not mention is one a reader assumes it checked and
//! found nothing wrong with — which is the same silence A7 forbids about a
//! model's own metadata, pointed at MCF's coverage.
//!
//! **What decides in-scope.** D42's test: *a probe earns its place when a wrong
//! answer to it would corrupt a measurement or a served answer*, and it must be
//! answerable by observation rather than by judgement. A modality fails that on
//! one of two grounds, and the two are kept apart because they lead to different
//! work:
//!
//! * **MCF cannot reach it.** Nothing in this build can put the question, so
//!   there is nothing to observe. The entry names what would be needed.
//! * **The answer would be a judgement.** MCF could put the question and could
//!   not grade the answer without a rater, which is a laboratory's work
//!   (§XIII) and not a probe's.
//!
//! Each entry names the register item that would change the answer, so this
//! list is a statement about *today* rather than a permanent refusal. When that
//! item lands, the entry is removed in the same change — the discipline
//! `mcf_core::touchstone` uses for the same reason (B-380).

/// A modality MCF does not probe, and what it would take to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declined {
    /// What is not probed.
    pub modality: &'static str,
    /// What MCF looked for, and what it found — so that *declined* is an
    /// observation rather than an omission.
    pub looked_for: &'static str,
    /// Why the question cannot be put or cannot be graded here.
    pub because: &'static str,
    /// What would make it answerable.
    pub needs: &'static str,
    /// The register item that would do that.
    pub until: &'static str,
}

/// Every modality MCF declines to probe today.
pub const DECLINED: [Declined; 3] = [
    Declined {
        modality: "vision",
        looked_for: "a vision projector in the artifact — the `clip.*` metadata and the tensors \
                     a multimodal file carries, or the separate projector file its repository \
                     publishes beside the weights",
        because: "MCF's own engine implements text transformers and nothing else, and no \
                  provisioned engine here is driven with an image. A probe that reported *no \
                  vision* would be reporting MCF's own reach as a property of the model, which \
                  is the confusion A21 exists to prevent",
        needs: "an engine that takes an image, and a way to acquire the projector as part of \
                the artifact rather than beside it",
        until: "B-320",
    },
    Declined {
        modality: "reasoning modes",
        looked_for: "the thinking markers a file's own vocabulary carries, which several \
                     families declare and which MCF can already find (F79)",
        because: "whether a model is *reasoning* is a judgement about what it produced; what is \
                  observable is that it emitted its declared markers and how many tokens it \
                  spent inside them. That second question is worth asking and is not this item \
                  — it changes the token budget a turn needs, which makes it a configuring \
                  probe with its own row rather than a modality",
        needs: "its own item: what was measured today is that a model spent a whole \
                two-hundred-token budget before producing an object, ten trials out of ten \
                (F106)",
        until: "B-054",
    },
    Declined {
        modality: "multilingual fluency",
        looked_for: "nothing — the question is asked of a model's answers rather than of its \
                     file, and grading an answer needs a rater",
        because: "what a language *costs* this vocabulary is answerable without an engine and \
                  is probed (`language-cost`); how well the model speaks it is a graded task. \
                  Reporting the first as the second is exactly the reading F81 was written to \
                  prevent",
        needs: "a laboratory with a graded task in each language, and the workload to grade \
                against",
        until: "B-110",
    },
];
