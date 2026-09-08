use super::{Behaviour, Engine, Run, StandIn, Vendored};
use crate::build_identity::BuildIdentity;
use crate::failure::Category;
use crate::measurement::{Conditions, Count, Floor, Measurement};

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

#[test]
fn both_engines_answer_a_behaviour_question() {
    let vendored: Run<Vendored> = Run::at_build("b4021");
    let stand_in: Run<StandIn> = Run::at_build("mcf-0.1.0-m0");

    let one = vendored.behaviour("the tool call parsed");
    let other = stand_in.behaviour("the tool call parsed");

    assert_eq!(one.observed(), other.observed());
    assert_eq!(one.engine(), "vendored");
    assert_eq!(other.engine(), "stand-in");
}

#[test]
fn a_vendored_run_produces_a_timing_that_carries_its_build() {
    let vendored: Run<Vendored> = Run::at_build("b4021");
    let measured = Measurement::of(Count(10), Count(30), [Count(20)], conditions());
    let timing = vendored.timing(measured);

    assert_eq!(timing.build(), "b4021");
    assert_eq!(timing.measured().n(), 3);
    assert!(timing.to_string().contains("vendored engine"), "{timing}");
    assert_eq!(timing.conditions().mcf(), BuildIdentity::current());
}

#[test]
fn a_stand_in_result_is_marked_with_what_was_lost() {
    let stand_in: Run<StandIn> = Run::at_build("mcf-0.1.0-m0");
    let marked = stand_in.mark(stand_in.behaviour("the loop terminated"));

    let cause = marked
        .degradation()
        .causes()
        .first()
        .expect("the mark names a cause");
    assert_eq!(cause.category(), Category::EngineUnavailable);
    assert_eq!(cause.context_value("engine"), Some("stand-in"));
    assert_eq!(cause.context_value("engine_build"), Some("mcf-0.1.0-m0"));
    assert!(
        cause
            .context_value("results_permitted")
            .is_some_and(|what| what.contains("behaviour-class")),
        "the mark does not say what the stand-in was allowed to report"
    );
    assert!(marked.to_string().contains("DEGRADED"), "{marked}");
    assert!(
        marked.to_string().contains("can never report a speed"),
        "{marked}"
    );
}

#[test]
fn each_engine_declares_whether_it_may_be_timed() {
    const { assert!(Vendored::MAY_BE_TIMED) };
    const { assert!(!StandIn::MAY_BE_TIMED) };
    assert_eq!(Run::<Vendored>::engine_name(), "vendored");
    assert_eq!(Run::<StandIn>::engine_name(), "stand-in");
}

#[test]
fn a_run_cannot_omit_its_build() {
    let run: Run<StandIn> = Run::at_build("mcf-0.1.0-m0");
    assert_eq!(run.build(), "mcf-0.1.0-m0");
    let behaviour: Behaviour<&str> = run.behaviour("something");
    assert_eq!(behaviour.build(), "mcf-0.1.0-m0");
}
