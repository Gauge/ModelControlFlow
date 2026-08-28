//! Energy carries its provenance and its rate; a timing knows what watched it
//! (B-188, B-163, B-164, B39, B30, B31, D11, A20, A7, §3.4, §6.25).
//!
//! **B39's violation, in one sentence:** a platform with no power interface
//! yields a number derived from processor utilization. Multiplying a
//! percentage by a nameplate wattage produces a figure with a unit and nothing
//! behind it, which is the most convincing kind of wrong — it has the right
//! shape, the right magnitude, and no measurement in it at all.
//!
//! **And B31's:** a timing taken while a profiler was attached is a timing of
//! the profiler as much as of the model. The condition floor has recorded an
//! instrumentation profile as free text since it was written, which is enough
//! to note it and not enough to refuse on it.
//!
//! Both are guarded by shape. What this file checks is what a compiler cannot:
//! that no escape hatch is added, and that the sentences a reader meets say
//! the thing the type is enforcing.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

/// The code of a module, without the prose that describes it.
///
/// F87's pattern: a rule and the sentence explaining the rule cannot be told
/// apart by substring search, and every one of these modules names its
/// forbidden thing in order to say it is absent.
fn code_of(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// A number leaves an energy figure only where one was read.
#[test]
fn no_energy_number_escapes_without_a_reading_behind_it() {
    let held = code_of("crates/mcf-core/src/energy.rs");
    assert!(
        held.contains("pub const fn measured_millijoules(&self) -> Option<u64>"),
        "the only way out must be an `Option`, so a caller meets the modelled and unknown \
         cases where they were about to flatten them (A20, B39)"
    );
    // `utilization` appears once, inside the sentence that refuses it. More
    // than once would mean it had become an implementation and not only a
    // warning.
    assert_eq!(
        held.matches("utilization").count(),
        1,
        "B39's violation is deriving a figure from processor utilization; the word belongs \
         only in the sentence that refuses it"
    );
    for hatch in [
        "unwrap_or",
        "millijoules_or",
        "impl Default for Energy",
        "fn joules(&self) -> u64",
    ] {
        assert!(
            !held.contains(hatch),
            "`{hatch}` would let a figure with no measurement behind it leave as a number, \
             which is B39's violation exactly"
        );
    }
}

/// The rate is in the type, not in a comment.
#[test]
fn the_sampling_rate_cannot_be_dropped() {
    let held = code_of("crates/mcf-core/src/energy.rs");
    assert!(
        held.contains("rate: PerSecond"),
        "energy read at one hertz across a two-second run saw two samples, and a surface that \
         only wanted the joules must not be able to leave the rate behind (§3.4, B-188)"
    );
    assert!(
        held.contains("counter: String"),
        "and which counter, since a package-level figure is not a device-level one"
    );
}

/// A timing cannot be built from a run it cannot account for.
#[test]
fn a_deep_instrumentation_run_produces_no_timing() {
    let held = code_of("crates/mcf-core/src/instrumentation.rs");
    assert!(
        held.contains("pub fn new(value: T, under: Profile) -> Result<Self, NotATiming>"),
        "B-164: the type system refuses the construction, which means the constructor is \
         fallible rather than a doc comment asking nicely"
    );
    assert!(
        held.contains("Profile::Deep { watcher } => Err("),
        "and the refusal is on the deep profile specifically"
    );
    assert!(
        !held.contains("impl Default for Timed"),
        "a default would be a timing with no profile, which is what B-163 forbids"
    );
}

/// A profile is a required argument, not an optional one.
#[test]
fn a_result_without_its_profile_cannot_be_constructed() {
    let held = code_of("crates/mcf-core/src/instrumentation.rs");
    assert!(
        held.contains("under: Profile,") && !held.contains("under: Option<Profile>"),
        "B-163: a result without its profile cannot be constructed, so the profile is a field \
         and not a possibility"
    );
}

/// Nothing declares an overhead acceptable on somebody else's behalf.
#[test]
fn the_residual_is_characterized_rather_than_thresholded() {
    let held = code_of("crates/mcf-core/src/instrumentation.rs");
    assert!(
        held.contains("residual: PartsPerMillion"),
        "B31 asks that the overhead be characterized: a `Light` profile carries what it cost"
    );
    for threshold in ["negligible", "acceptable", "small enough", "if residual <"] {
        assert!(
            !held.contains(threshold),
            "`{threshold}` would be MCF deciding how much perturbation is acceptable for \
             somebody else's measurement — the same kind of figure DEC-007 exists to derive"
        );
    }
}
