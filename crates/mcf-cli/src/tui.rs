//! `mcf tui`: MCF as a terminal application (A22, B-072).
//!
//! **A second surface, and a client of the same control plane.** Everything it
//! can do is a request this binary already sends; `mcf_tui::ACTIONS` names
//! which, and the parity check reads that table.
//!
//! **It refuses where there is no terminal rather than drawing at nothing.**
//! Piped output is not a fault — it is a fact about where MCF was pointed — so
//! the refusal says what to do instead (A7, A2).

use crate::Response;

/// Runs the terminal application until the operator quits.
pub(crate) fn run() -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine\n  \
                   the terminal application is a client of a running daemon, and there is \
                   no path to one"
                .to_owned(),
            served: false,
        };
    };

    // A person opening this expects the tools to be working. Starting a daemon
    // is MCF's job, not something to be reported to them as their problem.
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        };
    }

    match mcf_tui::run(socket) {
        Ok(()) => Response {
            text: String::new(),
            served: true,
        },
        Err(why) => Response {
            text: format!(
                "mcf: the terminal application did not start\n  {why}\n  \
                 `mcf status` and `mcf list` answer the same questions with no display \
                 attached (A22)"
            ),
            served: false,
        },
    }
}
