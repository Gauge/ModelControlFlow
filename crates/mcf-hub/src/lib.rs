//! Acquisition: resolving, fetching and pinning artifacts, provenance intact.
//!
//! §III commits MCF to accepting any reference without special-casing and
//! reaching a defined, actionable outcome for every one — B7 is careful that
//! this is a commitment to *no unhandled outcomes*, not to *no unsuccessful
//! ones*. A15 keeps repository code from ever running implicitly, and A7 keeps
//! an unread licence or an absent checksum recorded as unknown rather than
//! guessed.
//!
//! Empty at M0 beyond this statement of what it is for: M0 explicitly contains
//! no network fetch. B-020 onward fill it at M1.
