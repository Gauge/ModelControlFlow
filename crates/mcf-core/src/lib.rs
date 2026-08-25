//! Types every other MCF crate is built from.
//!
//! `mcf-core` is the bottom of the workspace and depends on no other MCF
//! crate. That is deliberate: §3.16 asks that the substrate enforce the rules
//! rather than that reviewers remember them, and the types this crate holds —
//! [`failure`] (B-003), and `Measurement` (B-005), `Provenance` (B-006) and
//! the time model (B-184) to come — are the ones that make A2, A6, A7 and B37
//! compiler-checked. A crate able to depend on the record store or on the
//! serving path could grow a convenience that lets one of those types be
//! constructed without its conditions.
//!
//! At M0 it carries [`build_identity`], which every record MCF writes names
//! among its conditions (§3.4); [`failure`], which is A2 expressed as a
//! signature; [`measurement`], which is A6 expressed as one; [`time`], which
//! is B37 expressed as two types that do not meet; and [`attested`], which is
//! A7 expressed as a variant; and [`provenance`], which is §3.6 expressed as a
//! field nothing can omit; and [`degradation`], which is A5 expressed as a
//! type with no way back; and [`configuration`], which is B57 expressed as a
//! struct with nowhere to put a machine.

pub mod attested;
pub mod build_identity;
pub mod configuration;
pub mod degradation;
pub mod failure;
pub mod hardware;
pub mod measurement;
pub mod origin;
pub mod provenance;
pub mod self_cost;
pub mod time;
pub mod trial;

pub use failure::{Failure, Result};
pub use measurement::Measurement;
