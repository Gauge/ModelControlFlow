use super::{By, Typing, Way};

const ROOM: usize = 1000;

fn typed(said: &str) -> Typing {
    Typing::of(said)
}

#[test]
fn a_field_opens_with_the_caret_after_what_is_already_there() {
    let held = typed("hello");
    assert_eq!(held.caret(), 5);
    assert!(held.selection().is_none());
}

#[test]
fn typing_goes_in_at_the_caret_not_at_the_end() {
    let mut held = typed("helo");
    held.go(Way::Back, By::Character, false);
    held.put("l", ROOM);
    assert_eq!(held.said(), "hello");
    assert_eq!(held.caret(), 4, "the caret follows what was typed");
}

#[test]
fn backspace_takes_the_character_before_the_caret() {
    let mut held = typed("hello");
    held.go(Way::Back, By::Character, false);
    held.rub(Way::Back, By::Character);
    assert_eq!(held.said(), "helo");
    assert_eq!(held.caret(), 3);
}

#[test]
fn delete_takes_the_character_after_the_caret() {
    let mut held = typed("hello");
    held.go(Way::Back, By::Line, false);
    held.rub(Way::On, By::Character);
    assert_eq!(held.said(), "ello");
    assert_eq!(held.caret(), 0);
}

#[test]
fn home_and_end_reach_both_ends() {
    let mut held = typed("hello world");
    held.go(Way::Back, By::Line, false);
    assert_eq!(held.caret(), 0);
    held.go(Way::On, By::Line, false);
    assert_eq!(held.caret(), 11);
}

#[test]
fn a_word_step_crosses_one_word_and_the_space_before_it() {
    let mut held = typed("alpha beta gamma");
    held.go(Way::Back, By::Word, false);
    assert_eq!(held.said().get(held.caret()..), Some("gamma"));
    held.go(Way::Back, By::Word, false);
    assert_eq!(held.said().get(held.caret()..), Some("beta gamma"));
    held.go(Way::On, By::Word, false);
    assert_eq!(held.said().get(held.caret()..), Some(" gamma"));
}

#[test]
fn a_word_rub_takes_the_whole_word() {
    let mut held = typed("alpha beta");
    held.rub(Way::Back, By::Word);
    assert_eq!(held.said(), "alpha ");
}

#[test]
fn holding_shift_keeps_the_anchor_and_makes_a_selection() {
    let mut held = typed("hello");
    held.go(Way::Back, By::Line, false);
    held.go(Way::On, By::Character, true);
    held.go(Way::On, By::Character, true);
    assert_eq!(held.selected(), "he");
    assert_eq!(held.selection(), Some((0, 2)));
}

#[test]
fn typing_over_a_selection_replaces_it() {
    let mut held = typed("hello world");
    held.all();
    held.put("bye", ROOM);
    assert_eq!(held.said(), "bye");
    assert!(held.selection().is_none());
}

#[test]
fn backspace_over_a_selection_takes_the_selection_and_nothing_more() {
    let mut held = typed("hello");
    held.go(Way::Back, By::Line, false);
    held.go(Way::On, By::Character, true);
    held.go(Way::On, By::Character, true);
    held.rub(Way::Back, By::Character);
    assert_eq!(held.said(), "llo");
    assert_eq!(held.caret(), 0);
}

#[test]
fn an_unshifted_arrow_collapses_a_selection_to_its_near_edge() {
    let mut held = typed("hello");
    held.all();
    held.go(Way::Back, By::Character, false);
    assert_eq!(
        held.caret(),
        0,
        "left goes to the start of what was selected"
    );
    held.all();
    held.go(Way::On, By::Character, false);
    assert_eq!(held.caret(), 5, "right goes to the end of it");
}

#[test]
fn select_all_then_copy_yields_the_whole_field() {
    let mut held = typed("hello");
    held.all();
    assert_eq!(held.copied(), Some("hello".to_owned()));
}

#[test]
fn copy_with_nothing_selected_yields_the_whole_field() {
    let held = typed("hello");
    assert_eq!(held.copied(), Some("hello".to_owned()));
    assert!(
        typed("").copied().is_none(),
        "an empty field copies nothing"
    );
}

#[test]
fn cut_returns_the_selection_and_removes_it() {
    let mut held = typed("hello world");
    held.go(Way::Back, By::Line, false);
    held.go(Way::On, By::Word, true);
    assert_eq!(held.cut(), Some("hello".to_owned()));
    assert_eq!(held.said(), " world");
    assert!(held.cut().is_none(), "nothing selected cuts nothing");
}

#[test]
fn a_click_places_the_caret_and_a_drag_selects() {
    let mut held = typed("hello world");
    held.place(6, false);
    assert_eq!(held.caret(), 6);
    assert!(held.selection().is_none());
    held.place(11, true);
    assert_eq!(held.selected(), "world");
}

#[test]
fn a_double_click_takes_the_word_under_it() {
    let mut held = typed("alpha beta gamma");
    held.word_at(8);
    assert_eq!(held.selected(), "beta");
    held.word_at(0);
    assert_eq!(held.selected(), "alpha");
    held.word_at(16);
    assert_eq!(held.selected(), "gamma");
}

#[test]
fn the_limit_counts_characters_and_is_never_exceeded() {
    let mut held = typed("abc");
    held.put("defghij", 5);
    assert_eq!(held.said(), "abcde", "only what fits goes in");
    held.put("x", 5);
    assert_eq!(held.said(), "abcde", "a full field takes nothing more");
}

#[test]
fn a_caret_never_lands_inside_a_character() {
    let mut held = typed("héllo wörld");
    for at in 0..=held.said().len() {
        held.place(at, false);
        assert!(
            held.said().is_char_boundary(held.caret()),
            "caret at {at} landed inside a character"
        );
    }
}

#[test]
fn stepping_through_multi_byte_text_moves_one_character_at_a_time() {
    let mut held = typed("héllo");
    held.go(Way::Back, By::Line, false);
    let mut seen = Vec::new();
    for _ in 0..5 {
        held.go(Way::On, By::Character, false);
        seen.push(held.caret());
    }
    assert_eq!(seen, vec![1, 3, 4, 5, 6], "é is two bytes and one step");
}

#[test]
fn a_null_byte_is_never_taken_in() {
    let mut held = typed("");
    held.put("a\u{0}b", ROOM);
    assert_eq!(held.said(), "ab");
}

#[test]
fn taking_the_text_leaves_the_field_empty_and_the_caret_at_the_start() {
    let mut held = typed("hello");
    assert_eq!(held.take(), "hello");
    assert!(held.is_empty());
    assert_eq!(held.caret(), 0);
}

#[test]
fn every_motion_on_an_empty_field_is_harmless() {
    for way in [Way::Back, Way::On] {
        for by in [By::Character, By::Word, By::Line] {
            let mut held = typed("");
            held.go(way, by, false);
            held.go(way, by, true);
            held.rub(way, by);
            assert_eq!(held.caret(), 0);
            assert!(held.is_empty());
        }
    }
}

#[test]
fn a_press_places_the_caret_and_a_drag_from_there_reaches_back_for_a_selection() {
    let mut held = Typing::of("one two three");
    held.place(4, false);
    assert_eq!(held.caret(), 4);
    assert_eq!(held.selection(), None, "a press alone selects nothing");
    held.place(7, true);
    assert_eq!(held.selection(), Some((4, 7)));
    assert_eq!(held.selected(), "two");
}

#[test]
fn a_drag_that_turns_back_on_itself_selects_the_other_way() {
    let mut held = Typing::of("one two three");
    held.place(7, false);
    held.place(4, true);
    assert_eq!(
        held.selection(),
        Some((4, 7)),
        "a selection is read low to high"
    );
    assert_eq!(held.caret(), 4, "the caret is where the drag ended");
}

#[test]
fn a_second_press_after_a_drag_drops_the_selection() {
    let mut held = Typing::of("one two three");
    held.place(4, false);
    held.place(7, true);
    held.place(0, false);
    assert_eq!(held.selection(), None);
    assert_eq!(held.caret(), 0);
}

#[test]
fn a_double_press_takes_the_word_under_it_whichever_end_is_touched() {
    for at in [4, 5, 6, 7] {
        let mut held = Typing::of("one two three");
        held.word_at(at);
        assert_eq!(held.selected(), "two", "touched at {at}");
    }
}
