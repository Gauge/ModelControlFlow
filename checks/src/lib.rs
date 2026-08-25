//! The checks, and the machinery the tiers that are not `cargo test` need.
//!
//! B16 says to prefer the machine-checked form of every rule, and this crate is
//! where the rules that are about the *repository* rather than about a value
//! are enforced: the crate split and its dependency direction (B-001), the
//! taxonomy in the code being the taxonomy in the document (B-003), the format
//! contract every document in `doc/` holds to (B-041), and the tier register
//! that keeps the suite honest about what it runs (B-191).
//!
//! Four modules are not checks but the machinery the tiers need — [`property`]
//! for generated inputs, [`fuzz`] for damaging known-good ones, [`scratch`] for
//! somewhere to build a real journal, and [`tiers`] for the declaration itself.
//! They live here for the reason this crate exists: it ships nothing
//! (`publish = false`) and no MCF crate depends on it, so a convenience written
//! for the suite has no way to reach the binary.
//!
//! It holds the smallest manifest reader the checks need, because B15 admits
//! weight only against a stated cost and a general TOML parser is weight the
//! gating tier does not need to carry — the manifests it reads are written in
//! this repository and hold to one shape.

pub mod document;
pub mod fuzz;
pub mod manifest;
pub mod property;
pub mod scratch;
pub mod taxonomy;
pub mod tiers;
pub mod workspace;
