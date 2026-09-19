use super::{Takes, in_template};

/// Every case here is a shape taken off a template held on the machine this was written
/// on, because the point of the reader is the templates people actually have rather than
/// the ones a reader is easy to write for.
#[test]
fn a_switch_is_read_with_the_side_it_falls_to() {
    let held = in_template(
        "{%- set enable_thinking = enable_thinking if enable_thinking is defined else True %}",
    );
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(
        held.first().map(|held| held.takes.clone()),
        Some(Takes::Switch {
            on_unless_asked: Some(true)
        })
    );
}

#[test]
fn a_switch_that_falls_the_other_way_is_read_that_way() {
    let held = in_template("{%- set low_effort = low_effort | default(false) -%}");
    assert_eq!(
        held.first().map(|held| held.takes.clone()),
        Some(Takes::Switch {
            on_unless_asked: Some(false)
        })
    );
}

/// gpt-oss guards its default the other way round — it asks whether it was given the
/// parameter and stands in for it when it was not — and the value it stands in with is
/// just as much the default as one written after `else`.
#[test]
fn a_default_set_under_a_guard_is_still_the_default() {
    let held = in_template(
        "{%- if reasoning_effort is not defined %}{%- set reasoning_effort = \"medium\" %}\
         {%- endif %}",
    );
    assert_eq!(
        held.first().map(|held| held.takes.clone()),
        Some(Takes::Word {
            allowed: Vec::new(),
            falls_back_to: Some("medium".to_owned())
        })
    );
}

/// Where the template checks what it was handed, that check is the set it accepts.
#[test]
fn a_word_that_is_checked_against_a_set_reads_as_that_set() {
    let held = in_template(
        "{%- set reasoning_effort = reasoning_effort | default('xhigh') %}\
         {%- if reasoning_effort not in ('xhigh', 'medium', 'low') %}\
         {{ raise_exception('no') }}{% endif %}",
    );
    let Some(Takes::Word { allowed, .. }) = held.first().map(|held| held.takes.clone()) else {
        panic!("a checked word reads as a word: {held:?}");
    };
    assert_eq!(allowed, vec!["xhigh", "medium", "low"]);
}

/// What the conversation itself carries is not a setting somebody can choose.
#[test]
fn the_conversation_is_not_a_parameter() {
    let held = in_template("{%- if tools is defined %}{{ tools }}{%- endif %}");
    assert!(held.is_empty(), "{held:?}");
}

/// A property of one message is a fact about that message.
#[test]
fn something_reached_through_a_dot_belongs_to_what_it_hangs_off() {
    let held = in_template(
        "{%- if message.reasoning_content is defined %}{{ message.content | default('', true) }}\
         {%- endif %}",
    );
    assert!(held.is_empty(), "{held:?}");
}

/// A name the template assigns from something else, before anybody asks whether it was
/// given, is the template's own working variable.
#[test]
fn a_working_variable_is_not_a_parameter() {
    let held = in_template(
        "{%- set system_message = messages[0][\"content\"] %}\
         {%- if system_message is defined %}{{ system_message }}{%- endif %}",
    );
    assert!(held.is_empty(), "{held:?}");
}

/// And a template that takes nothing says so, rather than being given something to offer.
#[test]
fn a_template_that_takes_nothing_offers_nothing() {
    let held = in_template("{%- for message in messages %}{{ message.role }}{%- endfor %}");
    assert!(held.is_empty(), "{held:?}");
}

/// Both sides of a switch are worth rendering; a word is worth rendering at every value it
/// admits.
#[test]
fn what_is_worth_asking_an_engine_follows_from_what_it_takes() {
    let switch = in_template("{%- set enable_thinking = enable_thinking | default(true) %}");
    assert_eq!(switch.first().map(|held| held.probes().len()), Some(2));

    let word = in_template(
        "{%- set effort = effort | default('low') %}\
         {%- if effort not in ('low', 'high') %}{{ raise_exception('no') }}{% endif %}",
    );
    assert_eq!(word.first().map(|held| held.probes().len()), Some(2));
}
