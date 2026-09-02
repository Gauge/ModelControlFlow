//! What a prompt does to a model, measured rather than judged (§3.8, A19, A7).
//!
//! **Not *is this a good prompt*.** That is a judgement, it needs a rater, and
//! §XIII says whose work that is. What is observable without one is narrower
//! and more useful than it sounds: which parts of a prompt the model actually
//! used, and whether the prompt settles the answer at all.
//!
//! **Ablation is the load-bearing measurement, and what it can carry was
//! measured rather than assumed.** Take the prompt apart into the sentences a
//! person wrote, remove one, and ask again with the seed held still; how much
//! of the answer moves is a fact about this prompt on this model, checkable by
//! anyone.
//!
//! **What it will not tell you is that a sentence was irrelevant.** Under
//! greedy decoding, removing any sentence from the middle of a prompt shifts
//! everything after it, and the answer is written differently whether or not
//! the sentence steered it. Measured here: an instruction scored 94.8%, an
//! obviously irrelevant sentence in the same prompt scored 50.6%, and a
//! control sentence carrying no instruction — inserted and then removed, the
//! same operation — put the floor at 14.0%. The irrelevant sentence sat far
//! above its own floor. Continuation drift dominates, and no threshold
//! recovers the distinction.
//!
//! So what this reports is an **ordering**, and it says so: the sentence that
//! steered the answer stands out from the ones that did not, which is the
//! question people actually ask of their own prompts. Reading the figure as
//! *relevance* would be reading drift as meaning. Separating the two needs a
//! comparison that is not text — whether the answer still satisfies what the
//! prompt asked for, which is the laboratory's mechanic, or the model's own
//! distribution over the prompt (B-373).
//!
//! **So a second measurement asks the model directly, without generating.**
//! The baseline's own opening is put after the shortened prompt, token by
//! token, and at each position the model is asked where it ranks the token the
//! baseline actually had there. Nothing is generated and nothing drifts: what
//! comes back is whether the model would still have *begun* the same answer
//! without the sentence, and how far its first token fell if not. On a
//! one-word answer, where every removal that changes the word scores the same
//! hundred per cent, this is what orders them — a sentence whose absence
//! dropped the answer's first token to its seventeenth choice did more than
//! one that dropped it to its fourth (measured: B-429).
//!
//! **What is taken apart is a document, and what is asked is a question after
//! it.** A persona or an instruction sheet is what a person crafts, and what
//! it does is only visible under a question: a persona alone is asked nothing,
//! and the model's answer to nothing measures nothing anybody wants. So the
//! text is taken apart — by paragraph where it has paragraphs, by sentence
//! where it does not, or as the caller says — and every variant of it is
//! followed by the same held question. Both go in the one turn the addressing
//! probe found: MCF has not probed for a system turn (D43) and does not assume
//! one here. The cap on how many parts are removed is the caller's, because
//! forty paragraphs is forty generations and whether that is worth it is a
//! choice about their time, not a constant (§3.15, B-430).
//!
//! **Settledness is the other half, and it is asked only at a temperature the
//! caller states.** The same prompt under several seeds either produces the
//! same answer or does not. Several different answers means the prompt
//! underdetermines the answer *for this model* — again a fact, and again not a
//! verdict: an open question deserves several answers, and a specification
//! does not. But every other generation here is greedy, and under a greedy
//! sampler the seed changes nothing: two more generations at temperature 0
//! measured nothing and cost two generations, and the line that said so was
//! honest and wasteful (F147, B-431). So the seeds are drawn at a temperature
//! only when the caller has stated one — B60 forbids MCF a house temperature —
//! and when none is stated no generation is spent on the question, and the
//! report says it was not asked rather than that the answer settled (A7).
//!
//! **Cross-checked by:** the two measurements are counting, and what needs
//! checking is what is counted. `prompt/tests` pins that an unchanged answer is
//! reported as *not used* rather than as *wrong*, that a clause list is the
//! sentences a person wrote and not the tokenizer's pieces, and that a prompt
//! of one sentence is refused as having nothing to ablate rather than reported
//! as fully used. There is no arithmetic here beyond a count of distinct
//! strings (A19).
//!
//! **What this costs.** One generation for the baseline, one per clause, and
//! one per extra seed; the forced reading is one request a token of the
//! opening for each clause, cached, and generates nothing. A prompt of six sentences under three seeds is nine
//! generations, which is why this is a thing somebody asks for rather than
//! something that happens on the way past (§3.8).

use mcf_core::configuration::Thousandths;

pub use crate::generation::Draw;

/// How many seeds the settledness question is asked under, when it is asked.
///
/// Three, because two cannot tell *the same twice* from *a coincidence* and
/// the cost is a generation each.
pub const SEEDS: usize = 3;

/// A sentence that carries no instruction, used to find the floor.
///
/// **Removing anything moves the answer.** Under greedy decoding a prompt one
/// sentence shorter is a different context, and what follows is written
/// differently whether or not the sentence mattered — so every clause reads as
/// influential and the measurement says nothing. What separates signal from
/// that is knowing how much the answer moves for a sentence that could not
/// have steered it.
///
/// So one is added and the answer measured against the baseline. It is flat,
/// carries nothing to act on, and is about the room rather than the task —
/// a sentence a model has nothing to do with (F: an irrelevant sentence in a
/// real prompt scored 50.6% against a real instruction's 94.8%, and only the
/// ordering of the two was informative).
pub const NO_INSTRUCTION: &str = "The room is quiet.";

/// How many tokens of the baseline's opening are put to the model under each
/// shortened prompt.
///
/// Twelve is an opening: a word or a short sentence, enough to see whether the
/// answer set off the same way and short enough that a report of eight clauses
/// is under a hundred requests, each one cached and generating nothing.
pub const MOST_FORCED: usize = 12;

/// The most parts that will be ablated unless the caller says otherwise.
///
/// A long prompt is a long run of generations, and a report that took an hour
/// is one nobody asked for. What is over the cap is said rather than silently
/// dropped (A7), and the cap itself is the caller's to raise ([`Taken::most`]).
pub const MOST_CLAUSES: usize = 8;

/// What a document is taken apart into.
///
/// A person writes in one or the other: a question is sentences, a persona is
/// paragraphs, and a persona ablated by sentence is hundreds of generations
/// about text whose units are its paragraphs. Which is which is decided from
/// the text where the caller does not say ([`Unit::for_text`]), and the
/// report says which it was and who decided (§3.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// A sentence: ended by `.`, `?`, `!` before whitespace, or a line break.
    Sentence,
    /// A paragraph: ended by a blank line.
    Paragraph,
}

impl Unit {
    /// The unit a text is written in: paragraphs where a blank line separates
    /// two of them, sentences otherwise.
    #[must_use]
    pub fn for_text(text: &str) -> Self {
        if parts_of(text, Self::Paragraph).len() > 1 {
            Self::Paragraph
        } else {
            Self::Sentence
        }
    }

    /// The word on the wire and in a report.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sentence => "sentence",
            Self::Paragraph => "paragraph",
        }
    }

    /// The unit a word names, if it names one.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "sentence" | "sentences" => Some(Self::Sentence),
            "paragraph" | "paragraphs" => Some(Self::Paragraph),
            _ => None,
        }
    }
}

/// One part of a document, and the whitespace that followed it.
///
/// **The separator travels with the part** so that a document with one part
/// removed is the document as written, less that part — a bullet list stays a
/// bullet list, paragraphs stay paragraphs. An earlier cut rejoined sentences
/// with a space, so every ablation of a multi-line prompt was also a
/// reformatting of it, and the baseline was the only variant with its line
/// breaks (F: found when the field took a document, B-430).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The part, as written, trimmed at both ends.
    pub text: String,
    /// What lay between it and the next part: whitespace, and any run of
    /// punctuation that was not a part by itself. Empty after the last.
    pub after: String,
}

/// What is taken apart, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Taken<'a> {
    /// The document: the text whose parts are removed in turn.
    pub text: &'a str,
    /// A question every variant is followed by, held still. `None` asks the
    /// document alone, which is what a prompt that is itself a question wants.
    pub then: Option<&'a str>,
    /// What to take the document apart into; `None` lets the text decide.
    pub by: Option<Unit>,
    /// How many parts to remove at most; `None` is [`MOST_CLAUSES`].
    pub most: Option<usize>,
}

impl Taken<'_> {
    /// The unit this will be taken apart by, and whether the caller chose it.
    #[must_use]
    pub fn unit(&self) -> (Unit, bool) {
        self.by
            .map_or_else(|| (Unit::for_text(self.text), false), |by| (by, true))
    }

    /// The cap on parts removed.
    #[must_use]
    pub fn cap(&self) -> usize {
        self.most.unwrap_or(MOST_CLAUSES).max(1)
    }

    /// The parts, in the unit this is taken apart by.
    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        parts_of(self.text, self.unit().0)
    }

    /// One variant of the document, followed by the held question.
    ///
    /// The question goes after a blank line, in the same turn (D43).
    #[must_use]
    pub fn asked(&self, document: &str) -> String {
        match self.then.map(str::trim).filter(|then| !then.is_empty()) {
            Some(then) => format!("{document}\n\n{then}"),
            None => document.to_owned(),
        }
    }
}

/// Whether the model would still have begun the same answer.
///
/// **Teacher-forced, so nothing drifts.** The ablation compares two answers,
/// and under greedy decoding two answers part at their first difference and
/// are written differently from there on — which is why that column is an
/// ordering. This puts the baseline's own opening after the shortened prompt
/// and asks, at each token, where the model ranks the token the baseline had
/// there. The answer is a rank, not a comparison of texts, and it does not
/// depend on what was written after the parting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    /// Where the model ranked the baseline's first token under this prompt,
    /// counting from one. `None` where it was outside the depth read — a
    /// bound rather than an absence (A7); the report says the depth.
    pub first: Option<usize>,
    /// How many of the opening's `of` tokens were still the model's first
    /// choice: the same answer would have begun the same way for this many
    /// tokens.
    pub kept: usize,
    /// How many tokens of the opening were asked about.
    pub of: usize,
}

/// One sentence of the prompt, and what happened without it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// The sentence, as the person wrote it.
    pub text: String,
    /// The answer when this sentence was left out.
    pub without: String,
    /// Whether leaving it out changed the answer at all.
    pub changed: bool,
    /// How much of the answer moved without it, in parts per million of the
    /// longer of the two answers.
    ///
    /// **Whether it changed is too blunt to be the measurement.** Removing any
    /// text shifts what follows it, so under greedy decoding almost every
    /// removal changes the answer *somehow* — and a report that said *used* for
    /// every sentence would tell a person nothing. What separates a sentence
    /// that steered the answer from one that merely perturbed it is how much
    /// moved: a word or two against half the reply.
    ///
    /// Word-level, because a person writes words and a difference counted in
    /// tokens would be a difference about the vocabulary (F: found by putting
    /// an irrelevant sentence into a prompt and watching it report as used).
    pub moved: u64,
    /// Whether the model would still have begun the baseline's answer without
    /// this sentence. `None` where the reading could not be taken, which the
    /// report says separately from a rank (A7).
    pub held: Option<Held>,
}

/// What several seeds made of the same prompt, at a stated temperature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    /// The temperature every seed was drawn at: the caller's, never MCF's
    /// (B60), and the condition every figure below is under (§3.4).
    pub temperature: Thousandths,
    /// How many seeds were asked.
    pub asked: usize,
    /// How many distinct answers came back.
    ///
    /// One means the prompt settles the answer for this model under these
    /// conditions. More than one does not mean the prompt is bad.
    pub distinct: usize,
    /// How far apart the two farthest of the sampled answers are, in parts
    /// per million of their words — [`moved_by`], over every pair.
    ///
    /// The count says *how many*; this says *how different*. Three answers
    /// that differ in a word each are three distinct answers and a settled
    /// prompt; three that share nothing are three distinct answers and are not.
    pub spread: u64,
    /// How far the farthest sample sits from the greedy answer, the same way.
    ///
    /// The greedy answer is what every ablation was compared against, so this
    /// is what sampling does to the thing the rest of the report is about.
    pub from_greedy: u64,
}

/// What a prompt did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// How much the answer moves for a sentence that carries no instruction.
    ///
    /// The floor every clause is read against: a clause that moved this much
    /// or less moved the answer no more than an inert sentence would have, and
    /// what it did beyond perturbing the context is not visible here.
    pub floor: u64,
    /// The same forced reading for the inert sentence: how the baseline's
    /// opening ranks with a sentence carrying no instruction put in. The
    /// floor of `held`, taken by the same operation.
    pub floor_held: Option<Held>,
    /// The answer to the prompt as written, which every ablation is compared
    /// against.
    pub baseline: String,
    /// Each sentence, and what happened without it.
    pub clauses: Vec<Clause>,
    /// How many parts the document had beyond the ones ablated.
    pub clauses_over_the_cap: usize,
    /// What the document was taken apart into.
    pub unit: Unit,
    /// Whether the caller chose the unit, or the text did.
    pub unit_chosen: bool,
    /// The cap the run was under.
    pub most: usize,
    /// The question every variant was followed by, if one was.
    pub then: Option<String>,
    /// What several seeds made of it at a stated temperature, or `None`
    /// where no temperature was stated and the question was not asked.
    pub settled: Option<Settled>,
}

impl Report {
    /// The sentences whose removal changed nothing at all.
    #[must_use]
    pub fn unused(&self) -> Vec<&Clause> {
        self.clauses.iter().filter(|held| !held.changed).collect()
    }

    /// The sentences whose removal moved less of the answer than `most`.
    ///
    /// What counts as *barely* is the reader's, which is why it is an argument:
    /// MCF has not measured what size of change a person notices, and a
    /// threshold baked in here would be a judgement wearing a number's clothes
    /// (§3.15, A7).
    #[must_use]
    pub fn barely_moved(&self, most: u64) -> Vec<&Clause> {
        self.clauses
            .iter()
            .filter(|held| held.moved <= most)
            .collect()
    }

    /// The sentences that moved the answer no more than an inert one would.
    ///
    /// Measured against this run's own floor rather than a number chosen here:
    /// how much a removal perturbs an answer is a property of the model and
    /// the prompt together, and a threshold that travelled between them would
    /// be a constant standing in for a measurement (A7, §3.4).
    #[must_use]
    pub fn at_the_floor(&self) -> Vec<&Clause> {
        self.clauses
            .iter()
            .filter(|held| held.moved <= self.floor)
            .collect()
    }
}

/// Splits a prompt into the sentences a person wrote.
///
/// **The person's units, not the tokenizer's.** A report about pieces would be
/// a report about the vocabulary — which `mcf segment` already gives — and the
/// question here is about the writing. Sentence ends are `.`, `?`, `!` and a
/// line break; a run of them is one end.
#[must_use]
pub fn clauses_of(prompt: &str) -> Vec<String> {
    parts_of(prompt, Unit::Sentence)
        .into_iter()
        .map(|part| part.text)
        .collect()
}

/// Whether a character ends a sentence here: `.`, `?` or `!` before
/// whitespace or the end, or a line break.
///
/// **A full stop ends a sentence only where a space follows it.** Splitting
/// on every `.` cut `0.001` into two clauses, `arr.Length` into two, and
/// `System.Numerics` into two — so a prompt about precision, or one naming
/// any dotted identifier, was ablated on fragments that were never sentences.
/// A newline is a break whatever follows it (F147).
///
/// What this does not fix is `e.g.`, which ends in a full stop and a space
/// and is not the end of a sentence. Telling that from a sentence ending in
/// the letter g needs a list of abbreviations, which is a fact about a
/// language rather than about this prompt, and getting it wrong in the other
/// direction would silently join two real sentences. The failure that remains
/// splits one clause into two; the one removed split a number in half.
fn ends_a_sentence(character: char, next: Option<char>) -> bool {
    character == '\n'
        || (matches!(character, '.' | '?' | '!') && next.is_none_or(char::is_whitespace))
}

/// Splits a document into its parts, each with the whitespace that followed.
///
/// A paragraph ends at a blank line — a run of whitespace with two line
/// breaks in it. A piece with nothing alphanumeric in it (a lone `...`, a
/// rule of dashes) is not a part: it is folded into the separator before it,
/// so that removing the part before it removes it too.
#[must_use]
pub fn parts_of(text: &str, by: Unit) -> Vec<Part> {
    let mut found: Vec<Part> = Vec::new();
    let mut held = String::new();
    let mut characters = text.chars().peekable();
    // What ended the last part and lay after it, kept until the next part
    // begins so that the last part's trailing whitespace is not its `after`.
    let mut between = String::new();
    while let Some(character) = characters.next() {
        let ends = match by {
            Unit::Sentence => {
                held.push(character);
                ends_a_sentence(character, characters.peek().copied())
            }
            Unit::Paragraph => {
                if character == '\n' {
                    // A blank line: this break, then only whitespace up to
                    // another one.
                    let mut ahead = characters.clone();
                    let blank = std::iter::from_fn(|| ahead.next())
                        .take_while(|next| next.is_whitespace())
                        .any(|next| next == '\n');
                    if !blank {
                        held.push(character);
                    }
                    blank
                } else {
                    held.push(character);
                    false
                }
            }
        };
        if !ends {
            continue;
        }
        let mut after = String::new();
        if by == Unit::Paragraph {
            after.push(character);
        }
        while let Some(next) = characters
            .peek()
            .copied()
            .filter(|next| next.is_whitespace())
        {
            after.push(next);
            characters.next();
        }
        // The separator kept from before this piece, and the piece's own
        // trailing whitespace, which a sentence keeps inside `held`.
        let trimmed = held.trim().to_owned();
        let trailing: String = held
            .chars()
            .rev()
            .take_while(|held| held.is_whitespace())
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();
        held.clear();
        if trimmed.chars().any(char::is_alphanumeric) {
            if let Some(last) = found.last_mut() {
                last.after.push_str(&between);
            }
            between.clear();
            found.push(Part {
                text: trimmed,
                after: String::new(),
            });
            between.push_str(&trailing);
            between.push_str(&after);
        } else {
            between.push_str(&trimmed);
            between.push_str(&trailing);
            between.push_str(&after);
        }
    }
    let trimmed = held.trim().to_owned();
    if trimmed.chars().any(char::is_alphanumeric) {
        if let Some(last) = found.last_mut() {
            last.after.push_str(&between);
        }
        found.push(Part {
            text: trimmed,
            after: String::new(),
        });
    }
    found
}

/// The parts put back together as they were written.
#[must_use]
pub fn joined(parts: &[Part]) -> String {
    let mut text = String::new();
    for (at, part) in parts.iter().enumerate() {
        text.push_str(&part.text);
        // What followed the last part followed nothing that is kept: a
        // separator, or a rule of dashes folded into one, goes with the part
        // it led to.
        if at.saturating_add(1) < parts.len() {
            text.push_str(&part.after);
        }
    }
    text
}

/// The document with an inert sentence put into it.
///
/// Second from the end rather than appended, so that removing it disturbs what
/// follows — which is what removing a clause does, and the whole point of the
/// control is that the two operations match. It takes the separator of the
/// part it follows, so that a paragraph gets a paragraph's break and a
/// sentence a sentence's.
#[must_use]
pub fn with_inert(parts: &[Part]) -> String {
    let mut all = parts.to_vec();
    let at = all.len().saturating_sub(1);
    let after = at
        .checked_sub(1)
        .and_then(|before| all.get(before))
        .map_or_else(|| " ".to_owned(), |part| part.after.clone());
    all.insert(
        at,
        Part {
            text: NO_INSTRUCTION.to_owned(),
            after,
        },
    );
    joined(&all)
}

/// The document with one part left out, as written.
#[must_use]
pub fn without(parts: &[Part], at: usize) -> String {
    let kept: Vec<Part> = parts
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != at)
        .map(|(_, held)| held.clone())
        .collect();
    joined(&kept)
}

/// How much two answers differ, in parts per million of the longer.
///
/// The Levenshtein distance over words rather than characters: what is being
/// compared is what the model said, and a difference of one letter inside a
/// word is not a different word. Nought means the two are the same run of
/// words; a million means nothing survived.
#[must_use]
pub fn moved_by(one: &str, other: &str) -> u64 {
    let a: Vec<&str> = one.split_whitespace().collect();
    let b: Vec<&str> = other.split_whitespace().collect();
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0;
    }
    // One row at a time: the whole table is not needed and an answer can be
    // long.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.iter().enumerate() {
        let mut previous = row.first().copied().unwrap_or(0);
        if let Some(first) = row.first_mut() {
            *first = i.saturating_add(1);
        }
        for (j, right) in b.iter().enumerate() {
            let held = row.get(j.saturating_add(1)).copied().unwrap_or(0);
            let cost = usize::from(left != right);
            let candidate = previous
                .saturating_add(cost)
                .min(held.saturating_add(1))
                .min(row.get(j).copied().unwrap_or(0).saturating_add(1));
            if let Some(cell) = row.get_mut(j.saturating_add(1)) {
                *cell = candidate;
            }
            previous = held;
        }
    }
    let distance = row.last().copied().unwrap_or(0);
    u64::try_from(distance)
        .unwrap_or(0)
        .saturating_mul(1_000_000)
        .wrapping_div(u64::try_from(longest).unwrap_or(1).max(1))
}

/// What the model said to one question: the text, and the identifiers it
/// said it in.
///
/// Both, because the forced reading puts the *identifiers* back to the model
/// — re-encoding the text could segment it differently from the way the model
/// produced it, and a rank read at a token the model never wrote would be a
/// rank of nothing (A21).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answered {
    /// The text.
    pub text: String,
    /// The same thing as identifiers.
    pub tokens: Vec<usize>,
}

/// How a caller asks the model one question.
///
/// The prompt and the draw in, what it said out. This module starts nothing
/// and reaches no socket: what generates is the daemon's, and holding that at
/// the boundary is what lets the whole measurement be tested without a model.
pub type Ask<'a> = &'a mut dyn FnMut(&str, Draw) -> Answered;

/// How a caller asks where the model ranks an opening after a prompt.
///
/// The prompt and the opening's identifiers in; where each ranked out, or
/// `None` where the reading could not be taken at all. Nothing is generated.
pub type Force<'a> = &'a mut dyn FnMut(&str, &[usize]) -> Option<Held>;

/// Measures what a prompt does.
///
/// **The seed is held still across every ablation, and every ablation is
/// greedy.** What must differ between the baseline and a clause left out is
/// the prompt and nothing else; a seed that moved would make every comparison
/// a comparison of two draws (D19). `settle` is the temperature the
/// settledness seeds are drawn at, and `None` asks them nothing.
#[must_use]
pub fn measure(
    taken: &Taken<'_>,
    seed: u64,
    settle: Option<Thousandths>,
    ask: Ask<'_>,
    force: Force<'_>,
) -> Report {
    let (unit, unit_chosen) = taken.unit();
    let all = parts_of(taken.text, unit);
    // The baseline is the parts put back together, not the text as pasted:
    // what differs between it and a variant must be the part removed and
    // nothing else, and a variant is always the joined parts (§3.4).
    let prompt = taken.asked(&joined(&all));
    let Answered {
        text: baseline,
        tokens: opening,
    } = ask(&prompt, Draw::greedy(seed));
    let opening: Vec<usize> = opening.into_iter().take(MOST_FORCED).collect();

    let most = taken.cap();
    let ablated = all.len().min(most);
    let mut clauses = Vec::with_capacity(ablated);
    // One clause is nothing to ablate: removing it leaves no prompt, and an
    // empty prompt's answer says nothing about the sentence.
    if all.len() > 1 {
        for at in 0..ablated {
            let shortened = taken.asked(&without(&all, at));
            let without_it = ask(&shortened, Draw::greedy(seed)).text;
            // The opening is an empty list where the baseline said nothing,
            // and a rank over nothing is not taken rather than read as kept.
            let held = if opening.is_empty() {
                None
            } else {
                force(&shortened, &opening)
            };
            clauses.push(Clause {
                changed: without_it.trim() != baseline.trim(),
                moved: moved_by(baseline.trim(), without_it.trim()),
                text: all
                    .get(at)
                    .map(|part| part.text.clone())
                    .unwrap_or_default(),
                without: without_it,
                held,
            });
        }
    }

    // The floor, measured by the SAME operation the clauses are.
    //
    // An earlier cut of this appended an inert sentence and measured what
    // adding it did — but every clause above is *removed*, and removing text
    // from the middle of a prompt disturbs what follows it far more than
    // appending to the end does. The two were not comparable, and the floor
    // came out at 12.9% while an irrelevant sentence in a real prompt scored
    // 50.6%: a floor that separated nothing.
    //
    // So the inert sentence is put in and then taken out. The prompt with it
    // is asked, and the answer compared against the baseline — which is that
    // same prompt with the inert sentence removed. One removal against
    // another, which is the comparison the numbers above need.
    let padded = taken.asked(&with_inert(&all));
    let floor = moved_by(
        ask(&padded, Draw::greedy(seed)).text.trim(),
        baseline.trim(),
    );
    let floor_held = if opening.is_empty() || all.len() <= 1 {
        None
    } else {
        force(&padded, &opening)
    };

    let settled = settle.map(|temperature| settled(&prompt, seed, temperature, &baseline, ask));

    Report {
        floor,
        floor_held,
        baseline,
        clauses,
        clauses_over_the_cap: all.len().saturating_sub(ablated),
        unit,
        unit_chosen,
        most,
        then: taken
            .then
            .map(str::trim)
            .filter(|then| !then.is_empty())
            .map(str::to_owned),
        settled,
    }
}

/// The settledness question: the same prompt, `SEEDS` seeds, one temperature.
///
/// The greedy baseline is not one of the samples — it was drawn under another
/// condition, and counting it among them would make the count a count of two
/// things (§3.4). It is what `from_greedy` is measured from.
fn settled(
    prompt: &str,
    seed: u64,
    temperature: Thousandths,
    baseline: &str,
    ask: Ask<'_>,
) -> Settled {
    let answers: Vec<String> = (0..SEEDS)
        .map(|extra| {
            let draw = Draw {
                seed: seed.wrapping_add(u64::try_from(extra).unwrap_or(u64::MAX)),
                temperature,
            };
            ask(prompt, draw).text.trim().to_owned()
        })
        .collect();
    let mut distinct = answers.clone();
    distinct.sort();
    distinct.dedup();
    let mut spread = 0_u64;
    for (at, one) in answers.iter().enumerate() {
        for other in answers.iter().skip(at.saturating_add(1)) {
            spread = spread.max(moved_by(one, other));
        }
    }
    let from_greedy = answers
        .iter()
        .map(|answer| moved_by(baseline.trim(), answer))
        .max()
        .unwrap_or(0);
    Settled {
        temperature,
        asked: SEEDS,
        distinct: distinct.len(),
        spread,
        from_greedy,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod splitting_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::clauses_of;

    /// A number is not two sentences (F147).
    #[test]
    fn a_decimal_does_not_end_a_sentence() {
        let held = clauses_of("Results must be accurate to 0.001 tolerance. Use doubles.");
        assert_eq!(
            held,
            vec![
                "Results must be accurate to 0.001 tolerance.".to_owned(),
                "Use doubles.".to_owned()
            ],
            "a prompt about precision was ablated on half a number"
        );
    }

    /// Nor is a dotted identifier, which is most of what a coding prompt says.
    #[test]
    fn a_dotted_name_does_not_end_a_sentence() {
        assert_eq!(
            clauses_of("Call arr.Length to get the size. Return an int."),
            vec![
                "Call arr.Length to get the size.".to_owned(),
                "Return an int.".to_owned()
            ]
        );
        assert_eq!(clauses_of("Use System.Numerics.").len(), 1);
    }

    /// A newline is a break whatever follows it.
    #[test]
    fn a_line_ending_is_a_break() {
        assert_eq!(clauses_of("One line.\nAnother line.").len(), 2);
        assert_eq!(clauses_of("no punctuation\nand more").len(), 2);
    }

    /// And what is still wrong, held so that it is a known limit rather than a
    /// surprise: an abbreviation ends in a full stop and a space.
    #[test]
    fn an_abbreviation_is_still_read_as_an_ending() {
        // Two sentences, read as three. Recorded because a check that passes
        // on what is fixed and says nothing about what is not is a check that
        // reads as a guarantee (A7).
        assert_eq!(
            clauses_of("Use a library, e.g. System.Numerics. Keep it simple.").len(),
            3
        );
    }

    /// The whole point: the prompt is the sentences a person wrote.
    #[test]
    fn a_prompt_is_split_where_a_person_would_split_it() {
        let held = clauses_of(
            "write a class in c#. the class handles all the basic math functions. each \
             function should output extremely accurate results.",
        );
        assert_eq!(held.len(), 3, "{held:?}");
        assert!(held[0].ends_with("c#."), "{held:?}");
    }
}
