use mcf_serve::control::Request;

use crate::Response;

const SHOWN: usize = 12;

pub(crate) fn run(last: Option<usize>) -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: there is nowhere to look for a daemon — neither XDG_RUNTIME_DIR, \
                   XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    let answer = match crate::serve::ask(
        &socket,
        &Request::Failures {
            last: last.unwrap_or(SHOWN),
        },
    ) {
        Ok(answer) if answer.served => answer,
        Ok(answer) => {
            return Response {
                text: crate::say::refused_because(&answer.body),
                served: false,
            };
        }
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let listed = answer
        .body
        .get("failures")
        .and_then(mcf_record::json::Value::as_list)
        .unwrap_or_default();
    let in_record = answer
        .body
        .get("in_record")
        .and_then(mcf_record::json::Value::as_integer)
        .unwrap_or(0);
    let mut lines = vec![match (in_record, listed.len()) {
        (0, _) => "no classified failure is in the record".to_owned(),
        (n, shown) if usize::try_from(n).is_ok_and(|n| n <= shown) => {
            format!("{n} classified failure(s) in the record, newest first")
        }
        (n, shown) => format!("the newest {shown} of {n} classified failures in the record"),
    }];
    for entry in listed {
        let fault = mcf_desk::fault_from(entry);
        lines.push(String::new());
        for (at, line) in mcf_desk::fault_lines(&fault).iter().enumerate() {
            if at == 0 {
                lines.push(format!("{}  {line}", fault.at));
            } else {
                lines.push(format!("  {line}"));
            }
        }
    }
    Response {
        text: lines.join("\n"),
        served: true,
    }
}
