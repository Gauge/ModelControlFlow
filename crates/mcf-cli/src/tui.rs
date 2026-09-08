use crate::Response;

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
