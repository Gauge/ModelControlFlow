use std::path::Path;

use crate::Response;

pub(crate) fn run(model: &str, prompt: &str) -> Response {
    let path = match crate::bench::located(model) {
        Ok(path) => path,
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    match segmented(&path, prompt) {
        Ok(text) => Response { text, served: true },
        Err(text) => Response {
            text,
            served: false,
        },
    }
}

struct Fragment {
    identifier: usize,
    text: String,
    byte: bool,
}

fn added_by(before: &str, now: &str) -> String {
    let common = now
        .as_bytes()
        .iter()
        .zip(before.as_bytes())
        .take_while(|(one, other)| one == other)
        .count();
    String::from_utf8_lossy(now.as_bytes().get(common..).unwrap_or_default()).into_owned()
}

fn segmented(path: &Path, prompt: &str) -> Result<String, String> {
    let bytes = crate::bench::read_prefix(path)
        .ok_or_else(|| format!("mcf: {} could not be read", path.display()))?;
    let file = mcf_standin::gguf::parse(&bytes).map_err(|failure| {
        crate::say::refusal("the file could not be read as a model", &failure)
    })?;
    let vocabulary = match mcf_standin::tokenizer::Vocabulary::read(&file) {
        Ok(vocabulary) => vocabulary,
        Err(failure) => return through_the_daemon(path, prompt, &file, &failure),
    };
    let identifiers = vocabulary.encode(prompt, true).map_err(|failure| {
        crate::say::refusal("this vocabulary cannot represent that text", &failure)
    })?;

    let mut fragments = Vec::new();
    let mut before = String::new();
    for at in 1..=identifiers.len() {
        let Some(taken) = identifiers.get(..at) else {
            break;
        };
        let now = vocabulary.decode(taken);
        let added = added_by(&before, &now);
        before = now;
        let identifier = identifiers.get(at.saturating_sub(1)).copied().unwrap_or(0);
        fragments.push(Fragment {
            identifier,
            text: added,
            byte: vocabulary.is_byte(identifier),
        });
    }

    Ok(render(
        prompt,
        &fragments,
        &before,
        &vocabulary,
        mcf_serve::probes::declared_context(&file),
        crate::history::probed_context(path),
    ))
}

fn through_the_daemon(
    path: &Path,
    prompt: &str,
    file: &mcf_standin::gguf::Model,
    refusal: &mcf_core::Failure,
) -> Result<String, String> {
    let read = read_by_the_daemon(path, prompt, true).map_err(|why| {
        format!(
            "{}\n  the engine that generates for this model could read it, through the \
             daemon: {why}",
            crate::say::refusal("this file carries no vocabulary MCF can read", refusal)
        )
    })?;
    let listed = mcf_standin::tokenizer::Tokens::read(file).ok();
    let fragments: Vec<Fragment> = read
        .pieces
        .into_iter()
        .map(|(identifier, piece)| Fragment {
            identifier,
            byte: is_a_byte_piece(&piece),
            text: piece,
        })
        .collect();

    let mut lines = vec![
        format!(
            "{} token(s) for {} character(s) of text{}, read by {}",
            fragments.len(),
            prompt.chars().count(),
            listed.as_ref().map_or_else(String::new, |tokens| format!(
                ", on a vocabulary of {} token(s)",
                tokens.len()
            )),
            read.by
        ),
        format!(
            "  MCF's own tokenizer does not read this file — {} — so this is the engine's \
             reading and not MCF's, which is the reading a generation is charged (B-442)",
            refusal.detail()
        ),
        String::new(),
    ];
    for (at, fragment) in fragments.iter().enumerate() {
        let shown = if fragment.text.is_empty() {
            "(nothing visible)".to_owned()
        } else {
            format!("{:?}", fragment.text)
        };
        lines.push(format!(
            "  #{at:<4} {:>7}  {shown}{}",
            fragment.identifier,
            if fragment.byte {
                "   ← a raw byte: this vocabulary has no piece for that text"
            } else {
                ""
            }
        ));
    }
    lines.push(String::new());
    lines.push(what_it_spends(
        fragments.len(),
        mcf_serve::probes::declared_context(file),
        crate::history::probed_context(path),
    ));
    lines.push(String::new());
    lines.push(whole_or_shattered(prompt, &fragments));
    if let Some(tokens) = listed {
        let has_token = |marker: &str| tokens.has_token(marker);
        let ordinary = |marker: &str| {
            read_by_the_daemon(path, marker, false).map_or_else(
                |why| format!("could not be read: {why}"),
                |read| format!("{} ordinary token(s)", read.pieces.len()),
            )
        };
        lines.extend(marker_fidelity(prompt, &has_token, &ordinary));
    }
    Ok(lines.join("\n"))
}

struct Reading {
    pieces: Vec<(usize, String)>,
    by: String,
}

fn read_by_the_daemon(path: &Path, text: &str, beginning: bool) -> Result<Reading, String> {
    let body = crate::hosting::ask(&mcf_serve::control::Request::Tokenize {
        model: path.display().to_string(),
        text: text.to_owned(),
        engine: None,
        beginning,
    })?;
    let pieces = body
        .get("read")
        .and_then(mcf_record::json::Value::as_list)
        .ok_or_else(|| "the daemon's answer carried no reading".to_owned())?
        .iter()
        .map(|token| {
            (
                token
                    .get("id")
                    .and_then(mcf_record::json::Value::as_integer)
                    .and_then(|held| usize::try_from(held).ok())
                    .unwrap_or(0),
                token
                    .get("piece")
                    .and_then(mcf_record::json::Value::as_text)
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect();
    let by = body
        .get("read_by")
        .and_then(mcf_record::json::Value::as_text)
        .map_or_else(
            || "a tokenizer the daemon did not name".to_owned(),
            str::to_owned,
        );
    Ok(Reading { pieces, by })
}

fn is_a_byte_piece(piece: &str) -> bool {
    piece.len() == 6
        && piece.starts_with("<0x")
        && piece.ends_with('>')
        && piece
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|held| held.is_ascii_hexdigit()))
}

fn render(
    prompt: &str,
    fragments: &[Fragment],
    decoded: &str,
    vocabulary: &mcf_standin::tokenizer::Vocabulary,
    context: Option<usize>,
    measured: Option<usize>,
) -> String {
    let mut lines = vec![
        format!(
            "{} token(s) for {} character(s) of text, on a vocabulary of {} token(s)",
            fragments.len(),
            prompt.chars().count(),
            vocabulary.len()
        ),
        String::new(),
    ];
    for (at, fragment) in fragments.iter().enumerate() {
        let spelling = vocabulary
            .token(fragment.identifier)
            .unwrap_or("<not in this vocabulary>");
        let shown = if fragment.text.is_empty() {
            format!("(nothing visible; spelled {spelling})")
        } else {
            format!("{:?}", fragment.text)
        };
        lines.push(format!(
            "  #{at:<4} {:>7}  {shown}{}",
            fragment.identifier,
            if fragment.byte {
                "   ← a raw byte: this vocabulary has no piece for that text"
            } else {
                ""
            }
        ));
    }
    lines.push(String::new());
    lines.push(what_it_spends(fragments.len(), context, measured));
    lines.push(String::new());
    lines.push(whole_or_shattered(prompt, fragments));
    let has_token = |marker: &str| vocabulary.has_token(marker);
    let ordinary = |marker: &str| {
        vocabulary.encode(marker, false).map_or_else(
            |_| "no tokens at all".to_owned(),
            |held| format!("{} ordinary token(s)", held.len()),
        )
    };
    lines.extend(marker_fidelity(prompt, &has_token, &ordinary));
    if decoded != prompt {
        lines.push(String::new());
        lines.push(format!(
            "What came back is not what was typed: {decoded:?} against {prompt:?}. A \
             segmentation is not a round trip — a leading space is a convention this \
             vocabulary was trained with, and a beginning-of-text token is one the file asks \
             for (F19, A1)."
        ));
    }
    lines.join("\n")
}

fn marker_fidelity(
    prompt: &str,
    has_token: &dyn Fn(&str) -> bool,
    ordinary: &dyn Fn(&str) -> String,
) -> Vec<String> {
    let found = marker_shaped(prompt);
    if found.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![String::new(), "Markers written into the prompt:".to_owned()];
    for marker in &found {
        let spelled = ordinary(marker);
        lines.push(if has_token(marker) {
            format!(
                "  {marker:?} → {spelled}. This vocabulary HAS a token spelled exactly that, \
                 and typed text still does not become it: nothing a person writes can produce \
                 a control token (D46, F26)."
            )
        } else {
            format!(
                "  {marker:?} → {spelled}. This vocabulary has no such token at all, so it is \
                 ordinary text here however it is spelled."
            )
        });
    }
    lines.push(
        "  A template whose markers do not survive is a template that does not do what it \
         looks like it does (F37, B-383)."
            .to_owned(),
    );
    lines
}

fn marker_shaped(prompt: &str) -> Vec<String> {
    const LONGEST: usize = 48;

    let mut found: Vec<String> = Vec::new();
    for (open, close) in [('<', '>'), ('[', ']')] {
        let mut rest = prompt;
        while let Some(at) = rest.find(open) {
            let after = rest.get(at.saturating_add(1)..).unwrap_or_default();
            let taken = after
                .char_indices()
                .take_while(|(offset, letter)| {
                    *offset < LONGEST && !letter.is_whitespace() && *letter != open
                })
                .map(|(offset, letter)| offset.saturating_add(letter.len_utf8()))
                .last()
                .unwrap_or(0);
            let window = after.get(..taken).unwrap_or_default();
            if let Some(end) = window.find(close) {
                let marker = rest
                    .get(at..at.saturating_add(end).saturating_add(2))
                    .unwrap_or_default();
                if !marker.is_empty() && !found.iter().any(|held| held == marker) {
                    found.push(marker.to_owned());
                }
            }
            rest = after;
        }
    }
    found
}

fn what_it_spends(tokens: usize, context: Option<usize>, measured: Option<usize>) -> String {
    if let Some(accepted) = measured.filter(|held| *held > 0) {
        let declared = context.map_or_else(
            || "the file declares none".to_owned(),
            |held| format!("the file declares {held}"),
        );
        let share = tokens
            .saturating_mul(1_000)
            .checked_div(accepted)
            .unwrap_or(0);
        return format!(
            "{tokens} token(s) of prompt against a MEASURED context of {accepted} token(s) — \
             {}.{}% of it, and {declared}. Measured is what `mcf probe` found this engine on \
             this machine actually accepts, which is the number a prompt has to fit (B-055, \
             F42, §3.4).",
            share.wrapping_div(10),
            share.wrapping_rem(10)
        );
    }
    let Some(context) = context.filter(|held| *held > 0) else {
        return format!(
            "{tokens} token(s) of prompt. This file declares no context length, so there is \
             nothing to state it against — which is unknown rather than unlimited (A7)."
        );
    };
    let share = tokens
        .saturating_mul(1_000)
        .checked_div(context)
        .unwrap_or(0);
    let fits = if tokens < context {
        format!(
            "{}.{}% of it, leaving {} token(s) for everything else — the answer, and anything \
             else in the window",
            share.wrapping_div(10),
            share.wrapping_rem(10),
            context.saturating_sub(tokens)
        )
    } else {
        format!(
            "{}.{}% of it: this prompt does not fit, before a single token of answer",
            share.wrapping_div(10),
            share.wrapping_rem(10)
        )
    };
    format!(
        "{tokens} token(s) of prompt against a DECLARED context of {context} token(s) — \
         {fits}. Declared is the file's claim about itself and not a measurement: what this \
         engine on this machine actually accepts is what `mcf probe` asks, and F42 found the \
         two can disagree (A21, B-055)."
    )
}

fn whole_or_shattered(prompt: &str, fragments: &[Fragment]) -> String {
    let words: Vec<&str> = prompt.split_whitespace().collect();
    if words.is_empty() {
        return "No whitespace-separated words to account for.".to_owned();
    }
    let mut whole = 0_usize;
    for word in &words {
        if fragments
            .iter()
            .any(|fragment| fragment.text.trim_start() == *word)
        {
            whole = whole.saturating_add(1);
        }
    }
    let bytes = fragments.iter().filter(|fragment| fragment.byte).count();
    let mut said = format!(
        "{whole} of {} whitespace-separated word(s) survived as a single token; the rest were \
         broken into pieces.",
        words.len()
    );
    if bytes > 0 {
        said = format!(
            "{said} {bytes} token(s) are raw bytes, which is this vocabulary having no piece \
             at all for that text."
        );
    }
    said.push_str(
        " Where a word breaks is a property of this model's vocabulary and not of the writing, \
         and nothing here rates it (§3.15).",
    );
    said
}

#[cfg(test)]
mod tests;
