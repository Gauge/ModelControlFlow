//! The laboratory: simulated clock, injected faults, replayable scenarios.
//!
//! §3.17 and D5 place MCF's confidence here rather than in ambient
//! observation, which is why B4 can refuse telemetry without costing anything
//! §II needs. B27 holds the laboratory to every rule that governs the rest of
//! the workspace, because everything else is believed on its authority.
//!
//! Two constraints are structural rather than aspirational. A11: no
//! performance number originates here, and B37 makes the simulated clock a
//! different type from the monotonic one so that a simulated duration cannot
//! become a throughput figure. A13: the fault catalogue and
//! [the taxonomy] are the same list, cross-checked in CI (B-010).
//!
//! Empty at M0 beyond this statement of what it is for; B-009 brings the
//! harness, gated on DEC-021.
//!
//! [the taxonomy]: ../../../doc/taxonomy.md
