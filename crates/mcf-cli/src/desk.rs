//! `mcf desk`: MCF in a window (A22, B-072, B-405).
//!
//! **A third surface, and a client of the same control plane.** It draws the
//! console's own screens at another scale, so a layout changed once changes
//! both; `mcf_desk::ACTIONS` names every action it offers and the parity check
//! reads that table.
//!
//! **It refuses where there is no window library rather than failing to
//! build.** A crate that would not compile without a provisioned component
//! would break the build for everybody who has not provisioned one, so the
//! refusal happens here, in words, with the command that fixes it.

use crate::Response;

/// Opens the window and stays there until it is closed.
pub(crate) fn run() -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine\n  \
                   the window is a client of a running daemon, and there is no path to one"
                .to_owned(),
            served: false,
        };
    };
    match mcf_desk::run(socket) {
        Ok(()) => Response {
            text: String::new(),
            served: true,
        },
        Err(why) => Response {
            text: format!(
                "mcf: the window did not open\n  {why}\n  \
                 `mcf tui` is the same screens with no display attached (A22)"
            ),
            served: false,
        },
    }
}
