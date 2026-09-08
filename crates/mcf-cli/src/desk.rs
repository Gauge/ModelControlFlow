use crate::Response;

pub(crate) fn run() -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine\n  \
                   the window is a client of a running daemon, and there is no path to one"
                .to_owned(),
            served: false,
        };
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        };
    }

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
