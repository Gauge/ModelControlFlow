use super::Arriving;
use crate::screen::{Ink, Screen};

fn a_transfer(id: u64, state: &str, arrived: u64, whole: u64) -> Arriving {
    Arriving {
        id,
        reference: "owner/model".to_owned(),
        file: "a-model.gguf".to_owned(),
        part: 1,
        parts: 1,
        arrived,
        whole,
        state: state.to_owned(),
        why: None,
    }
}

fn drawn(queue: &[Arriving]) -> String {
    let mut screen = Screen::new(90, 30);
    super::draw(&mut screen, 2, queue, 0);
    screen
        .rendered()
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn an_empty_queue_says_how_to_put_something_in_it() {
    let shown = drawn(&[]);
    assert!(shown.contains("Nothing on its way"), "{shown}");
    assert!(
        shown.contains("downloads add"),
        "an empty list that does not say what to do next is a dead end: {shown}"
    );
}

#[test]
fn every_transfer_is_shown_with_how_far_along_it_is() {
    let shown = drawn(&[
        a_transfer(1, "fetching", 250, 1_000),
        a_transfer(2, "paused", 500, 1_000),
    ]);
    assert!(shown.contains("25%"), "{shown}");
    assert!(shown.contains("50%"), "{shown}");
    assert!(shown.contains("paused"), "{shown}");
    assert!(
        shown.contains("One file on its way"),
        "of those two only the one still fetching is on its way; the paused one is not: \
         {shown}"
    );
}

#[test]
fn a_share_is_only_shown_once_there_is_a_whole_to_be_a_share_of() {
    assert_eq!(a_transfer(1, "queued", 0, 0).per_cent(), None);
    assert_eq!(a_transfer(1, "fetching", 250, 1_000).per_cent(), Some(25));
}

#[test]
fn a_failure_is_shown_in_the_words_the_refusal_used() {
    let mut failed = a_transfer(1, "failed", 10, 1_000);
    failed.why = Some("the hub did not answer".to_owned());
    let shown = drawn(&[failed.clone()]);
    assert!(shown.contains("the hub did not answer"), "{shown}");
    assert_eq!(failed.ink(), Ink::Refusal);
}

#[test]
fn the_keys_this_screen_answers_to_are_written_on_it() {
    let shown = drawn(&[a_transfer(1, "fetching", 1, 2)]);
    for key in ["pause", "carry on", "give up"] {
        assert!(
            shown.contains(key),
            "a key that is not written down is a key nobody presses: {key} is missing \
             from\n{shown}"
        );
    }
}

#[test]
fn what_is_still_going_and_what_has_finished_are_told_apart() {
    for state in ["queued", "fetching", "checking"] {
        assert!(a_transfer(1, state, 0, 0).under_way(), "{state}");
        assert!(!a_transfer(1, state, 0, 0).settled(), "{state}");
    }
    for state in ["done", "failed", "cancelled"] {
        assert!(!a_transfer(1, state, 0, 0).under_way(), "{state}");
        assert!(a_transfer(1, state, 0, 0).settled(), "{state}");
    }
    assert!(
        !a_transfer(1, "paused", 0, 0).under_way(),
        "a paused transfer is not moving"
    );
    assert!(
        !a_transfer(1, "paused", 0, 0).settled(),
        "and it is not finished either: there is more of it to take"
    );
}
