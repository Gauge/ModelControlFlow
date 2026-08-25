//! Hosting: the daemon, the engine adapters, and the serving surface.
//!
//! §VI asks that having a model and using a model be one command apart, and §I
//! asks that the thing serving it stay up. A3 is this crate's defining
//! constraint: no failure of a managed thing — an engine that dies mid-token,
//! a runtime that will not spawn — may take MCF down. B8 keeps it bound
//! locally until the user deliberately says otherwise.
//!
//! Empty at M0 beyond this statement of what it is for: M0 explicitly contains
//! no inference. B-030 onward fill it at M2.
