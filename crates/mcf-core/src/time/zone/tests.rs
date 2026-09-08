use super::{Zone, offset_at};
use crate::attested::Attested;
use crate::time::{Timestamp, UtcOffset};

#[test]
fn the_offset_agrees_with_what_the_system_says() {
    let Ok(output) = std::process::Command::new("date").arg("+%z").output() else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let Ok(reported) = String::from_utf8(output.stdout) else {
        return;
    };
    let reported = reported.trim();
    if reported.len() != 5 {
        return;
    }

    let Attested::Known(read) = offset_at(Timestamp::now()) else {
        return;
    };

    let rendered = read.to_string().replace(':', "");
    assert_eq!(
        rendered, reported,
        "MCF reads {rendered} and the system reports {reported}"
    );
}

#[test]
fn a_past_moment_uses_the_offset_that_was_in_force_then() {
    const SIX_MONTHS: i128 = 182 * 24 * 3_600 * 1_000_000_000;

    let Ok(bytes) = std::fs::read("/etc/localtime") else {
        return;
    };
    let Some(zone) = Zone::parse(&bytes) else {
        return;
    };

    let now = Timestamp::now();
    let earlier = Timestamp::from_utc_nanos(now.utc_nanos() - SIX_MONTHS, Attested::Unknown);
    let (a, b) = (zone.offset_at(now), zone.offset_at(earlier));
    assert!(a.is_some(), "the current moment has no offset");
    assert!(b.is_some(), "a moment six months ago has no offset");

    let ancient = Timestamp::from_utc_nanos(-2_208_988_800_000_000_000, Attested::Unknown);
    assert!(
        zone.offset_at(ancient).is_some(),
        "a moment in 1900 has no offset, so the initial type was not read"
    );
}

#[test]
fn a_moment_beyond_the_file_is_unknown_rather_than_extrapolated() {
    let Ok(bytes) = std::fs::read("/etc/localtime") else {
        return;
    };
    let Some(zone) = Zone::parse(&bytes) else {
        return;
    };
    let distant = Timestamp::from_utc_nanos(253_370_764_800_000_000_000, Attested::Unknown);
    assert_eq!(zone.offset_at(distant), None);
    assert_eq!(Zone::safe_horizon(), 0);
}

#[test]
fn what_is_not_a_zone_file_is_refused() {
    assert_eq!(Zone::parse(b""), None);
    assert_eq!(Zone::parse(b"not a zone file at all"), None);
    let mut truncated = b"TZif2".to_vec();
    truncated.extend(std::iter::repeat_n(0_u8, 30));
    assert_eq!(Zone::parse(&truncated), None);
}

#[test]
fn the_offset_read_is_a_plausible_offset() {
    let Attested::Known(read) = offset_at(Timestamp::now()) else {
        return;
    };
    assert!(UtcOffset::from_seconds_east(read.seconds_east()).is_some());
    assert!(read.seconds_east().abs() <= 14 * 3_600);
    assert_eq!(read.seconds_east() % 60, 0, "{read} is not whole minutes");
}

fn a_zone_with_two_transitions() -> Vec<u8> {
    const TRANSITIONS: [i32; 2] = [1_000, 2_000];
    const OFFSETS: [i32; 3] = [3_600, 7_200, 10_800];

    let mut bytes = b"TZif1".to_vec();
    bytes.extend(std::iter::repeat_n(0_u8, 15));
    for count in [0_u32, 0, 0, 2, 3, 4] {
        bytes.extend(count.to_be_bytes());
    }
    for when in TRANSITIONS {
        bytes.extend(when.to_be_bytes());
    }
    bytes.extend([1_u8, 2]);
    for east in OFFSETS {
        bytes.extend(east.to_be_bytes());
        bytes.extend([0_u8, 0]);
    }
    bytes.extend(b"UTC\0");
    bytes
}

#[test]
fn the_offset_between_two_transitions_is_the_one_the_earlier_switched_to() {
    const NANOS_PER_SECOND: i128 = 1_000_000_000;
    let at =
        |seconds: i128| Timestamp::from_utc_nanos(seconds * NANOS_PER_SECOND, Attested::Unknown);
    let east = |seconds| UtcOffset::from_seconds_east(seconds);

    let zone = Zone::parse(&a_zone_with_two_transitions()).expect("the built file is read");

    assert_eq!(zone.offset_at(at(0)), east(3_600));
    assert_eq!(zone.offset_at(at(999)), east(3_600));
    assert_eq!(zone.offset_at(at(1_000)), east(7_200));
    assert_eq!(zone.offset_at(at(1_500)), east(7_200));
    assert_eq!(zone.offset_at(at(1_999)), east(7_200));
    assert_eq!(zone.offset_at(at(2_000)), east(10_800));
    assert_eq!(zone.offset_at(at(2_001)), None);
}
