//! Hosting: the daemon, the engine adapters, and the serving surface.
//!
//! §VI asks that having a model and using a model be one command apart, and §I
//! asks that the thing serving it stay up. A3 is this crate's defining
//! constraint: no failure of a managed thing — an engine that dies mid-token,
//! a runtime that will not spawn — may take MCF down. B8 keeps it bound
//! locally until the user deliberately says otherwise.
//!
//! **What is here now.** The daemon and the control plane it answers on
//! (B-030, B-036). Not inference: MCF has no engine to drive, and a daemon that
//! advertised serving would be claiming what it cannot do (A19). What it does
//! is stay up, recover what it was holding across a restart, answer three
//! questions, and cost nothing while nobody is asking — which is the part §3.13
//! makes central, because a daemon idles far more than it works.

pub mod adapters;
pub mod anatomy;
pub mod bandwidth;
pub mod configured;
pub mod control;
pub mod cost;
pub mod crosscheck;
pub mod daemon;
pub mod declared;
pub mod engines;
mod generation;
pub mod hosting;
pub mod ladder;
pub mod probes;
pub mod projector;
pub mod prompt;
pub mod provisioning;
pub mod served;
pub mod takes;
pub mod turn;
