//! Acquisition: resolving, fetching and pinning artifacts, provenance intact.
//!
//! §III commits MCF to accepting any reference without special-casing and
//! reaching a defined, actionable outcome for every one — B7 is careful that
//! this is a commitment to *no unhandled outcomes*, not to *no unsuccessful
//! ones*. A15 keeps repository code from ever running implicitly, and A7 keeps
//! an unread licence or an absent checksum recorded as unknown rather than
//! guessed.
//!
//! **What is here, and what it deliberately is not.** [`reference`](mod@reference) reads what a
//! user named — every way of writing it, and a defined outcome for every string
//! — without asking anybody anything. Resolving a reference against the hub,
//! fetching bytes and verifying them is B-021 onward and needs the network,
//! which M0 explicitly does not have; keeping the two apart is what lets the
//! laboratory exercise every branch of the first without a hub (D26).

pub mod client;
pub mod credentials;
pub mod decay;
pub mod fetch;
pub mod fitment;
pub mod http;
pub mod inspect;
pub mod licence;
pub mod recommendation;
pub mod reference;
pub mod source;
pub mod store;
pub mod wire;
