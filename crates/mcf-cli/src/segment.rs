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
