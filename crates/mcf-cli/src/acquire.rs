//! `mcf offered` and `mcf acquire`: the two things the window's *Add a model*
//! screen does, without the window.
//!
//! **Why these exist beside `mcf pull`.** `pull` does the same work in this
//! process, and needs no daemon; these ask the daemon to do it. That is not
//! two implementations — both end at `mcf_hub::acquisition::one` — but it is
//! two *paths*, and the daemon's is the one the window drives. A22 forbids a
//! capability reachable only through a client, and a path nothing headless
//! exercises is a path the laboratory cannot test (B-072, A22).
//!
//! **They print what the daemon said.** No summarising, no rewording of a
//! refusal: a second opinion about what happened is not something a client is
//! for (A2).

use crate::say::refused_because;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

/// What a repository publishes, and which of it will run here.
pub(crate) fn offered(reference: &str, from: Option<&str>) -> Response {
    ask(
        &Request::Offered {
            reference: reference.to_owned(),
            from: from.map(str::to_owned),
        },
        &published,
    )
}

/// Fetches one published file into this machine's store.
pub(crate) fn acquire(reference: &str, file: &str, from: Option<&str>) -> Response {
    ask(
        &Request::Acquire {
            reference: reference.to_owned(),
            file: file.to_owned(),
            from: from.map(str::to_owned),
        },
        &arriving,
    )
}

/// Sends one request and renders every line it answers with.
fn ask(request: &Request, render: &dyn Fn(&Value) -> Vec<String>) -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine".to_owned(),
            served: false,
        };
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        };
    }
    let Ok(mut connection) = UnixStream::connect(&socket) else {
        return Response {
            text: "mcf: MCF is not answering".to_owned(),
            served: false,
        };
    };
    // No read deadline: a transfer is as long as the file and the network make
    // it, and a timeout here would report a working download as a broken
    // daemon.
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return Response {
            text: "mcf: the request could not be sent".to_owned(),
            served: false,
        };
    }

    let mut lines = Vec::new();
    let mut served = true;
    for read in BufReader::new(&connection).lines() {
        let Ok(read) = read else { break };
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served {
            served = false;
            lines.push(format!("mcf: refused\n  {}", refused_because(&answer.body)));
            break;
        }
        lines.extend(render(&answer.body));
        if matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            break;
        }
    }
    Response {
        text: lines.join("\n"),
        served,
    }
}

/// A listing, one row a published file.
fn published(body: &Value) -> Vec<String> {
    let mut lines = vec![format!(
        "{} at {}",
        body.get("repository")
            .and_then(Value::as_text)
            .unwrap_or("?"),
        body.get("revision")
            .and_then(Value::as_text)
            .unwrap_or("an unstated revision")
    )];
    // Before the files, because it is the thing a person may need to decide
    // not to download at all (B-023).
    if let Some(terms) = body.get("terms").and_then(Value::as_text) {
        lines.push(format!("  {terms}"));
    }
    // A7 and A19: where MCF could not judge whether these would run, it says
    // so once rather than leaving every row silently unjudged.
    if let Some(why) = body.get("no_plan").and_then(Value::as_text) {
        lines.push(format!(
            "  MCF cannot say which of these would run here: {why}"
        ));
    }
    // Where the shape came from is a condition of every verdict below it: a
    // configuration can say which blocks are full-attention and a header
    // cannot, so a hybrid model judged from a header has its cache overstated
    // — which errs toward refusing something that would fit (A6, F16).
    // A21: declared, verified, unknown. Every verdict below rests on a number
    // the repository supplied — MCF has not fetched the weights, and saying
    // *fits* about a claim without saying it is one would make a plan read as
    // a finding.
    if let Some(from) = body.get("shape_from").and_then(Value::as_text) {
        lines.push(format!("  these rest on {from}"));
        lines
            .push("  the arithmetic is MCF's; the shape it is over is the repository's".to_owned());
    }
    for file in body.get("files").and_then(Value::as_list).unwrap_or(&[]) {
        let name = file.get("file").and_then(Value::as_text).unwrap_or("?");
        let bytes = file.get("bytes").and_then(Value::as_integer).unwrap_or(0);
        let verdict = file
            .get("why")
            .and_then(Value::as_text)
            .map_or_else(String::new, |why| format!(" — {why}"));
        lines.push(format!("  {name} — {bytes} bytes{verdict}"));
    }
    lines
}

/// A transfer, one line each time it says how far it has got.
fn arriving(body: &Value) -> Vec<String> {
    let file = body
        .get("acquiring")
        .and_then(Value::as_text)
        .unwrap_or("?");
    if matches!(body.get("done"), Some(Value::Bool(true))) {
        let path = body.get("path").and_then(Value::as_text).unwrap_or("?");
        let mut lines = vec![format!("{file} is here: {path}")];
        match body.get("recorded").and_then(Value::as_text) {
            Some(where_) => lines.push(format!("  written down in {where_}")),
            // A24: an acquisition MCF cannot account for is worse than one it
            // did not make, so a record that would not write is said out loud.
            None => lines.push("  MCF could not write this acquisition to its record".to_owned()),
        }
        return lines;
    }
    let arrived = body.get("arrived").and_then(Value::as_integer);
    let total = body.get("bytes").and_then(Value::as_integer);
    match (body.get("doing").and_then(Value::as_text), arrived, total) {
        (Some("checking"), _, _) => {
            vec![format!(
                "  {file}: checking that what arrived is what was published"
            )]
        }
        (_, Some(arrived), Some(total)) => {
            vec![format!("  {file}: {arrived} of {total} bytes")]
        }
        _ => vec![format!("  {file}: starting")],
    }
}

#[cfg(test)]
mod tests {
    use super::published;
    use mcf_record::json::Value;

    /// Terms are shown before the files, not after the download.
    ///
    /// **Downloading is a use.** B-023 asks that a licence be surfaced before
    /// one, and a person who learns what a model's terms are once it is on
    /// their disk has learned it too late to decide. The Add-a-model flow
    /// showed a list of files and a button and no terms anywhere.
    #[test]
    fn the_terms_come_before_the_files() {
        let answered = Value::map([
            ("repository", Value::text("owner/repository")),
            ("terms", Value::text("licence: apache-2.0 (permissive)")),
            (
                "files",
                Value::List(vec![Value::map([
                    ("file", Value::text("a-model.gguf")),
                    ("bytes", Value::Integer(1_000)),
                ])]),
            ),
        ]);
        let lines = published(&answered);
        let terms = lines
            .iter()
            .position(|line| line.contains("apache-2.0"))
            .expect("the terms are shown");
        let file = lines
            .iter()
            .position(|line| line.contains("a-model.gguf"))
            .expect("the files are shown");
        assert!(
            terms < file,
            "the terms are printed after the files, so a reader meets the button first"
        );
    }

    /// A repository that declares nothing says so, and MCF does not fill it in.
    ///
    /// A7: the three states stay distinct and none of them is a default. A
    /// plausible guess at a licence is the one answer worse than no answer.
    #[test]
    fn nothing_declared_is_said_and_never_guessed() {
        let answered = Value::map([
            ("repository", Value::text("owner/repository")),
            (
                "terms",
                Value::text(
                    "licence: unknown — the repository declared none, and MCF has not guessed",
                ),
            ),
            ("files", Value::List(Vec::new())),
        ]);
        let lines = published(&answered).join("\n");
        assert!(lines.contains("unknown"), "{lines}");
        assert!(lines.contains("has not guessed"), "{lines}");
        // And no licence name is invented anywhere in it.
        for invented in ["apache", "mit", "gpl", "permissive"] {
            assert!(
                !lines.to_lowercase().contains(invented),
                "a licence was named for a repository that declared none: {lines}"
            );
        }
    }

    /// A listing with no terms at all still lists its files.
    ///
    /// An older daemon answers without the field, and a client that refused to
    /// render anything would turn a missing line into a missing screen.
    #[test]
    fn a_listing_without_terms_still_lists() {
        let answered = Value::map([
            ("repository", Value::text("owner/repository")),
            (
                "files",
                Value::List(vec![Value::map([("file", Value::text("a-model.gguf"))])]),
            ),
        ]);
        let lines = published(&answered).join("\n");
        assert!(lines.contains("a-model.gguf"), "{lines}");
    }
}
