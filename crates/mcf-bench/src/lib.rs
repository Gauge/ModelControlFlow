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
//! Empty at M0 beyond this statement of what it is for; M5 and M6 fill it.
