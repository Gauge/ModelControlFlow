//! Measurement, and the laboratories that produce it.
//!
//! This crate depends on `mcf-serve` because a measurement of a model is taken
//! through the thing that hosts it; the reverse edge does not exist, so no
//! serving path can acquire a dependency on the benchmark harness. A18 keeps
//! the two disciplines apart: tests gate correctness and are green,
//! benchmarks produce measurements and have no pass condition.
//!
//! B35 and B49 divide the work by class — timing-class runs open an exclusive
//! window, behaviour-class runs yield and record the contention they ran under
//! — and that division is expected to be visible in this crate's types rather
//! than in its documentation.
//!
//! M5 has begun to fill it: [`enough`] holds the stopping condition a
//! comparison carries instead of a repeat count, because the count turned out
//! not to be a constant (F53).
//!
//! **Not an instrument:** it names the modules beneath it.

/// The question MCF asks when nobody names one (B-160, B42, §6.37).
///
/// **A benchmark needs something to time, and who chose it decides whether the
/// result can be shared.** The rule MCF already applies to laboratories is that
/// one ships a standard workload and accepts a replacement: results from the
/// standard travel, results from a replacement stay where they were taken,
/// because nobody else has the replacement and so nobody else can interpret the
/// number. `mcf bench` had no standard, so every result it had ever produced
/// fell into *replacement* by default rather than by anybody's judgement
/// (F115).
///
/// **Chosen by measurement, across every vocabulary this machine holds.** A
/// standard question is only standard if it is the same amount of work for
/// every model asked it, and it is not: the same English sentence costs 16
/// tokens on one vocabulary and 52 on another once other languages are involved
/// (F110), and even in English four candidates spread differently. Measured
/// over 27 vocabularies:
///
/// | candidate | tokens | spread |
/// |---|---|---|
/// | *The quick brown fox…* | 16–19 | 19% |
/// | *Write a short paragraph…* | 15–20 | 33% |
/// | **this one** | **20–22** | **10%** |
/// | *Describe in three sentences what happens…* | 16–18 | 12% |
///
/// Two tokens of spread across every family MCF can read, which is as close to
/// *the same work* as a question in words gets.
///
/// **Why this shape.** Plain English, because that is where vocabularies agree.
/// No markers, no punctuation a tokenizer treats specially, and no numerals. An
/// instruction rather than a fragment, because that is what people benchmark. A
/// bounded answer — *three sentences* — because the run pins a token budget and
/// a question with no natural end measures the budget instead.
///
/// **It is fixed for life.** A standard question that changed would silently
/// make yesterday's shared results incomparable with today's, which is the one
/// thing a standard exists to prevent. Changing it means a new name beside this
/// one, never an edit.
pub const STANDARD_QUESTION: &str = "In three sentences, describe what happens to a river between its source in the mountains \
     and the sea.";

pub mod compare;
pub mod enough;
pub mod planned;
pub mod project;
pub mod record;
pub mod seeds;
pub mod warmth;
