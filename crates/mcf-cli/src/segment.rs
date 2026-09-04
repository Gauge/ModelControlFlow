//! The prompt as the model actually receives it (B-381, PR11, §3.15, §X).
//!
//! **The observation this came from.** A vocabulary handles text it does not
//! contain by shattering it, and *where* it shatters is invisible to the
//! person who wrote the text. Two prompts that read identically to a human
//! can reach a model as twelve tokens and forty, and the difference is not in
//! the writing — it is in whether this particular model's vocabulary happens
//! to have words for it.
//!
//! **No generation and no judgement.** Nothing here runs the model, and
//! nothing here says a segmentation is good or bad. It reads the vocabulary
//! out of the file and shows what it does. What a reader concludes from
//! forty tokens where they expected twelve is theirs to conclude — §3.15's
//! *show, do not rate*.
//!
//! **What it can show that a count cannot.** A token count answers *how much*;
//! this answers *where*. A word that survives whole and a word shattered into
//! seven bytes both add to the same total, and only one of them is a sign that
//! the model has never seen the term.
//!
//! **Every contribution is exact, not guessed.** A token's text is taken as
//! the difference between decoding the first *k* identifiers and the first
//! *k-1* — which is by construction what that token contributed — rather than
//! by decoding it alone. Decoding a token alone is wrong for every
//! byte-level vocabulary, where one character is two to four tokens' worth of
//! bytes and each on its own is a replacement mark (F19).
//!
//! **Whose reading.** MCF's own tokenizer reads the file in this process
//! where it can. Where it cannot — a scheme it does not implement, a
//! pre-tokenizer it refuses — the prompt is read by the daemon through the
//! tokenizer of the engine that generates for this model, which is the
//! reading a generation would actually be charged (B-442, F158). The page
//! says which read it, because the two are different readers and a count
//! with no reader named is a number with no provenance (A21).

use std::path::Path;

use crate::Response;

/// Shows how a model's vocabulary segments a prompt.
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

/// One token, and what it contributed.
struct Fragment {
    identifier: usize,
    /// The text this token added, exactly.
    text: String,
    /// Whether the vocabulary reached for a raw byte here.
    byte: bool,
}

/// What the latest token added, as bytes rather than as characters.
///
/// **Why not `strip_prefix`.** A byte-level vocabulary can spell one character
/// across several tokens, and decoding a prefix that ends inside a character
/// yields a replacement mark (F19). So the decode of *k* tokens is not always
/// the decode of *k-1* with something appended — the replacement mark is
/// *replaced* by the character it was standing in for. A prefix strip fails
/// there and, with a fallback, reports the whole text as one token's
/// contribution: the last token of a Japanese phrase appearing to have
/// produced the entire phrase. That is a silent fallback lying about a
/// measurement, which is what A1 and A2 forbid.
///
/// Comparing bytes and taking what follows the common run is correct in both
/// cases: where a token appends, the common run is everything before it; where
/// a token completes a character, the common run stops at the replacement mark
/// and what follows is the completed character.
fn added_by(before: &str, now: &str) -> String {
    let common = now
        .as_bytes()
        .iter()
        .zip(before.as_bytes())
        .take_while(|(one, other)| one == other)
        .count();
    String::from_utf8_lossy(now.as_bytes().get(common..).unwrap_or_default()).into_owned()
}

/// The segmentation, rendered — or why there is none.
fn segmented(path: &Path, prompt: &str) -> Result<String, String> {
    let bytes = crate::bench::read_prefix(path)
        .ok_or_else(|| format!("mcf: {} could not be read", path.display()))?;
    let file = mcf_standin::gguf::parse(&bytes).map_err(|failure| {
        crate::say::refusal("the file could not be read as a model", &failure)
    })?;
    let vocabulary = match mcf_standin::tokenizer::Vocabulary::read(&file) {
        Ok(vocabulary) => vocabulary,
        // MCF's own tokenizer cannot read this file; the engine that
        // generates for it can, and the daemon reads through that one.
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

/// The prompt as the engine that generates for this model reads it, asked of
/// the daemon — for a file MCF's own tokenizer refuses (B-442, F158).
///
/// The refusal is kept on the page: that MCF's own tokenizer cannot read
/// this file is a fact about the file a reader should have, and the daemon's
/// reading is not a substitute for it but the other reader (A7, A21).
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

/// A text as the daemon reads it, and who read it.
struct Reading {
    /// Each token: its identifier and what it spells.
    pieces: Vec<(usize, String)>,
    /// The tokenizer that read, in the daemon's words.
    by: String,
}

/// Asks the daemon to read a text through the tokenizer of the engine that
/// generates for this model — as the start of a turn, with the beginning
/// marker, or as text alone.
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

/// Whether a piece is one raw byte, spelled the way the engine spells one.
fn is_a_byte_piece(piece: &str) -> bool {
    piece.len() == 6
        && piece.starts_with("<0x")
        && piece.ends_with('>')
        && piece
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|held| held.is_ascii_hexdigit()))
}

/// What a reader sees.
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
        // The text a token contributed, made visible: a token that adds
        // nothing a reader can see — a marker, a space — is exactly the one a
        // count would hide.
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
        // A1 and §3.15: what the model receives is not always what was typed,
        // and the difference is the reader's to see rather than MCF's to
        // smooth over.
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

/// Which of the markers somebody wrote are real control tokens here (B-383).
///
/// **The measured cost behind this** (F37): `<|im_start|>` written into a
/// prompt reaches the model as **eight ordinary tokens**, and the table that
/// produced was the most decisive-looking wrong answer in this repository. A
/// person tuning a prompt has, until now, had strictly less visibility than
/// the probe that was fooled by it.
///
/// **The two questions are separate, and both matter.** Whether the vocabulary
/// *has* a token spelled that way, and what the text *becomes* when typed.
/// They have different answers, and the second is always the same one: text a
/// person types never becomes a control token. That is D46 and F26's safety
/// property, deliberate and load-bearing — a surface that let typed characters
/// become the token a chat template uses to start a turn would let anyone
/// forge a turn boundary. So a vocabulary that *has* `<|im_start|>` and a
/// prompt that *contains* `<|im_start|>` still do not meet, and the reader is
/// told exactly that rather than left to infer it from a token count.
///
/// **No judgement** (§3.15). Writing a marker into a prompt is not a mistake;
/// it is a thing whose effect is invisible, and this makes it visible.
///
/// Two questions of whichever tokenizer read the prompt: whether it *has* a
/// token spelled so, and what the spelling becomes as typed text — so the
/// same page is written whether MCF's own tokenizer read or the engine's
/// did (B-442).
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

/// Substrings shaped like the markers models use.
///
/// `<...>` and `[...]`, which covers `<|im_start|>`, `<s>`, `<bos>`,
/// `<start_of_turn>` and `[INST]`. Bounded in length and stopped by
/// whitespace, because an unbounded scan would call half an English sentence
/// containing *a < b* a marker.
///
/// Shape and not a list: MCF does not keep a table of every family's markers,
/// and one would be out of date the week it was written. What it does is
/// notice the shape and then ask *this* vocabulary about it, which is the only
/// authority that matters.
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

/// What the prompt spends, against what this model says it can take
/// (B-382, PR11, B-055, A21, §3.8).
///
/// **The question it answers before anything is sent**: is this guidance
/// document, this transcript, this file, too long for this model? The same
/// text is a rounding error on one model's window and does not fit at all on
/// another's, and there is no surface today that says so.
///
/// **Declared, and said to be declared** (A21). The number in the file is the
/// model's claim about itself. What an engine on *this* machine actually
/// accepts is a different number — F42 measured a divergence and B-055 exists
/// because of it — and MCF has no measurement of it here, because a probe's
/// outcome is not yet written to the record (B-386). So this states the
/// declaration, names it as one, and names the command that would verify it.
/// A declared figure presented as a measured one is exactly A21's failure.
fn what_it_spends(tokens: usize, context: Option<usize>, measured: Option<usize>) -> String {
    // A measurement supersedes a declaration, which is the whole point of
    // taking one (A21). The declared figure stays in the sentence, because
    // *this file claims 8192 and this machine takes 4096* is the finding, and
    // dropping the claim would hide that the two disagree.
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

/// Which words survived whole, and which were broken up.
///
/// Words rather than tokens, because a word is what the person wrote. The
/// counting is by whitespace, which is not linguistics — a script that does
/// not separate words with spaces has one long word here, and saying so is
/// better than pretending to segment a language MCF does not know (A7).
fn whole_or_shattered(prompt: &str, fragments: &[Fragment]) -> String {
    let words: Vec<&str> = prompt.split_whitespace().collect();
    if words.is_empty() {
        return "No whitespace-separated words to account for.".to_owned();
    }
    let mut whole = 0_usize;
    for word in &words {
        // A word survived whole where one token contributed exactly it,
        // ignoring the space convention that travels in front of a word.
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
