//! Tests for the stand-in prohibition.
//!
//! B65's enforcement is that a timing from a stand-in does not compile, and a
//! test cannot assert what does not compile. What is tested is everything
//! around it: that both engines answer behaviour questions, that a stand-in
//! result is marked, that the build travels, and that no conversion exists
//! between the two runs.

use super::{Behaviour, Engine, Run, StandIn, Vendored};
use crate::build_identity::BuildIdentity;
use crate::failure::Category;
use crate::measurement::{Conditions, Count, Floor, Measurement};

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

/// B31: a behaviour question does not depend on how fast the arithmetic was,
/// so both engines answer it. This is the whole of what a stand-in is for.
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

/// Only a vendored run can produce a timing. The compiler enforces it — there
/// is no `timing` method on `Run<StandIn>` — so what is recorded here is that
/// the one that exists works and carries what a comparison needs.
#[test]
fn a_vendored_run_produces_a_timing_that_carries_its_build() {
    let vendored: Run<Vendored> = Run::at_build("b4021");
    let measured = Measurement::of(Count(10), Count(30), [Count(20)], conditions());
    let timing = vendored.timing(measured);

    assert_eq!(timing.build(), "b4021");
    assert_eq!(timing.measured().n(), 3);
    assert!(timing.to_string().contains("vendored engine"), "{timing}");
    // The conditions travel, so a later comparison can tell whether two
    // timings are comparable at all (A6, A8).
    assert_eq!(timing.conditions().mcf(), BuildIdentity::current());
}

/// A5: a result taken under reduced capability is marked, and the mark names
/// what was lost. The reduced capability here is *the engine the artifact was
/// meant to run on*.
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
    // B21: enough to rebuild the situation from the record alone — which
    // implementation ran, at which build, and what it was allowed to report.
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

/// The two engines declare what they permit, so a surface can say which it is
/// looking at without branching on a type parameter.
#[test]
fn each_engine_declares_whether_it_may_be_timed() {
    // A `const` assertion, which is stronger: it fails the *build* rather than
    // a test run, so an engine that quietly gained permission to be timed does
    // not compile.
    const { assert!(Vendored::MAY_BE_TIMED) };
    const { assert!(!StandIn::MAY_BE_TIMED) };
    assert_eq!(Run::<Vendored>::engine_name(), "vendored");
    assert_eq!(Run::<StandIn>::engine_name(), "stand-in");
}

/// Intent v16 makes the engine build part of a configuration's identity, so a
/// run cannot be built without one.
#[test]
fn a_run_cannot_omit_its_build() {
    let run: Run<StandIn> = Run::at_build("mcf-0.1.0-m0");
    assert_eq!(run.build(), "mcf-0.1.0-m0");
    let behaviour: Behaviour<&str> = run.behaviour("something");
    assert_eq!(behaviour.build(), "mcf-0.1.0-m0");
}
