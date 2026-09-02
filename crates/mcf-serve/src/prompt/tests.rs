//! What a prompt report may and may not conclude (§3.8, A19, A7).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::*;

/// What a fake model said, with identifiers to match.
fn said(text: &str) -> Answered {
    Answered {
        text: text.to_owned(),
        tokens: text.split_whitespace().map(str::len).collect(),
    }
}

/// A forced reading nobody took.
fn unforced(_: &str, _: &[usize]) -> Option<Held> {
    None
}

/// The clauses are the sentences a person wrote.
///
/// Not the tokenizer's pieces: what a vocabulary does to the writing is
/// `mcf segment`'s question, and answering it here would report the model's
/// units as the person's.
#[test]
fn the_clauses_are_the_sentences_somebody_wrote() {
    assert_eq!(
        clauses_of("Summarise this. Keep it short! Why? Because."),
        vec!["Summarise this.", "Keep it short!", "Why?", "Because."]
    );
    // A line break ends one too, which is how instructions are usually written.
    assert_eq!(
        clauses_of("Do the thing\nBe brief"),
        vec!["Do the thing", "Be brief"]
    );
    // Punctuation with no words in it is not a sentence.
    assert_eq!(clauses_of("...\n\n!!!"), Vec::<String>::new());
    assert_eq!(clauses_of(""), Vec::<String>::new());
}

/// Leaving a clause out leaves the others in their order.
#[test]
fn a_prompt_without_one_clause_keeps_the_rest() {
    let all = clauses_of("One. Two. Three.");
    assert_eq!(without(&all, 1), "One. Three.");
    assert_eq!(without(&all, 0), "Two. Three.");
    // An index past the end removes nothing rather than panicking.
    assert_eq!(without(&all, 9), "One. Two. Three.");
}

/// An answer that did not change means the clause did not reach it — and that
/// is not a claim that the clause was wrong.
#[test]
fn an_unchanged_answer_says_the_clause_was_not_used() {
    // The model ignores the second sentence entirely.
    let mut ask = |prompt: &str, _: u64| {
        if prompt.contains("One") {
            said("the same answer")
        } else {
            said("a different answer")
        }
    };
    let report = measure("One. Two.", 41, &mut ask, &mut unforced);
    let unused = report.unused();
    assert_eq!(unused.len(), 1, "one clause changed nothing");
    assert_eq!(
        unused.first().map(|held| held.text.as_str()),
        Some("Two."),
        "removing the second sentence left the answer alone, so it did not reach it"
    );
    // And the one whose removal changed the answer is marked as having.
    assert!(
        report
            .clauses
            .iter()
            .any(|held| held.text == "One." && held.changed)
    );
}

/// The seed is held still across every ablation.
///
/// What must differ between the baseline and a clause left out is the prompt.
/// A seed that moved would make every comparison a comparison of two draws, and
/// every clause would read as used (D19).
#[test]
fn the_seed_does_not_move_between_a_clause_and_its_baseline() {
    let mut seeds = Vec::new();
    let mut ask = |_: &str, seed: u64| {
        seeds.push(seed);
        said("always the same")
    };
    let _report = measure("One. Two. Three.", 41, &mut ask, &mut unforced);
    // The baseline and the three ablations all at 41; the extra seeds are the
    // settledness question and are meant to differ.
    let ablation_seeds: Vec<u64> = seeds.iter().copied().take(4).collect();
    assert_eq!(ablation_seeds, vec![41, 41, 41, 41]);
}

/// One sentence is nothing to ablate, and says so by holding no clauses.
///
/// Removing the only sentence leaves an empty prompt, whose answer says
/// nothing about the sentence. Reporting it as *used* would be an observation
/// nobody made (A7).
#[test]
fn a_prompt_of_one_sentence_has_nothing_to_ablate() {
    let mut ask = |_: &str, _: u64| said("an answer");
    let report = measure("Just the one sentence.", 41, &mut ask, &mut unforced);
    assert!(
        report.clauses.is_empty(),
        "there is no clause whose absence could be observed"
    );
    // The settledness question is still asked: it needs no second sentence.
    assert_eq!(report.settled.asked, SEEDS);
}

/// The same answer under every seed is one distinct answer.
#[test]
fn a_prompt_that_settles_the_answer_reports_one_answer() {
    let mut ask = |_: &str, _: u64| said("the one answer");
    let report = measure("One. Two.", 41, &mut ask, &mut unforced);
    assert_eq!(report.settled.distinct, 1);
}

/// Different answers under different seeds are counted, not judged.
#[test]
fn a_prompt_that_does_not_settle_reports_how_many_answers() {
    let mut ask = |_: &str, seed: u64| said(&format!("answer for {seed}"));
    let report = measure("One. Two.", 41, &mut ask, &mut unforced);
    assert_eq!(
        report.settled.distinct, SEEDS,
        "every seed gave its own answer"
    );
}

/// A long prompt is capped, and says how much was not ablated.
#[test]
fn a_prompt_longer_than_the_cap_says_what_was_left_out() {
    let mut long = String::new();
    for at in 0..MOST_CLAUSES + 3 {
        use std::fmt::Write as _;
        let _wrote = write!(long, "Sentence {at}. ");
    }
    let mut ask = |_: &str, _: u64| said("an answer");
    let report = measure(&long, 41, &mut ask, &mut unforced);
    assert_eq!(report.clauses.len(), MOST_CLAUSES);
    assert_eq!(report.clauses_over_the_cap, 3, "and it says how many");
}

/// How much moved is measured in words, and against the longer answer.
///
/// F: the first cut of this reported *used* for every sentence, because
/// removing any text shifts what follows it and greedy decoding then writes a
/// different string. An irrelevant sentence dropped into a prompt read exactly
/// like the instruction that carried it. Whether it changed is too blunt to be
/// the measurement; how much moved is the one that separates them.
#[test]
fn how_much_moved_is_counted_in_words() {
    assert_eq!(moved_by("the same words", "the same words"), 0);
    assert_eq!(moved_by("", ""), 0);
    // Nothing survives: the whole of the longer answer moved.
    assert_eq!(moved_by("one two", "three four"), 1_000_000);
    // One word of four: a quarter.
    assert_eq!(moved_by("a b c d", "a b c e"), 250_000);
    // Against the LONGER answer, so an answer that grew is not counted as
    // having moved more than all of itself.
    assert!(moved_by("a b", "a b c d") <= 1_000_000);
    assert_eq!(moved_by("a b", "a b c d"), 500_000);
    // A letter inside a word is a different word, and no more than that: what
    // is compared is what the model said, not how it spelled it.
    assert_eq!(moved_by("hello world", "hallo world"), 500_000);
}

/// A sentence that steered the answer is told from one that only perturbed it.
#[test]
fn a_sentence_that_steered_the_answer_is_told_from_one_that_perturbed_it() {
    // Removing the first rewrites the answer; removing the second changes one
    // word of four.
    let mut ask = |prompt: &str, _: u64| {
        if !prompt.contains("First") {
            said("something else entirely here")
        } else if prompt.contains("Second") {
            said("the answer is four words")
        } else {
            said("the answer is five words")
        }
    };
    let report = measure("First. Second.", 41, &mut ask, &mut unforced);
    let steered = report
        .clauses
        .iter()
        .find(|held| held.text == "First.")
        .expect("the first clause is there");
    let perturbed = report
        .clauses
        .iter()
        .find(|held| held.text == "Second.")
        .expect("the second clause is there");
    assert!(
        steered.moved > perturbed.moved,
        "removing the steering sentence moved more of the answer: {} against {}",
        steered.moved,
        perturbed.moved
    );
    // Both *changed* the answer, which is why the blunt reading told them
    // apart from nothing.
    assert!(steered.changed && perturbed.changed);
    // And the reader's own threshold separates them.
    let barely = report.barely_moved(300_000);
    assert_eq!(barely.len(), 1);
    assert_eq!(
        barely.first().map(|held| held.text.as_str()),
        Some("Second.")
    );
}

/// The floor is measured, not assumed.
///
/// How much a removal perturbs an answer is a property of the model and the
/// prompt together. A threshold chosen in the source would be a constant
/// standing in for a measurement, and it would travel to prompts it was never
/// taken on (A7, §3.4).
#[test]
fn the_floor_is_what_an_inert_sentence_does() {
    // The model rewrites the answer whenever the prompt changes at all, by a
    // fixed amount — which is exactly the perturbation the floor is for.
    let mut ask = |prompt: &str, _: u64| {
        if prompt.contains(NO_INSTRUCTION) {
            said("one two three different")
        } else if prompt.contains("Steer") {
            said("one two three four")
        } else {
            said("utterly different words entirely")
        }
    };
    let report = measure("Steer this. Inert here.", 41, &mut ask, &mut unforced);
    assert!(
        report.floor > 0,
        "an inert sentence moved the answer, and that is the floor"
    );
    // The clause whose removal rewrote everything is above the floor.
    let steered = report
        .clauses
        .iter()
        .find(|held| held.text == "Steer this.")
        .expect("the clause is there");
    assert!(
        steered.moved > report.floor,
        "removing it moved {} against a floor of {}",
        steered.moved,
        report.floor
    );
    assert!(
        !report
            .at_the_floor()
            .iter()
            .any(|held| held.text == "Steer this."),
        "a clause above the floor is not at it"
    );
}

/// The floor is measured by the same operation the clauses are.
///
/// F: it appended an inert sentence and measured what *adding* it did, while
/// every clause is *removed* — and removing from the middle of a prompt
/// disturbs what follows far more than appending to the end. The floor came out
/// at 12.9% against an irrelevant sentence's 50.6%, which separated nothing.
#[test]
fn the_inert_sentence_goes_where_a_clause_would_be() {
    let padded = with_inert("First. Second. Third.");
    assert!(
        padded.contains(NO_INSTRUCTION),
        "the inert sentence is in the prompt: {padded}"
    );
    // Second from the end, so removing it disturbs what follows — which is
    // what removing a clause does.
    assert_eq!(padded, format!("First. Second. {NO_INSTRUCTION} Third."));
    // A prompt of one sentence still gets one, before the only sentence.
    assert_eq!(with_inert("Only."), format!("{NO_INSTRUCTION} Only."));
}

/// The forced reading is asked with the baseline's own identifiers, for each
/// shortened prompt and for the inert one — and never for the baseline itself,
/// whose opening ranks first under its own prompt by construction.
///
/// On a one-word answer every removal that changes the word moves it a
/// hundred per cent, and three sentences tie. The rank orders them, which is
/// what B-429 is for: a sentence whose absence dropped the first token to its
/// seventeenth choice did more than one that dropped it to its fourth.
#[test]
fn the_opening_is_put_back_to_the_model_under_each_shortened_prompt() {
    let mut ask = |prompt: &str, _: u64| {
        if prompt.starts_with("One.") && prompt.ends_with("Three.") {
            said("Blue.")
        } else {
            said("Something else.")
        }
    };
    let mut forced: Vec<(String, Vec<usize>)> = Vec::new();
    let mut force = |prompt: &str, opening: &[usize]| {
        forced.push((prompt.to_owned(), opening.to_vec()));
        Some(Held {
            first: if prompt.contains("Two.") { Some(1) } else { Some(17) },
            kept: usize::from(prompt.contains("Two.")),
            of: opening.len(),
        })
    };
    let report = measure("One. Two. Three.", 41, &mut ask, &mut force);
    // Three clauses and the inert sentence: four readings, each with the
    // baseline's identifiers — `said` makes them the word lengths.
    assert_eq!(forced.len(), 4, "{forced:?}");
    assert!(
        forced.iter().all(|(_, opening)| opening == &vec![5]),
        "every reading was asked with the baseline's own opening: {forced:?}"
    );
    assert!(
        forced.iter().any(|(prompt, _)| prompt == "One. Three."),
        "the shortened prompt is what the model is asked under: {forced:?}"
    );
    assert!(
        forced
            .iter()
            .any(|(prompt, _)| prompt.contains(NO_INSTRUCTION)),
        "the floor is read by the same operation: {forced:?}"
    );
    let two = report
        .clauses
        .iter()
        .find(|held| held.text == "Two.")
        .expect("the second clause is reported");
    // The fake ranks the opening first wherever "Two." is still in the
    // prompt: so with "Two." removed it fell, and with "One." removed it held.
    assert_eq!(
        two.held,
        Some(Held {
            first: Some(17),
            kept: 0,
            of: 1
        })
    );
    let one = report
        .clauses
        .iter()
        .find(|held| held.text == "One.")
        .expect("the first clause is reported");
    assert_eq!(one.held.and_then(|held| held.first), Some(1));
    assert_eq!(
        report.floor_held.map(|held| held.of),
        Some(1),
        "the floor's reading is kept beside the floor"
    );
}

/// A baseline that said nothing has no opening to put back, and the reading
/// is not taken rather than reported as fully kept (A7).
#[test]
fn an_empty_answer_has_no_opening_to_force() {
    let mut ask = |_: &str, _: u64| said("");
    let mut force = |_: &str, _: &[usize]| Some(Held::default());
    let report = measure("One. Two.", 41, &mut ask, &mut force);
    assert!(report.clauses.iter().all(|held| held.held.is_none()));
    assert_eq!(report.floor_held, None);
}
