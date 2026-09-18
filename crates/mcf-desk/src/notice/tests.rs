use super::{KEPT, Notice, Notices, SETTLES_AFTER, Tone};
use std::time::{Duration, Instant};

fn a_notice(tone: Tone, about: &str, what: &str) -> Notice {
    Notice::new(tone, about, what)
}

#[test]
fn a_second_notice_about_the_same_thing_replaces_the_first() {
    let mut notices = Notices::new();
    notices.working("transfer:7", "Getting one.gguf · 10%");
    notices.working("transfer:7", "Getting one.gguf · 46%");
    notices.working("transfer:7", "Getting one.gguf · 92%");
    assert_eq!(
        notices.len(),
        1,
        "a transfer reporting itself once a second must be one line, not a thousand"
    );
    assert_eq!(
        notices.about("transfer:7").map(|held| held.what.as_str()),
        Some("Getting one.gguf · 92%")
    );
}

#[test]
fn notices_about_different_things_all_stand() {
    let mut notices = Notices::new();
    notices.working("transfer:7", "Getting one.gguf");
    notices.refused("hold", "Could not hold it");
    notices.done("engine:llama.cpp", "Built llama.cpp");
    assert_eq!(notices.len(), 3);
}

#[test]
fn what_finished_fades_and_what_wants_answering_does_not() {
    let mut notices = Notices::new();
    notices.done("hold", "Server stopped — freed 70.6 GB");
    notices.refused("transfer:7", "The hub would not say");
    notices.warning("network", "The hold answers the network without a key");
    notices.working("engine:llama.cpp", "Building llama.cpp");

    let later = Instant::now()
        .checked_add(SETTLES_AFTER + Duration::from_secs(1))
        .expect("a later instant");
    notices.expire(later);

    assert!(
        notices.about("hold").is_none(),
        "a finished thing that wants nothing must go on its own"
    );
    assert!(
        notices.about("transfer:7").is_some(),
        "a refusal is answered, not waited out"
    );
    assert!(
        notices.about("network").is_some(),
        "a warning goes when the condition does"
    );
    assert!(
        notices.about("engine:llama.cpp").is_some(),
        "work is not over because time passed"
    );
}

#[test]
fn nothing_fades_before_its_time() {
    let mut notices = Notices::new();
    notices.done("hold", "Server stopped");
    notices.expire(Instant::now());
    assert_eq!(notices.len(), 1);
}

#[test]
fn the_strip_shows_work_before_a_refusal_and_a_refusal_before_the_rest() {
    let mut notices = Notices::new();
    notices.done("a", "finished");
    notices.warning("b", "careful");
    notices.refused("c", "refused");
    assert_eq!(
        notices.foremost().map(|held| held.tone),
        Some(Tone::Refused)
    );
    notices.working("d", "working");
    assert_eq!(
        notices.foremost().map(|held| held.tone),
        Some(Tone::Working),
        "what is happening now outranks what already happened"
    );
}

#[test]
fn of_two_of_a_kind_the_newer_is_foremost() {
    let mut notices = Notices::new();
    notices.refused("a", "the older one");
    std::thread::sleep(Duration::from_millis(5));
    notices.refused("b", "the newer one");
    assert_eq!(
        notices.foremost().map(|held| held.what.as_str()),
        Some("the newer one")
    );
}

#[test]
fn work_is_taken_back_by_whoever_started_it() {
    let mut notices = Notices::new();
    notices.working("transfer:7", "Getting one.gguf");
    notices.forget("transfer:7");
    assert!(notices.is_empty());
}

#[test]
fn dismissing_the_settled_leaves_what_is_still_happening() {
    let mut notices = Notices::new();
    notices.working("transfer:7", "Getting one.gguf");
    notices.refused("hold", "Could not hold it");
    notices.done("engine", "Built it");
    notices.dismiss_the_settled();
    assert_eq!(notices.len(), 1);
    assert_eq!(
        notices.foremost().map(|held| held.tone),
        Some(Tone::Working)
    );
}

#[test]
fn refusals_are_counted_because_the_strip_says_how_many_want_answering() {
    let mut notices = Notices::new();
    notices.refused("a", "one");
    notices.refused("b", "two");
    notices.done("c", "three");
    assert_eq!(notices.refusals(), 2);
}

#[test]
fn the_list_is_newest_first() {
    let mut notices = Notices::new();
    notices.done("a", "first");
    notices.done("b", "second");
    let recent = notices.recent();
    assert_eq!(
        recent.first().map(|held| held.what.as_str()),
        Some("second")
    );
}

#[test]
fn the_list_does_not_grow_without_end() {
    let mut notices = Notices::new();
    for at in 0..(KEPT * 2) {
        notices.done(format!("thing:{at}"), format!("number {at}"));
    }
    assert_eq!(notices.len(), KEPT);
    assert_eq!(
        notices.recent().first().map(|held| held.what.clone()),
        Some(format!("number {}", KEPT * 2 - 1)),
        "the newest must survive the trim"
    );
}

#[test]
fn a_share_is_kept_within_the_whole_and_absent_when_unknown() {
    let held = a_notice(Tone::Working, "transfer:7", "Getting one.gguf").so_far(Some(1.4));
    assert_eq!(held.share, Some(1.0));
    let none = a_notice(Tone::Working, "transfer:7", "Getting one.gguf").so_far(None);
    assert_eq!(
        none.share, None,
        "a share MCF was not given is not a share of nought"
    );
}

#[test]
fn a_detail_of_nothing_is_no_detail() {
    assert!(
        a_notice(Tone::Done, "a", "b")
            .saying("   ")
            .detail
            .is_none()
    );
    assert!(
        a_notice(Tone::Done, "a", "b")
            .saying("why")
            .detail
            .is_some()
    );
}

#[test]
fn the_clock_reads_as_hours_and_minutes() {
    let said = a_notice(Tone::Done, "a", "b").clock_said();
    let (hours, rest) = said.split_once(':').expect("a clock has a colon in it");
    assert_eq!(hours.len(), 2, "{said}");
    assert!(rest.len() >= 2, "{said}");
    assert!(
        hours.parse::<u8>().is_ok_and(|held| held < 24),
        "not an hour: {said}"
    );
}
