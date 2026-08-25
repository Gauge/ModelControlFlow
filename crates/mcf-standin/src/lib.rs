//! MCF's own implementation of inference (D31, B-360).
//!
//! **Why this exists, in the order that justifies it.** A19 forbids believing
//! published numbers from software that cannot demonstrate it computes what it
//! claims, and for inference the only available demonstration is a second
//! implementation that agrees. Coverage — a model no vendored engine will run
//! still runs, marked — is what that also buys, and is not what pays for it
//! (B23).
//!
//! **It cannot report a speed, and that is structural.** B65 and
//! [`mcf_core::engine`] make a timing a method on `Run<Vendored>` alone, so a
//! throughput figure from this code is not a rule somebody might break but a
//! program that does not compile. D32 settles the other side: MCF delegates the
//! kernels and owns the wrapper, and F8 measures why — the best safe portable
//! Rust here is twenty-five to fifty times a specialist's single core.
//!
//! **Deliberately slow, and written to be read.** No SIMD, no fusion, no
//! threading, no accelerator path, no memory-layout work. Its maintenance is
//! proportional to *architectures* rather than to hardware, which is exactly the
//! treadmill §7.4 refused. Where a choice is between fast and legible, this
//! crate takes legible, and says so at the site.
//!
//! **A separate crate, because the prohibition is a boundary.** `mcf-serve`
//! holds engine *adapters*; this is an engine. Keeping it out of the serving
//! and measurement crates means the only way a result from it reaches a surface
//! is through the handle types that carry the mark (A5).
//!
//! ## What is here
//!
//! | Module | Holds |
//! |---|---|
//! | [`gguf`] | The reader for the format §XII's reference model is published in |
//!
//! What is not here yet: dequantization per scheme, the transformer operations,
//! and sampling. B-360 names them and they arrive in that order, because a
//! format nobody can read is a model nobody can run.

pub mod gguf;
