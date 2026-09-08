use std::path::Path;

use super::{Space, on};
use crate::attested::Attested;

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

#[test]
fn the_reading_agrees_with_the_platforms_own_tool() {
    let Attested::Known(space) = on(Path::new(".")) else {
        panic!("this filesystem could not be measured");
    };
    let Some(reported) = df_available() else {
        println!("cannot check against df on this machine; the reading stands unverified");
        return;
    };

    let apart = space.available.0.abs_diff(reported);
    let tolerated = space.total.0.checked_div(100).unwrap_or(0);
    assert!(
        apart <= tolerated.max(4096),
        "MCF says {} available and df says {reported}: {apart} apart, which is more than a \
         moving number explains",
        space.available.0
    );
}

#[test]
fn what_cannot_be_measured_is_unknown_and_not_empty() {
    assert_eq!(
        on(Path::new("/this/path/does/not/exist/at/all")),
        Attested::Unknown
    );
    assert_eq!(on(Path::new("with a \0 in it")), Attested::Unknown);
}

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

fn df_available() -> Option<u64> {
    let output = std::process::Command::new("df")
        .args(["-B1", "--output=avail", "."])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    text.lines().nth(1)?.trim().parse().ok()
}
