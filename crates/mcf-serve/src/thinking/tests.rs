use super::{CONVENTIONAL, Thinking};

#[test]
fn a_template_that_reads_no_level_offers_none() {
    let held = Thinking::in_template("{{ messages }}");
    assert!(!held.reads_a_level());
    assert!(held.levels.is_empty());
    assert!(
        held.said().contains("says nothing about thinking"),
        "and says so rather than offering a setting that does nothing: {}",
        held.said()
    );
}

/// Naming no level is not the same as not thinking, and what MCF says has to tell the two
/// apart: most families name no level and can still be told not to think.
#[test]
fn a_template_with_no_level_but_a_switch_says_that_thinking_can_still_be_turned_off() {
    let held = Thinking::in_template("{%- if enable_thinking %}<think>{% endif %}");
    assert!(held.levels.is_empty());
    assert!(held.said().contains("can be turned off"), "{}", held.said());

    let cut = Thinking::in_template("{{- '<think>' }} and no switch");
    assert!(
        cut.said().contains("cut short"),
        "a section with no switch is stopped by the engine instead: {}",
        cut.said()
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

/// The shapes the families on a real machine actually use.
///
/// Read off the templates of the models held here, one case a family, because MCF was
/// reading four of the seven as unable to think at all: it only recognised a template that
/// named an effort variable, and most name none. What a template has to say about thinking
/// is three separate things — whether it marks a section, whether it reads a switch, and
/// whether it names levels — and a family may have any combination of them.
mod the_shapes_families_use {
    use super::Thinking;

    /// Qwen3, Nemotron-3, Laguna: a switch and a section, and no named level.
    #[test]
    fn a_switch_and_a_section_without_levels_is_still_a_model_that_can_stop_thinking() {
        let held = Thinking::in_template(
            "{%- set enable_thinking = enable_thinking if enable_thinking is defined else True %}\
             {%- if enable_thinking %}{{- '<think>\\n' }}{%- else %}{{- '<think></think>' }}\
             {%- endif %}",
        );
        assert!(held.section, "the template opens a thinking section");
        assert!(
            held.switch,
            "and reads a switch that decides whether it does"
        );
        assert_eq!(held.variable, None, "but it names no effort variable");
    }

    /// Gemma: a switch, and a tag no list of MCF's had ever named.
    #[test]
    fn a_section_is_found_by_what_the_tag_says_rather_than_by_a_list_of_tags() {
        let held = Thinking::in_template(
            "{%- set enable_thinking = enable_thinking | default(false) -%}\
             {%- if enable_thinking -%}{{- '<|think|>\\n' -}}{%- endif -%}",
        );
        assert!(
            held.section,
            "a tag that says it is for thinking opens a thinking section, whoever wrote it"
        );
        assert!(held.switch);
    }

    /// gpt-oss: levels and a section, and no switch — the engine cuts it short instead.
    #[test]
    fn levels_without_a_switch_are_read_as_levels_without_a_switch() {
        let held = Thinking::in_template(
            "{%- if reasoning_effort is not defined %}{%- set reasoning_effort = \"medium\" %}\
             {%- endif %}{{- \"Reasoning: \" + reasoning_effort }}\
             {{- \"<|channel|>analysis\" }}",
        );
        assert!(held.section);
        assert!(!held.switch);
        assert_eq!(held.variable.as_deref(), Some("reasoning_effort"));
    }

    /// A coder model that does not think: nothing to report, and nothing invented.
    #[test]
    fn a_template_that_says_nothing_about_thinking_is_not_given_a_thinking_section() {
        let held = Thinking::in_template(
            "{%- for message in messages %}{{- '<|im_start|>' + message.role }}\
             {{- '<tool_call>' }}{%- endfor %}",
        );
        assert!(
            !held.section,
            "tags that are not about thinking do not open a thinking section"
        );
        assert!(!held.switch);
        assert!(held.levels.is_empty());
    }

    /// Prose about thinking is not a thinking section: a macro named for stripping thought
    /// out of a message says nothing about whether this model produces any.
    #[test]
    fn a_name_that_is_not_a_tag_is_not_a_tag() {
        let held =
            Thinking::in_template("{%- macro strip_thinking(text) -%}{{- text -}}{%- endmacro -%}");
        assert!(!held.section);
    }
}
