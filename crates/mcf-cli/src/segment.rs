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
    let vocabulary = mcf_standin::tokenizer::Vocabulary::read(&file).map_err(|failure| {
        crate::say::refusal("this file carries no vocabulary MCF can read", &failure)
    })?;
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

    Ok(render(prompt, &fragments, &before, &vocabulary))
}

/// What a reader sees.
fn render(
    prompt: &str,
    fragments: &[Fragment],
    decoded: &str,
    vocabulary: &mcf_standin::tokenizer::Vocabulary,
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
    lines.push(whole_or_shattered(prompt, fragments));
    for said in marker_fidelity(prompt, vocabulary) {
        lines.push(said);
    }
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
fn marker_fidelity(prompt: &str, vocabulary: &mcf_standin::tokenizer::Vocabulary) -> Vec<String> {
    let found = marker_shaped(prompt);
    if found.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![String::new(), "Markers written into the prompt:".to_owned()];
    for marker in &found {
        let spelled = vocabulary.encode(marker, false).map_or_else(
            |_| "no tokens at all".to_owned(),
            |held| format!("{} ordinary token(s)", held.len()),
        );
        lines.push(if vocabulary.has_token(marker) {
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
