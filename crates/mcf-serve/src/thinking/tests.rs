use super::{CONVENTIONAL, Thinking};

#[test]
fn a_template_that_reads_no_level_offers_none() {
    let held = Thinking::in_template("{{ messages }}");
    assert!(!held.reads_a_level());
    assert!(held.levels.is_empty());
    assert!(
        held.said().contains("changes nothing"),
        "and says so rather than offering a setting that does nothing: {}",
        held.said()
    );
}

#[test]
fn a_template_that_only_writes_the_level_out_gets_the_usual_ladder() {
    let held = Thinking::in_template(
        "{%- if reasoning_effort is not defined %}{%- set reasoning_effort = \"medium\" %}\
         {%- endif %}{{- \"Reasoning: \" + reasoning_effort }}",
    );
    assert!(held.reads_a_level());
    assert_eq!(held.variable.as_deref(), Some("reasoning_effort"));
    assert!(
        !held.closed,
        "it validates nothing, so anything passes through"
    );
    assert_eq!(held.levels, CONVENTIONAL.map(str::to_owned).to_vec());
}

#[test]
fn a_template_that_checks_the_level_is_read_for_what_it_allows() {
    let held = Thinking::in_template(
        "{%- set resolved_reasoning_effort = reasoning_effort|default('xhigh') %}\
         {%- if resolved_reasoning_effort not in ('xhigh', 'medium', 'low') %}\
         {{- raise_exception('Unexpected reasoning effort') }}{%- endif %}",
    );
    assert!(
        held.closed,
        "this one raises rather than passing anything on"
    );
    assert_eq!(
        held.levels,
        vec!["low".to_owned(), "medium".to_owned(), "xhigh".to_owned()],
        "and the ladder runs from least thinking to most, whatever order it was written in"
    );
    assert!(held.allows("xhigh"));
    assert!(
        !held.allows("high"),
        "high is not a word this template takes"
    );
}

#[test]
fn a_template_that_reads_a_strength_rather_than_an_effort_is_read_too() {
    let held = Thinking::in_template(
        "{%- set rs = reasoning_strength if reasoning_strength is defined else 'high' -%}\
         {{- 'Reasoning strength: ' + rs -}}",
    );
    assert_eq!(held.variable.as_deref(), Some("reasoning_strength"));
    assert!(held.reads_a_level());
}

#[test]
fn a_template_that_can_turn_thinking_off_offers_that_as_the_lowest_level() {
    let held =
        Thinking::in_template("{%- if enable_thinking %}<think>{% endif %}{{ reasoning_effort }}");
    assert_eq!(held.levels.first().map(String::as_str), Some("none"));
    assert!(held.switch);
}

#[test]
fn a_template_with_no_thinking_switch_is_not_offered_a_level_that_turns_it_off() {
    let held = Thinking::in_template("{{- \"Reasoning: \" + reasoning_effort }}");
    assert!(!held.switch);
    assert!(
        !held.allows("none"),
        "nothing in this template reads a switch, so none would quietly do nothing"
    );
}

#[test]
fn a_thinking_section_is_noticed_because_the_budget_depends_on_one() {
    assert!(Thinking::in_template("<think>a</think>").section);
    assert!(Thinking::in_template("<|channel|>analysis").section);
    assert!(Thinking::in_template("<seed:think>").section);
    assert!(
        !Thinking::in_template("{{ messages }}").section,
        "without one the engine builds no budget sampler at all"
    );
}

#[test]
fn a_set_written_with_brackets_and_double_quotes_reads_the_same() {
    let held = Thinking::in_template(
        "{%- if reasoning_effort not in [\"low\", \"high\"] %}{{ raise_exception('no') }}{% endif %}",
    );
    assert!(held.closed);
    assert_eq!(held.levels, vec!["low".to_owned(), "high".to_owned()]);
}

#[test]
fn an_unknown_word_in_a_set_is_kept_and_sorted_after_the_ones_we_know() {
    let held = Thinking::in_template(
        "{%- if reasoning_effort not in ('medium', 'ludicrous', 'low') %}{{ raise_exception('no') }}{% endif %}",
    );
    assert_eq!(
        held.levels,
        vec![
            "low".to_owned(),
            "medium".to_owned(),
            "ludicrous".to_owned()
        ],
        "a word MCF does not rank still belongs to the model that named it"
    );
}
