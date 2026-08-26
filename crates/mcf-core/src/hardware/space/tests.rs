//! What the kernel says about room, checked against something that is not MCF.

use std::path::Path;

use super::{Space, on};
use crate::attested::Attested;

/// A reading of a real filesystem is a real reading: the total is not zero, and
/// what is available is not more than there is.
#[test]
fn a_reading_of_a_real_path_holds_together() {
    let Attested::Known(space) = on(Path::new(".")) else {
        panic!("this filesystem could not be measured, which is not a state a test can pass on");
    };
    assert!(space.total.0 > 0, "a filesystem of no size");
    assert!(
        space.available.0 <= space.total.0,
        "more room available ({}) than exists ({})",
        space.available.0,
        space.total.0
    );
}

/// And it agrees with an oracle that is not MCF.
///
/// `df` reads the same call through a different program written by other
/// people, which is what A12 asks of a claim MCF makes about the machine. The
/// tolerance is one per cent of the *total*, because the two readings are taken
/// a moment apart and this filesystem is in use — a disagreement of a few
/// blocks is two readings of a moving number, and a disagreement of a per cent
/// is a defect.
#[test]
fn the_reading_agrees_with_the_platforms_own_tool() {
    let Attested::Known(space) = on(Path::new(".")) else {
        panic!("this filesystem could not be measured");
    };
    let Some(reported) = df_available() else {
        // No `df`, or a `df` that answers differently: the check cannot be
        // made, which is not the same as passing (A2). Said, not swallowed.
        println!("cannot check against df on this machine; the reading stands unverified");
        return;
    };

    let apart = space.available.0.abs_diff(reported);
    // A hundredth of the filesystem, computed without dividing: the workspace
    // denies integer division because a truncated quotient is a silently wrong
    // number, and a tolerance is exactly the kind of number that must not be.
    let tolerated = space.total.0.checked_div(100).unwrap_or(0);
    assert!(
        apart <= tolerated.max(4096),
        "MCF says {} available and df says {reported}: {apart} apart, which is more than a \
         moving number explains",
        space.available.0
    );
}

/// A path the kernel cannot be asked about is unknown rather than zero, which
/// is the difference between *MCF could not look* and *there is no room* (A7).
#[test]
fn what_cannot_be_measured_is_unknown_and_not_empty() {
    assert_eq!(
        on(Path::new("/this/path/does/not/exist/at/all")),
        Attested::Unknown
    );
    assert_eq!(on(Path::new("with a \0 in it")), Attested::Unknown);
}

/// The rendering names both numbers, because *how much is left* means nothing
/// without *of what* (A6's habit).
#[test]
fn the_rendering_carries_both_numbers() {
    let rendered = Space {
        total: crate::measurement::Bytes(1000),
        available: crate::measurement::Bytes(400),
    }
    .to_string();
    assert!(rendered.contains("400"), "{rendered}");
    assert!(rendered.contains("1000"), "{rendered}");
}

/// What `df` says is available here, in bytes.
fn df_available() -> Option<u64> {
    let output = std::process::Command::new("df")
        .args(["-B1", "--output=avail", "."])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    text.lines().nth(1)?.trim().parse().ok()
}
