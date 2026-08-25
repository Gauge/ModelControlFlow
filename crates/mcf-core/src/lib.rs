//! Types every other MCF crate is built from.
//!
//! `mcf-core` is the bottom of the workspace and depends on no other MCF
//! crate. That is deliberate: §3.16 asks that the substrate enforce the rules
//! rather than that reviewers remember them, and the types this crate will
//! hold — the failure type (B-003), `Measurement` (B-005), `Provenance`
//! (B-006) and the time model (B-184) — are the ones that make A2, A6, A7 and
//! B37 compiler-checked. A crate able to depend on the record store or on the
//! serving path could grow a convenience that lets one of those types be
//! constructed without its conditions.
//!
//! At M0 it carries only [`build_identity`], which every record MCF writes
//! names among its conditions (§3.4).

pub mod build_identity;
