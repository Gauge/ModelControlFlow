//! Workspace-shape checks.
//!
//! B-001 states the crate split and the direction of its dependency edges.
//! B16 says to prefer the machine-checked form of every rule, so the split is
//! a test rather than a paragraph: an edge that would invert the layering
//! fails the gating suite instead of surviving until somebody notices it in
//! review.
//!
//! This crate ships nothing (`publish = false`) and is not a dependency of any
//! MCF crate. It holds the smallest manifest reader the checks need, because
//! B15 admits weight only against a stated cost and a general TOML parser is
//! weight the gating suite does not need to carry — the manifests it reads are
//! written in this repository and hold to one shape.

pub mod manifest;
pub mod workspace;
