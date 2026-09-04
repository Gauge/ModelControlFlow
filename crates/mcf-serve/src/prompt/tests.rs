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
        thought: None,
    }
}

/// A forced reading nobody took.
fn unforced(_: &str, _: &[usize]) -> Option<Held> {
    None
}

/// A document taken apart on its own: no question after it, the text deciding
/// the unit, the default cap.
/// A caller's temperature with nothing cut — the condition the settledness
/// tests draw under.
fn warm() -> Settle {
    Settle {
        temperature: Thousandths(700),
        truncation: Truncation::OFF,
    }
}

fn only(text: &str) -> Taken<'_> {
    Taken {
        text,
        ..Taken::default()
    }
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
    let all = parts_of("One. Two. Three.", Unit::Sentence);
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
    let mut ask = |prompt: &str, _: Draw| {
        if prompt.contains("One") {
            said("the same answer")
        } else {
            said("a different answer")
        }
    };
    let report = measure(&only("One. Two."), 41, None, &mut ask, &mut unforced);
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
    let mut draws = Vec::new();
    let mut ask = |_: &str, draw: Draw| {
        draws.push(draw);
        said("always the same")
    };
    let _report = measure(
        &only("One. Two. Three."),
        41,
        Some(warm()),
        &mut ask,
        &mut unforced,
    );
    // The baseline, the three ablations and the control, all greedy at 41;
    // the seeds after them are the settledness question and are meant to
    // differ, at the temperature stated.
    let ablations: Vec<Draw> = draws.iter().copied().take(5).collect();
    assert_eq!(ablations, vec![Draw::greedy(41); 5]);
    let settling: Vec<Draw> = draws.iter().copied().skip(5).collect();
    assert_eq!(
        settling,
        (0..3)
            .map(|extra| Draw {
                seed: 41 + extra,
                temperature: Thousandths(700),
                truncation: Truncation::OFF,
            })
            .collect::<Vec<_>>()
    );
}

/// One sentence is nothing to ablate, and says so by holding no clauses.
///
/// Removing the only sentence leaves an empty prompt, whose answer says
/// nothing about the sentence. Reporting it as *used* would be an observation
/// nobody made (A7).
#[test]
fn a_prompt_of_one_sentence_has_nothing_to_ablate() {
    let mut ask = |_: &str, _: Draw| said("an answer");
    let report = measure(
        &only("Just the one sentence."),
        41,
        Some(warm()),
        &mut ask,
        &mut unforced,
    );
    assert!(
        report.clauses.is_empty(),
        "there is no clause whose absence could be observed"
    );
    // The settledness question is still asked: it needs no second sentence.
    assert_eq!(report.settled.map(|held| held.asked), Some(SEEDS));
}

/// No temperature stated, no seed drawn: under a greedy sampler the seed
/// changes nothing, and two generations that could not differ would measure
/// nothing and cost two generations (F147, B-431).
#[test]
fn without_a_temperature_the_seeds_are_not_asked() {
    let mut asked = 0_usize;
    let mut ask = |_: &str, _: Draw| {
        asked += 1;
        said("an answer")
    };
    let report = measure(&only("One. Two."), 41, None, &mut ask, &mut unforced);
    assert_eq!(
        report.settled, None,
        "not settled, not unsettled: not asked"
    );
    // The baseline, two ablations and the control, and nothing more.
    assert_eq!(asked, 4);
}

/// The same answer under every seed is one distinct answer, no spread.
#[test]
fn a_prompt_that_settles_the_answer_reports_one_answer() {
    let mut ask = |_: &str, _: Draw| said("the one answer");
    let report = measure(
        &only("One. Two."),
        41,
        Some(warm()),
        &mut ask,
        &mut unforced,
    );
    let settled = report.settled.expect("a temperature was stated");
    assert_eq!(settled.distinct, 1);
    assert_eq!(settled.spread, 0);
    assert_eq!(settled.from_greedy, 0);
    assert_eq!(settled.temperature, Thousandths(700));
}

/// Different answers under different seeds are counted, not judged — and how
/// far apart they are is said beside how many there were.
#[test]
fn a_prompt_that_does_not_settle_reports_how_many_answers_and_how_far_apart() {
    let mut ask = |_: &str, draw: Draw| {
        // Greedy, or the first seed, gives the one answer; the seeds after it
        // each give their own.
        if draw.is_greedy() || draw.seed == 41 {
            said("the greedy answer here")
        } else {
            said(&format!("answer for seed {}", draw.seed))
        }
    };
    let report = measure(
        &only("One. Two."),
        41,
        Some(warm()),
        &mut ask,
        &mut unforced,
    );
    let settled = report.settled.expect("a temperature was stated");
    assert_eq!(settled.distinct, SEEDS, "every seed gave its own answer");
    // "the greedy answer here" against "answer for seed 42": four words,
    // every one different.
    assert_eq!(settled.spread, 1_000_000);
    assert_eq!(settled.from_greedy, 1_000_000);
}

/// A long prompt is capped, and says how much was not ablated.
#[test]
fn a_prompt_longer_than_the_cap_says_what_was_left_out() {
    let mut long = String::new();
    for at in 0..MOST_CLAUSES + 3 {
        use std::fmt::Write as _;
        let _wrote = write!(long, "Sentence {at}. ");
    }
    let mut ask = |_: &str, _: Draw| said("an answer");
    let report = measure(&only(&long), 41, None, &mut ask, &mut unforced);
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
    let mut ask = |prompt: &str, _: Draw| {
        if !prompt.contains("First") {
            said("something else entirely here")
        } else if prompt.contains("Second") {
            said("the answer is four words")
        } else {
            said("the answer is five words")
        }
    };
    let report = measure(&only("First. Second."), 41, None, &mut ask, &mut unforced);
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
    let mut ask = |prompt: &str, _: Draw| {
        if prompt.contains(NO_INSTRUCTION) {
            said("one two three different")
        } else if prompt.contains("Steer") {
            said("one two three four")
        } else {
            said("utterly different words entirely")
        }
    };
    let report = measure(
        &only("Steer this. Inert here."),
        41,
        None,
        &mut ask,
        &mut unforced,
    );
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
    let padded = with_inert(&parts_of("First. Second. Third.", Unit::Sentence));
    assert!(
        padded.contains(NO_INSTRUCTION),
        "the inert sentence is in the prompt: {padded}"
    );
    // Second from the end, so removing it disturbs what follows — which is
    // what removing a clause does.
    assert_eq!(padded, format!("First. Second. {NO_INSTRUCTION} Third."));
    // A prompt of one sentence still gets one, before the only sentence.
    assert_eq!(
        with_inert(&parts_of("Only.", Unit::Sentence)),
        format!("{NO_INSTRUCTION} Only.")
    );
}

/// A document keeps its line breaks when a part is taken out of it.
///
/// F: sentences were rejoined with a space, so every ablation of a multi-line
/// prompt was also a reformatting of it, and the baseline was the only
/// variant that had its line breaks (B-430).
#[test]
fn a_part_removed_leaves_the_document_as_written() {
    let text = "Be brief.\n- Do one thing.\n- Say when done.\n\nNever guess.";
    let sentences = parts_of(text, Unit::Sentence);
    assert_eq!(
        sentences
            .iter()
            .map(|part| part.after.as_str())
            .collect::<Vec<_>>(),
        vec!["\n", "\n", "\n\n", ""],
        "each part carries what followed it: {sentences:?}"
    );
    assert_eq!(
        joined(&sentences),
        text,
        "put back together, it is the text"
    );
    assert_eq!(
        without(&sentences, 1),
        "Be brief.\n- Say when done.\n\nNever guess."
    );
    assert_eq!(
        with_inert(&sentences),
        format!("Be brief.\n- Do one thing.\n- Say when done.\n\n{NO_INSTRUCTION}\n\nNever guess."),
        "the control takes the separator of the part it follows"
    );
}

/// A text with blank lines is paragraphs, and a paragraph holds its lines.
#[test]
fn a_document_with_blank_lines_is_taken_apart_by_paragraph() {
    let text = "You are a dungeon master.\nKeep the party moving.\n\nNever roll for the \
                players.\n\n\nDescribe rooms in two sentences.";
    assert_eq!(Unit::for_text(text), Unit::Paragraph);
    assert_eq!(Unit::for_text("One. Two.\nThree."), Unit::Phrase);
    let paragraphs = parts_of(text, Unit::Paragraph);
    assert_eq!(
        paragraphs
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "You are a dungeon master.\nKeep the party moving.",
            "Never roll for the players.",
            "Describe rooms in two sentences.",
        ]
    );
    assert_eq!(joined(&paragraphs), text);
    // A rule of dashes between paragraphs is not a paragraph.
    let ruled = parts_of("First.\n\n---\n\nSecond.", Unit::Paragraph);
    assert_eq!(ruled.len(), 2, "{ruled:?}");
    assert_eq!(without(&ruled, 0), "Second.");
    assert_eq!(without(&ruled, 1), "First.");
}

/// The whole prompt is what is asked — its last paragraph is a part like any
/// other, not a question held out — and the cap is the caller's.
#[test]
fn the_whole_prompt_is_asked_and_the_cap_is_chosen() {
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
        said("an answer")
    };
    let taken = Taken {
        text: "Be terse.\n\nBe kind.\n\nBe right.\n\nWhat is 2 + 2?",
        by: None,
        most: Some(2),
        extras: Extras::NONE,
    };
    let report = measure(&taken, 41, None, &mut ask, &mut unforced);
    assert_eq!(report.unit, Unit::Paragraph);
    assert!(!report.unit_chosen, "the text decided");
    assert_eq!(report.most, 2);
    assert_eq!(report.clauses.len(), 2);
    assert_eq!(report.clauses_over_the_cap, 2);
    let asked = asked.borrow();
    assert!(
        asked
            .iter()
            .all(|prompt| prompt.ends_with("\n\nWhat is 2 + 2?")),
        "under a cap of two the question at the end was never removed: {asked:?}"
    );
    assert_eq!(
        asked.first().map(String::as_str),
        Some("Be terse.\n\nBe kind.\n\nBe right.\n\nWhat is 2 + 2?")
    );
    assert_eq!(
        asked.get(1).map(String::as_str),
        Some("Be kind.\n\nBe right.\n\nWhat is 2 + 2?")
    );
    drop(asked);
    // The caller can overrule the text.
    let by_sentence = Taken {
        by: Some(Unit::Sentence),
        most: None,
        ..taken
    };
    let report = measure(&by_sentence, 41, None, &mut ask, &mut unforced);
    assert_eq!(report.unit, Unit::Sentence);
    assert!(report.unit_chosen);
    assert_eq!(report.most, MOST_CLAUSES);
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
    let mut ask = |prompt: &str, _: Draw| {
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
            first: if prompt.contains("Two.") {
                Some(1)
            } else {
                Some(17)
            },
            kept: usize::from(prompt.contains("Two.")),
            of: opening.len(),
        })
    };
    let report = measure(&only("One. Two. Three."), 41, None, &mut ask, &mut force);
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
    let mut ask = |_: &str, _: Draw| said("");
    let mut force = |_: &str, _: &[usize]| Some(Held::default());
    let report = measure(&only("One. Two."), 41, None, &mut ask, &mut force);
    assert!(report.clauses.iter().all(|held| held.held.is_none()));
    assert_eq!(report.floor_held, None);
}

/// The rank reading grouped by part: each token goes to the part the prompt's
/// cursor is in, a separator to none, and a piece the prompt does not have
/// next is placed nowhere rather than guessed (B-433, A19).
#[test]
fn the_rank_reading_is_grouped_by_part_and_a_stray_piece_is_placed_nowhere() {
    let parts = parts_of("Answer in one word.\n\nWhat colour is it?", Unit::Paragraph);
    assert_eq!(parts.len(), 2);
    let ranked: Vec<(String, Rank)> = [
        ("<|im_start|>", None),
        ("Answer", None),
        (" in", Some(3)),
        (" one", Some(7)),
        (" word", Some(32)),
        (".", Some(1)),
        ("\n\n", Some(1)),
        ("What", Some(1)),
        (" colour", Some(18)),
        (" is", Some(1)),
        (" it", Some(2)),
        ("?", Some(1)),
    ]
    .into_iter()
    .map(|(text, rank)| (text.to_owned(), rank.map_or(Rank::PastDepth, Rank::At)))
    .collect();
    let (by_part, nowhere) = surprise_by_part(&parts, &ranked);
    assert_eq!(
        nowhere, 1,
        "the template piece is the prompt's next text nowhere"
    );
    assert_eq!(
        by_part,
        vec![
            Surprise {
                tokens: 5,
                first_choice: 1,
                past_depth: 1,
                no_context: 0,
            },
            Surprise {
                tokens: 5,
                first_choice: 3,
                past_depth: 0,
                no_context: 0,
            },
        ]
    );
    // A reading cut off part-way places what it read and nothing more.
    let (short, nowhere) = surprise_by_part(&parts, ranked.get(..4).unwrap_or_default());
    assert_eq!(nowhere, 1);
    assert_eq!(short.first().map(|held| held.tokens), Some(3));
    assert_eq!(short.get(1).map(|held| held.tokens), Some(0));
}

/// The inert sentence lands at every position, with the separator of the
/// part before it, and at the end the last part takes the separator so the
/// two are not run together (B-434).
#[test]
fn the_inert_sentence_can_be_put_at_any_position() {
    let parts = parts_of("One.\n\nTwo.\n\nThree.", Unit::Paragraph);
    let inert = NO_INSTRUCTION;
    assert_eq!(
        with_inert_at(&parts, 0),
        format!("{inert}\n\nOne.\n\nTwo.\n\nThree.")
    );
    assert_eq!(
        with_inert_at(&parts, 1),
        format!("One.\n\n{inert}\n\nTwo.\n\nThree.")
    );
    assert_eq!(with_inert_at(&parts, 2), with_inert(&parts));
    assert_eq!(
        with_inert_at(&parts, 3),
        format!("One.\n\nTwo.\n\nThree.\n\n{inert}")
    );
    // Past the end is the end, not a panic.
    assert_eq!(with_inert_at(&parts, 9), with_inert_at(&parts, 3));
    // The prompt with the sentence removed again is the prompt as written.
    let sentences = parts_of("Be terse. Be kind.", Unit::Sentence);
    assert_eq!(
        with_inert_at(&sentences, 2),
        format!("Be terse. Be kind. {inert}")
    );
}

/// Asked for, the floor is drawn at every position — one generation a
/// position, the one already drawn reused — and the report carries the
/// least, the middle and the most of them; not asked for, none is spent
/// and the spread is absent rather than one number wide (B-434, A7).
#[test]
fn the_floor_at_every_position_is_a_spread_and_costs_a_generation_each() {
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
        // The inert sentence moves the answer more the earlier it lands.
        match prompt.find(NO_INSTRUCTION) {
            Some(0) => said("a b c d e f g h"),
            Some(_) if prompt.starts_with("One.\n\nNothing") => said("one b c d e f g h"),
            Some(_) if prompt.ends_with(NO_INSTRUCTION) => said("one two three d e f g h"),
            Some(_) => said("one two c d e f g h"),
            None => said("one two three four e f g h"),
        }
    };
    let taken = Taken {
        text: "One.\n\nTwo.\n\nThree.",
        by: None,
        most: None,
        extras: Extras::NONE.with(Extra::Floors, true),
    };
    let report = measure(&taken, 41, None, &mut ask, &mut unforced);
    let floors = report.floors.clone().unwrap_or_default();
    assert_eq!(
        floors.iter().map(|at| at.position).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        floors.get(2).map(|at| at.moved),
        Some(report.floor),
        "the position already drawn is the floor itself"
    );
    // One for the baseline, three removals, one control, and three further
    // positions: the drawn position is not asked twice.
    assert_eq!(asked.borrow().len(), 8);
    let spread = report.floor_spread().expect("asked for, so present");
    assert_eq!(spread.least, floors.get(3).map_or(0, |at| at.moved));
    assert_eq!(spread.most, floors.first().map_or(0, |at| at.moved));
    assert_eq!(
        spread.middle,
        floors.get(2).map_or(0, |at| at.moved),
        "the upper median of four draws is the third smallest"
    );
    assert!(spread.least < spread.middle && spread.middle < spread.most);

    asked.borrow_mut().clear();
    let one_draw = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&one_draw, 41, None, &mut ask, &mut unforced);
    assert_eq!(report.floors, None);
    assert_eq!(report.floor_spread(), None);
    assert_eq!(asked.borrow().len(), 5);
}

/// Each part alone is the part as the whole prompt, the first `most` of
/// them, read against the answer as written; the control is the inert
/// sentence alone; a generation each, and none where not asked (B-435).
#[test]
fn each_part_alone_is_read_against_the_answer_as_written_and_costs_a_generation_each() {
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
        match prompt {
            "One." => said("one two c d"),
            "Two." => said("one two three four"),
            "Three." => said("x y z w"),
            NO_INSTRUCTION => said("hello there friend now"),
            _ => said("one two three four"),
        }
    };
    let taken = Taken {
        text: "One.\n\nTwo.\n\nThree.",
        by: None,
        most: Some(2),
        extras: Extras::NONE.with(Extra::Alone, true),
    };
    let report = measure(&taken, 41, None, &mut ask, &mut unforced);
    let alone = report.alone.clone().expect("asked for, so present");
    assert_eq!(alone.len(), 2, "the first `most` parts, like the removals");
    assert_eq!(alone.first().map(|read| read.moved), Some(500_000));
    assert_eq!(
        alone.first().map(|read| read.answer.as_str()),
        Some("one two c d")
    );
    assert_eq!(
        alone.get(1).map(|read| read.moved),
        Some(0),
        "a part whose answer alone is the answer as written carries all of it"
    );
    let floor = report.alone_floor.clone().expect("asked for, so present");
    assert_eq!(floor.moved, 1_000_000);
    assert_eq!(floor.answer, "hello there friend now");
    assert_eq!(
        asked
            .borrow()
            .iter()
            .filter(|prompt| prompt.as_str() == NO_INSTRUCTION)
            .count(),
        1,
        "the control alone is asked as itself, nothing joined to it"
    );
    // One for the baseline, two removals, one control, two alone, one
    // control alone.
    assert_eq!(asked.borrow().len(), 7);

    asked.borrow_mut().clear();
    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced);
    assert_eq!(report.alone, None);
    assert_eq!(report.alone_floor, None);
    assert_eq!(asked.borrow().len(), 4);
}

/// The prompt grown from the front is read one prefix at a time, never the
/// whole — whose answer is the baseline — and costs a generation a prefix
/// (B-436).
#[test]
fn the_prompt_grown_from_the_front_is_read_short_of_the_whole_and_costs_a_generation_each() {
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
        match prompt {
            "One." => said("a b c d"),
            "One.\n\nTwo." => said("one two c d"),
            "One.\n\nTwo.\n\nThree." => said("one two three d"),
            _ => said("one two three four"),
        }
    };
    let taken = Taken {
        text: "One.\n\nTwo.\n\nThree.\n\nFour.",
        by: None,
        most: Some(3),
        extras: Extras::NONE.with(Extra::Prefixes, true),
    };
    assert_eq!(Extra::Prefixes.generations(4, 3), 3);
    assert_eq!(
        Extra::Prefixes.generations(4, 4),
        3,
        "the whole is never a prefix read"
    );
    assert_eq!(Extra::Prefixes.generations(1, 1), 0);
    let report = measure(&taken, 41, None, &mut ask, &mut unforced);
    let prefixes = report.prefixes.clone().expect("asked for, so present");
    let moved: Vec<u64> = prefixes.iter().map(|read| read.moved).collect();
    assert_eq!(
        moved,
        vec![1_000_000, 500_000, 250_000],
        "the answer arrives a part at a time"
    );
    assert_eq!(
        prefixes.first().map(|read| read.answer.as_str()),
        Some("a b c d")
    );
    // One for the baseline, three removals, one control, three prefixes.
    assert_eq!(asked.borrow().len(), 8);
    assert_eq!(
        asked
            .borrow()
            .iter()
            .filter(|prompt| prompt.as_str() == "One.\n\nTwo.\n\nThree.\n\nFour.")
            .count(),
        1,
        "the whole prompt is asked once, as the baseline"
    );

    asked.borrow_mut().clear();
    let capped = Taken {
        most: Some(2),
        ..taken
    };
    let report = measure(&capped, 41, None, &mut ask, &mut unforced);
    assert_eq!(
        report.prefixes.map(|prefixes| prefixes.len()),
        Some(2),
        "as many prefixes as parts removed"
    );

    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced);
    assert_eq!(report.prefixes, None);
}

/// Each part is swapped with the one after it, every word kept and the
/// separators left in place, one generation a pair and never the last
/// part with nothing; not asked for, the reading is absent (B-437, A7).
#[test]
fn neighbours_are_swapped_in_turn_with_the_breaks_left_where_they_were() {
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
        match prompt {
            "Two.\n\nOne.\nThree." => said("two one three"),
            "One.\n\nThree.\nTwo." => said("one three two"),
            _ => said("one two three"),
        }
    };
    let taken = Taken {
        text: "One.\n\nTwo.\nThree.",
        by: Some(Unit::Sentence),
        most: None,
        extras: Extras::NONE.with(Extra::Swaps, true),
    };
    assert_eq!(
        Extra::Swaps.generations(3, 3),
        2,
        "a pair a part but the last"
    );
    assert_eq!(
        Extra::Swaps.generations(3, 1),
        1,
        "capped like the removals"
    );
    assert_eq!(Extra::Swaps.generations(1, 1), 0);
    assert_eq!(swapped(&taken.parts(), 0), "Two.\n\nOne.\nThree.");
    assert_eq!(swapped(&taken.parts(), 1), "One.\n\nThree.\nTwo.");
    assert_eq!(
        swapped(&taken.parts(), 2),
        "One.\n\nTwo.\nThree.",
        "the last part has no neighbour after it"
    );
    let report = measure(&taken, 41, None, &mut ask, &mut unforced);
    let swaps = report.swaps.clone().expect("asked for, so present");
    let moved: Vec<u64> = swaps.iter().map(|read| read.moved).collect();
    assert_eq!(moved.len(), 2);
    assert!(
        moved.iter().all(|&moved| moved > 0),
        "every swap moved the answer: {moved:?}"
    );
    assert_eq!(
        swaps.first().map(|read| read.answer.as_str()),
        Some("two one three")
    );
    // One for the baseline, three removals, one control, two swaps.
    assert_eq!(asked.borrow().len(), 7);

    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced);
    assert_eq!(report.swaps, None);
}

/// A phrase is the part of a sentence a comma, semicolon or colon ends —
/// the unit an instruction is written in — and its punctuation stays with
/// it, as a sentence keeps its full stop.
#[test]
fn a_sentence_is_taken_apart_by_phrase_at_its_commas() {
    let phrases = parts_of(
        "As a senior engineer, answer in one word: what is 1,000 plus a::b?",
        Unit::Phrase,
    );
    assert_eq!(
        phrases
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "As a senior engineer,",
            "answer in one word:",
            "what is 1,000 plus a::b?"
        ],
        "a comma inside a number and a colon inside a name end nothing"
    );
    assert_eq!(
        without(&phrases, 0),
        "answer in one word: what is 1,000 plus a::b?"
    );
    // A cut that leaves fewer than three words on a side is not made.
    let short = |text: &str| {
        parts_of(text, Unit::Phrase)
            .into_iter()
            .map(|part| part.text)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        short("The party steps into a cold, dripping cave. Yes, we go in; the torch is lit."),
        vec![
            "The party steps into a cold, dripping cave.",
            "Yes, we go in;",
            "the torch is lit."
        ],
        "a comma in a noun phrase and a one-word opener cut nothing"
    );
    assert_eq!(
        short("Take red, green, blue:\nmix them."),
        vec!["Take red, green, blue:\nmix them."],
        "a list's commas cut nothing, and the line break the colon led to is kept as written"
    );
    assert_eq!(Unit::for_text("Be terse, be kind."), Unit::Phrase);
}

/// A word ends at whitespace and keeps the punctuation attached to it;
/// removing one leaves the rest as written.
#[test]
fn a_prompt_is_taken_apart_by_word_at_its_spaces() {
    let words = parts_of("Answer in one word.\nWhat colour?", Unit::Word);
    assert_eq!(
        words
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Answer", "in", "one", "word.", "What", "colour?"]
    );
    assert_eq!(
        words.get(3).map(|part| part.after.as_str()),
        Some("\n"),
        "the line break travels with the word before it"
    );
    assert_eq!(without(&words, 2), "Answer in word.\nWhat colour?");
    assert_eq!(joined(&words), "Answer in one word.\nWhat colour?");
}

/// The rank reading by word: each word's pieces and the rank of its first,
/// placed in the part it begins in; a template piece falls in no word.
#[test]
fn the_reading_is_placed_by_word() {
    let parts = parts_of("Be very terse, be so meticulous.", Unit::Phrase);
    let ranked: Vec<(String, Rank)> = vec![
        ("<|im_start|>".to_owned(), Rank::At(1)),
        ("Be".to_owned(), Rank::At(3)),
        (" very".to_owned(), Rank::At(1)),
        (" terse".to_owned(), Rank::PastDepth),
        (",".to_owned(), Rank::At(1)),
        (" be".to_owned(), Rank::At(1)),
        (" so".to_owned(), Rank::At(2)),
        (" met".to_owned(), Rank::At(40)),
        ("icul".to_owned(), Rank::At(1)),
        ("ous".to_owned(), Rank::At(1)),
        (".".to_owned(), Rank::At(1)),
    ];
    let (words, nowhere) = expected_by_word(&parts, &ranked);
    assert_eq!(nowhere, 1, "the marker is in no word");
    let word = |text: &str, pieces, rank, first_choice, part| Expected {
        text: text.to_owned(),
        pieces,
        rank,
        first_choice,
        unread: 0,
        part,
    };
    assert_eq!(
        words,
        vec![
            word("Be", 1, Some(3), 0, Some(1)),
            word("very", 1, Some(1), 1, Some(1)),
            word("terse,", 2, None, 1, Some(1)),
            word("be", 1, Some(1), 1, Some(2)),
            word("so", 1, Some(2), 0, Some(2)),
            word("meticulous.", 4, Some(40), 3, Some(2)),
        ]
    );
}

#[test]
fn a_first_piece_nothing_preceded_is_unread_not_a_first_choice_and_strands_nothing() {
    // A prompt that went with no turn markers has nothing before its first
    // piece, so that piece was never ranked. It must still be placed — else
    // the walk never advances and every later piece is nowhere (F160).
    let parts = parts_of("Be terse.", Unit::Word);
    let ranked: Vec<(String, Rank)> = vec![
        ("Be".to_owned(), Rank::NoContext),
        (" terse".to_owned(), Rank::At(2)),
        (".".to_owned(), Rank::At(1)),
    ];
    let (words, nowhere) = expected_by_word(&parts, &ranked);
    assert_eq!(nowhere, 0, "every piece is in a word");
    assert_eq!(
        words,
        vec![
            Expected {
                text: "Be".to_owned(),
                pieces: 1,
                rank: None,
                first_choice: 0,
                unread: 1,
                part: Some(1),
            },
            Expected {
                text: "terse.".to_owned(),
                pieces: 2,
                rank: Some(2),
                first_choice: 1,
                unread: 0,
                part: Some(2),
            },
        ]
    );
    let (by_part, nowhere) = surprise_by_part(&parts, &ranked);
    assert_eq!(nowhere, 0);
    assert_eq!(
        by_part.iter().map(|part| part.no_context).sum::<usize>(),
        1,
        "the unread piece is counted, not called past the depth"
    );
    assert_eq!(by_part.iter().map(|part| part.past_depth).sum::<usize>(), 0);
}

/// What the model spent before its answer is carried from every generation
/// into the report: the prompt as written, each part removed, and the
/// control, so a part that costs a long thought is visible beside one that
/// changes what is said (B-455).
#[test]
fn the_thoughts_cost_reaches_the_report() {
    // A model that thinks longer the more it is given: the whole prompt
    // costs most, each removal less, and the control — a sentence longer
    // than the whole — most of all.
    let mut ask = |prompt: &str, _: Draw| Answered {
        text: "Blue.".to_owned(),
        tokens: vec![1],
        thought: Some(prompt.len()),
    };
    let report = measure(&only("One. Two. Three."), 41, None, &mut ask, &mut unforced);
    assert_eq!(report.baseline_thought, Some("One. Two. Three.".len()));
    let thoughts: Vec<Option<usize>> = report
        .clauses
        .iter()
        .map(|clause| clause.thought)
        .collect();
    assert!(
        thoughts.iter().all(Option::is_some),
        "every removal's thought is counted: {thoughts:?}"
    );
    assert!(
        thoughts
            .iter()
            .flatten()
            .all(|thought| *thought < "One. Two. Three.".len()),
        "a shorter prompt is a shorter thought here: {thoughts:?}"
    );
    // The control is measured by the same operation, so its cost is a
    // figure and not a dash.
    assert!(report.floor_thought.is_some(), "the control's own thought");
}

/// A model that thinks nowhere counts nothing, and *not counted* is not
/// nought (A7).
#[test]
fn a_turn_with_no_thought_counts_none() {
    let mut ask = |_: &str, _: Draw| said("Blue.");
    let report = measure(&only("One. Two."), 41, None, &mut ask, &mut unforced);
    assert_eq!(report.baseline_thought, None);
    assert_eq!(report.floor_thought, None);
    assert!(report.clauses.iter().all(|clause| clause.thought.is_none()));
}
