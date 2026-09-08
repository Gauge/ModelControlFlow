#![allow(clippy::panic, clippy::expect_used)]

use super::*;

fn said(text: &str) -> Answered {
    Answered {
        text: text.to_owned(),
        tokens: text.split_whitespace().map(str::len).collect(),
        thought: None,
    }
}

fn unforced(_: &str, _: &[usize]) -> Option<Held> {
    None
}

fn quietly(_: Step) {}

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

#[test]
fn the_clauses_are_the_sentences_somebody_wrote() {
    assert_eq!(
        clauses_of("Summarise this. Keep it short! Why? Because."),
        vec!["Summarise this.", "Keep it short!", "Why?", "Because."]
    );
    assert_eq!(
        clauses_of("Do the thing\nBe brief"),
        vec!["Do the thing", "Be brief"]
    );
    assert_eq!(clauses_of("...\n\n!!!"), Vec::<String>::new());
    assert_eq!(clauses_of(""), Vec::<String>::new());
}

#[test]
fn a_prompt_without_one_clause_keeps_the_rest() {
    let all = parts_of("One. Two. Three.", Unit::Sentence);
    assert_eq!(without(&all, 1), "One. Three.");
    assert_eq!(without(&all, 0), "Two. Three.");
    assert_eq!(without(&all, 9), "One. Two. Three.");
}

#[test]
fn an_unchanged_answer_says_the_clause_was_not_used() {
    let mut ask = |prompt: &str, _: Draw| {
        if prompt.contains("One") {
            said("the same answer")
        } else {
            said("a different answer")
        }
    };
    let report = measure(
        &only("One. Two."),
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    let unused = report.unused();
    assert_eq!(unused.len(), 1, "one clause changed nothing");
    assert_eq!(
        unused.first().map(|held| held.text.as_str()),
        Some("Two."),
        "removing the second sentence left the answer alone, so it did not reach it"
    );
    assert!(
        report
            .clauses
            .iter()
            .any(|held| held.text == "One." && held.changed)
    );
}

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
        &mut quietly,
    );
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

#[test]
fn a_prompt_of_one_sentence_has_nothing_to_ablate() {
    let mut ask = |_: &str, _: Draw| said("an answer");
    let report = measure(
        &only("Just the one sentence."),
        41,
        Some(warm()),
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    assert!(
        report.clauses.is_empty(),
        "there is no clause whose absence could be observed"
    );
    assert_eq!(report.settled.map(|held| held.asked), Some(SEEDS));
}

#[test]
fn without_a_temperature_the_seeds_are_not_asked() {
    let mut asked = 0_usize;
    let mut ask = |_: &str, _: Draw| {
        asked += 1;
        said("an answer")
    };
    let report = measure(
        &only("One. Two."),
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    assert_eq!(
        report.settled, None,
        "not settled, not unsettled: not asked"
    );
    assert_eq!(asked, 4);
}

#[test]
fn a_prompt_that_settles_the_answer_reports_one_answer() {
    let mut ask = |_: &str, _: Draw| said("the one answer");
    let report = measure(
        &only("One. Two."),
        41,
        Some(warm()),
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    let settled = report.settled.expect("a temperature was stated");
    assert_eq!(settled.distinct, 1);
    assert_eq!(settled.spread, 0);
    assert_eq!(settled.from_greedy, 0);
    assert_eq!(settled.temperature, Thousandths(700));
}

#[test]
fn a_prompt_that_does_not_settle_reports_how_many_answers_and_how_far_apart() {
    let mut ask = |_: &str, draw: Draw| {
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
        &mut quietly,
    );
    let settled = report.settled.expect("a temperature was stated");
    assert_eq!(settled.distinct, SEEDS, "every seed gave its own answer");
    assert_eq!(settled.spread, 1_000_000);
    assert_eq!(settled.from_greedy, 1_000_000);
}

#[test]
fn a_prompt_longer_than_the_cap_says_what_was_left_out() {
    let mut long = String::new();
    for at in 0..MOST_CLAUSES + 3 {
        use std::fmt::Write as _;
        let _wrote = write!(long, "Sentence {at}. ");
    }
    let mut ask = |_: &str, _: Draw| said("an answer");
    let report = measure(
        &only(&long),
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    assert_eq!(report.clauses.len(), MOST_CLAUSES);
    assert_eq!(report.clauses_over_the_cap, 3, "and it says how many");
}

#[test]
fn how_much_moved_is_counted_in_words() {
    assert_eq!(moved_by("the same words", "the same words"), 0);
    assert_eq!(moved_by("", ""), 0);
    assert_eq!(moved_by("one two", "three four"), 1_000_000);
    assert_eq!(moved_by("a b c d", "a b c e"), 250_000);
    assert!(moved_by("a b", "a b c d") <= 1_000_000);
    assert_eq!(moved_by("a b", "a b c d"), 500_000);
    assert_eq!(moved_by("hello world", "hallo world"), 500_000);
}

#[test]
fn a_full_stop_is_not_a_different_river() {
    assert_eq!(moved_by("Nile.", "Nile"), 0);
    assert_eq!(moved_by("\"Yes,\" she said.", "Yes she said"), 0);
    assert_eq!(moved_by("one two ... three", "one two three"), 0);
    assert_eq!(moved_by("L'Indus.", "LIndus"), 1_000_000);
    assert_eq!(moved_by("x.y", "xy"), 1_000_000);
    assert_eq!(moved_by("a + b", "a - b"), 333_333);
    assert_eq!(moved_by("...", ""), 0);
    assert_eq!(moved_by("Nile.", "L'Indus."), 1_000_000);
}

#[test]
fn a_sentence_that_steered_the_answer_is_told_from_one_that_perturbed_it() {
    let mut ask = |prompt: &str, _: Draw| {
        if !prompt.contains("First") {
            said("something else entirely here")
        } else if prompt.contains("Second") {
            said("the answer is four words")
        } else {
            said("the answer is five words")
        }
    };
    let report = measure(
        &only("First. Second."),
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
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
    assert!(steered.changed && perturbed.changed);
    let barely = report.barely_moved(300_000);
    assert_eq!(barely.len(), 1);
    assert_eq!(
        barely.first().map(|held| held.text.as_str()),
        Some("Second.")
    );
}

#[test]
fn the_floor_is_what_an_inert_sentence_does() {
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
        &mut quietly,
    );
    assert!(
        report.floor > 0,
        "an inert sentence moved the answer, and that is the floor"
    );
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

#[test]
fn the_inert_sentence_goes_where_a_clause_would_be() {
    let padded = with_inert(&parts_of("First. Second. Third.", Unit::Sentence));
    assert!(
        padded.contains(NO_INSTRUCTION),
        "the inert sentence is in the prompt: {padded}"
    );
    assert_eq!(padded, format!("First. Second. {NO_INSTRUCTION} Third."));
    assert_eq!(
        with_inert(&parts_of("Only.", Unit::Sentence)),
        format!("{NO_INSTRUCTION} Only.")
    );
}

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
    let ruled = parts_of("First.\n\n---\n\nSecond.", Unit::Paragraph);
    assert_eq!(ruled.len(), 2, "{ruled:?}");
    assert_eq!(without(&ruled, 0), "Second.");
    assert_eq!(without(&ruled, 1), "First.");
}

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
    let report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
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
    let by_sentence = Taken {
        by: Some(Unit::Sentence),
        most: None,
        ..taken
    };
    let report = measure(
        &by_sentence,
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    assert_eq!(report.unit, Unit::Sentence);
    assert!(report.unit_chosen);
    assert_eq!(report.most, MOST_CLAUSES);
}

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
    let report = measure(
        &only("One. Two. Three."),
        41,
        None,
        &mut ask,
        &mut force,
        &mut quietly,
    );
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

#[test]
fn an_empty_answer_has_no_opening_to_force() {
    let mut ask = |_: &str, _: Draw| said("");
    let mut force = |_: &str, _: &[usize]| Some(Held::default());
    let report = measure(
        &only("One. Two."),
        41,
        None,
        &mut ask,
        &mut force,
        &mut quietly,
    );
    assert!(report.clauses.iter().all(|held| held.held.is_none()));
    assert_eq!(report.floor_held, None);
}

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
    let (short, nowhere) = surprise_by_part(&parts, ranked.get(..4).unwrap_or_default());
    assert_eq!(nowhere, 1);
    assert_eq!(short.first().map(|held| held.tokens), Some(3));
    assert_eq!(short.get(1).map(|held| held.tokens), Some(0));
}

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
    assert_eq!(with_inert_at(&parts, 9), with_inert_at(&parts, 3));
    let sentences = parts_of("Be terse. Be kind.", Unit::Sentence);
    assert_eq!(
        with_inert_at(&sentences, 2),
        format!("Be terse. Be kind. {inert}")
    );
}

#[test]
fn the_floor_at_every_position_is_a_spread_and_costs_a_generation_each() {
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
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
    let report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
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
    let report = measure(&one_draw, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(report.floors, None);
    assert_eq!(report.floor_spread(), None);
    assert_eq!(asked.borrow().len(), 5);
}

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
    let report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
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
    assert_eq!(asked.borrow().len(), 7);

    asked.borrow_mut().clear();
    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(report.alone, None);
    assert_eq!(report.alone_floor, None);
    assert_eq!(asked.borrow().len(), 4);
}

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
    let report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
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
    let report = measure(&capped, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(
        report.prefixes.map(|prefixes| prefixes.len()),
        Some(2),
        "as many prefixes as parts removed"
    );

    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(report.prefixes, None);
}

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
    let report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
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
    assert_eq!(asked.borrow().len(), 7);

    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(report.swaps, None);
}

#[test]
fn each_form_keeps_every_word_and_changes_only_the_dress() {
    let parts = parts_of("Be terse.\n\nAnswer in\nFrench.", Unit::Paragraph);
    assert_eq!(
        Form::OneLine.render(&parts).as_deref(),
        Some("Be terse. Answer in French.")
    );
    assert_eq!(
        Form::Bullets.render(&parts).as_deref(),
        Some("- Be terse.\n- Answer in French.")
    );
    assert_eq!(
        Form::Numbered.render(&parts).as_deref(),
        Some("1. Be terse.\n2. Answer in French.")
    );
    assert_eq!(
        Form::Headings.render(&parts).as_deref(),
        Some("## 1\n\nBe terse.\n\n## 2\n\nAnswer in French.")
    );
    assert_eq!(
        Form::Tags.render(&parts).as_deref(),
        Some("<instruction>Be terse.</instruction>\n<instruction>Answer in French.</instruction>")
    );
    assert_eq!(
        Form::Capitals.render(&parts).as_deref(),
        Some("BE TERSE.\n\nANSWER IN\nFRENCH."),
        "capitals keep the document as written"
    );
    assert_eq!(Extra::Forms.generations(2, 2), 6);
    assert_eq!(Extra::Forms.generations(1, 1), 2);
    assert!(Extra::Forms.at_most());
    assert!(!Extra::Swaps.at_most());
}

#[test]
fn a_form_already_worn_is_not_asked_and_one_part_is_nothing_to_list() {
    let taken = Taken {
        text: "Be terse.\n\nAnswer in\nFrench.",
        by: Some(Unit::Paragraph),
        most: None,
        extras: Extras::NONE.with(Extra::Forms, true),
    };
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let mut ask = |prompt: &str, _: Draw| {
        asked.borrow_mut().push(prompt.to_owned());
        if prompt.starts_with("- ") {
            said("oui")
        } else {
            said("yes")
        }
    };
    let report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
    let forms = report.forms.clone().expect("asked for, so present");
    assert_eq!(forms.len(), 6);
    assert!(
        forms
            .iter()
            .all(|formed| matches!(formed.outcome, Rendering::Read(_))),
        "a two-line prompt is in none of the forms: {forms:?}"
    );
    let bullets = forms
        .iter()
        .find(|formed| formed.form == Form::Bullets)
        .and_then(|formed| match &formed.outcome {
            Rendering::Read(read) => Some(read),
            Rendering::NotRendered(_) => None,
        })
        .expect("the bullets were read");
    assert!(bullets.moved > 0, "the list moved the answer");
    assert_eq!(bullets.answer, "oui");
    assert_eq!(asked.borrow().len(), 10);

    let one_line = Taken {
        text: "Be terse. Answer in French.",
        by: Some(Unit::Sentence),
        ..taken
    };
    asked.borrow_mut().clear();
    let report = measure(&one_line, 41, None, &mut ask, &mut unforced, &mut quietly);
    let forms = report.forms.clone().expect("asked for");
    assert_eq!(
        forms.first().map(|formed| &formed.outcome),
        Some(&Rendering::NotRendered(AS_WRITTEN))
    );
    assert_eq!(asked.borrow().len(), 9, "five forms, not six");

    let one_part = Taken {
        text: "Be terse.",
        ..taken
    };
    asked.borrow_mut().clear();
    let report = measure(&one_part, 41, None, &mut ask, &mut unforced, &mut quietly);
    let forms = report.forms.clone().expect("asked for");
    let not_rendered: Vec<&str> = forms
        .iter()
        .filter_map(|formed| match formed.outcome {
            Rendering::NotRendered(why) => Some(why),
            Rendering::Read(_) => None,
        })
        .collect();
    assert_eq!(
        not_rendered,
        vec![AS_WRITTEN, ONE_PART, ONE_PART, ONE_PART, ONE_PART],
        "{forms:?}"
    );
    assert!(
        matches!(
            forms.last().map(|formed| &formed.outcome),
            Some(Rendering::Read(_))
        ),
        "capitals dress a whole of one part"
    );
    assert_eq!(asked.borrow().len(), 3);

    let not_asked = Taken {
        extras: Extras::NONE,
        ..taken
    };
    let report = measure(&not_asked, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(report.forms, None);
}

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

#[test]
fn the_thoughts_cost_reaches_the_report() {
    let mut ask = |prompt: &str, _: Draw| Answered {
        text: "Blue.".to_owned(),
        tokens: vec![1],
        thought: Some(prompt.len()),
    };
    let report = measure(
        &only("One. Two. Three."),
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    assert_eq!(report.baseline_thought, Some("One. Two. Three.".len()));
    let thoughts: Vec<Option<usize>> = report.clauses.iter().map(|clause| clause.thought).collect();
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
    assert!(report.floor_thought.is_some(), "the control's own thought");
}

#[test]
fn a_turn_with_no_thought_counts_none() {
    let mut ask = |_: &str, _: Draw| said("Blue.");
    let report = measure(
        &only("One. Two."),
        41,
        None,
        &mut ask,
        &mut unforced,
        &mut quietly,
    );
    assert_eq!(report.baseline_thought, None);
    assert_eq!(report.floor_thought, None);
    assert!(report.clauses.iter().all(|clause| clause.thought.is_none()));
}

#[test]
fn every_generation_is_said_before_it_is_asked_and_the_plan_is_the_count() {
    let mut asked = 0_usize;
    let mut ask = |_: &str, _: Draw| {
        asked += 1;
        said("the same")
    };
    let mut steps = Vec::new();
    let mut say = |step: Step| steps.push(step);
    let taken = Taken {
        text: "One. Two. Three.",
        by: None,
        most: None,
        extras: Extras::named(["floors", "alone", "prefixes", "swaps", "forms"]),
    };
    let _report = measure(&taken, 41, Some(warm()), &mut ask, &mut unforced, &mut say);
    assert_eq!(steps.len(), asked, "a generation went unannounced");
    let of = planned(&taken, true);
    assert_eq!(asked, of, "the plan said {of}; the run asked {asked}");
    assert!(
        steps
            .iter()
            .enumerate()
            .all(|(at, step)| step.count == at + 1 && step.of == of),
        "the steps are not counted one at a time from one to the plan: {steps:?}"
    );
    let words: Vec<&str> = steps.iter().map(|step| step.what.as_str()).collect();
    assert_eq!(words[0], "the prompt as written");
    assert_eq!(words[1], "without part 1 of 3");
    assert_eq!(words[4], "the control sentence added");
    assert!(
        words.contains(&"the control sentence at position 0 of 3"),
        "{words:?}"
    );
    assert!(words.contains(&"part 2 of 3 alone"), "{words:?}");
    assert!(words.contains(&"the control sentence alone"), "{words:?}");
    assert!(words.contains(&"the first part alone"), "{words:?}");
    assert!(words.contains(&"the first 2 parts"), "{words:?}");
    assert!(words.contains(&"parts 2 and 3 swapped"), "{words:?}");
    assert!(words.contains(&"as bullets"), "{words:?}");
    assert_eq!(
        words.last().copied(),
        Some("seed 3 of 3 at temperature 0.700")
    );
}

#[test]
fn one_part_plans_two_generations() {
    let mut asked = 0_usize;
    let mut ask = |_: &str, _: Draw| {
        asked += 1;
        said("the same")
    };
    let taken = only("One sentence only.");
    let _report = measure(&taken, 41, None, &mut ask, &mut unforced, &mut quietly);
    assert_eq!(planned(&taken, false), 2);
    assert_eq!(asked, 2);
}

#[test]
fn a_step_is_said_as_one_line() {
    let body = Value::map([
        ("reporting", Value::text("a model")),
        (
            "step",
            Step {
                what: "without part 2 of 4".to_owned(),
                count: 3,
                of: 12,
            }
            .to_value(),
        ),
    ]);
    assert_eq!(
        step_said(&body).as_deref(),
        Some("generation 3 of 12: without part 2 of 4")
    );
    assert_eq!(step_said(&Value::map([("done", Value::Bool(true))])), None);
}
