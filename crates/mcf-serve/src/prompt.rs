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
//! **Settledness is the other half.** The same prompt under several seeds
//! either produces the same answer or does not. Several different answers means
//! the prompt underdetermines the answer *for this model* — again a fact, and
//! again not a verdict: an open question deserves several answers, and a
//! specification does not.
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
//! one per extra seed. A prompt of six sentences under three seeds is nine
//! generations, which is why this is a thing somebody asks for rather than
//! something that happens on the way past (§3.8).

/// How many seeds the settledness question is asked under.
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

/// The most clauses that will be ablated.
///
/// A long prompt is a long run of generations, and a report that took an hour
/// is one nobody waits for. What is over the cap is said rather than silently
/// dropped (A7).
pub const MOST_CLAUSES: usize = 8;

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
}

/// What several seeds made of the same prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    /// How many seeds were asked.
    pub asked: usize,
    /// How many distinct answers came back.
    ///
    /// One means the prompt settles the answer for this model under these
    /// conditions. More than one does not mean the prompt is bad.
    pub distinct: usize,
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
    /// The answer to the prompt as written, which every ablation is compared
    /// against.
    pub baseline: String,
    /// Each sentence, and what happened without it.
    pub clauses: Vec<Clause>,
    /// How many sentences the prompt had, where more than were ablated.
    pub clauses_over_the_cap: usize,
    /// What several seeds made of it.
    pub settled: Settled,
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
        self.clauses.iter().filter(|held| held.moved <= most).collect()
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
    let mut found = Vec::new();
    let mut held = String::new();
    for character in prompt.chars() {
        held.push(character);
        if matches!(character, '.' | '?' | '!' | '\n') {
            let trimmed = held.trim().to_owned();
            if !trimmed.is_empty() && trimmed.chars().any(char::is_alphanumeric) {
                found.push(trimmed);
            }
            held.clear();
        }
    }
    let trimmed = held.trim().to_owned();
    if !trimmed.is_empty() && trimmed.chars().any(char::is_alphanumeric) {
        found.push(trimmed);
    }
    found
}

/// The prompt with an inert sentence put into it.
///
/// Second from the end rather than appended, so that removing it disturbs what
/// follows — which is what removing a clause does, and the whole point of the
/// control is that the two operations match.
#[must_use]
pub fn with_inert(prompt: &str) -> String {
    let mut all = clauses_of(prompt);
    let at = all.len().saturating_sub(1);
    all.insert(at, NO_INSTRUCTION.to_owned());
    all.join(" ")
}

/// The prompt with one clause left out.
#[must_use]
pub fn without(clauses: &[String], at: usize) -> String {
    clauses
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != at)
        .map(|(_, held)| held.as_str())
        .collect::<Vec<&str>>()
        .join(" ")
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

/// How a caller asks the model one question.
///
/// The prompt and the seed in, what it said out. This module starts nothing and
/// reaches no socket: what generates is the daemon's, and holding that at the
/// boundary is what lets the whole measurement be tested without a model.
pub type Ask<'a> = &'a mut dyn FnMut(&str, u64) -> String;

/// Measures what a prompt does.
///
/// **The seed is held still across every ablation.** What must differ between
/// the baseline and a clause left out is the prompt and nothing else; a seed
/// that moved would make every comparison a comparison of two draws (D19).
#[must_use]
pub fn measure(prompt: &str, seed: u64, ask: Ask<'_>) -> Report {
    let all = clauses_of(prompt);
    let baseline = ask(prompt, seed);

    let ablated = all.len().min(MOST_CLAUSES);
    let mut clauses = Vec::with_capacity(ablated);
    // One clause is nothing to ablate: removing it leaves no prompt, and an
    // empty prompt's answer says nothing about the sentence.
    if all.len() > 1 {
        for at in 0..ablated {
            let shortened = without(&all, at);
            let without_it = ask(&shortened, seed);
            clauses.push(Clause {
                changed: without_it.trim() != baseline.trim(),
                moved: moved_by(baseline.trim(), without_it.trim()),
                text: all.get(at).cloned().unwrap_or_default(),
                without: without_it,
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
    let padded = with_inert(prompt);
    let floor = moved_by(ask(&padded, seed).trim(), baseline.trim());

    // Settledness: the same prompt, other seeds. The baseline's own seed counts
    // as one of them, so a report of three asks twice more.
    let mut answers = vec![baseline.trim().to_owned()];
    for extra in 1..SEEDS {
        let said = ask(prompt, seed.wrapping_add(extra as u64));
        answers.push(said.trim().to_owned());
    }
    let mut distinct = answers.clone();
    distinct.sort();
    distinct.dedup();

    Report {
        floor,
        baseline,
        clauses,
        clauses_over_the_cap: all.len().saturating_sub(ablated),
        settled: Settled {
            asked: SEEDS,
            distinct: distinct.len(),
        },
    }
}

#[cfg(test)]
mod tests;
