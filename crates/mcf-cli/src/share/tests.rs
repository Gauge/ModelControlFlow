use super::*;

/// A row that leaves never names a path.
///
/// The record names an arm by the file it ran, which is right for a local
/// record. What a contribution held before this was checked was
/// `/home/somebody/.local/share/mcf/models/…` — the operator's user name and
/// the shape of their disk, in a file whose whole purpose is to be published.
#[test]
fn an_arm_that_leaves_is_a_name_and_not_a_path() {
    let cases = [
        (
            "/home/somebody/.local/share/mcf/models/owner/repo/example-8b-q4.gguf",
            "example-8b-q4.gguf",
        ),
        ("C:\\Users\\somebody\\models\\example.gguf", "example.gguf"),
        ("./relative/example.gguf", "example.gguf"),
        // Already a name: left alone.
        ("example-8b-q4.gguf", "example-8b-q4.gguf"),
    ];
    for (held, wanted) in cases {
        assert_eq!(publishable(held).as_str(), wanted, "from {held}");
    }
}

/// Whatever it is given, what comes out has no separator in it.
#[test]
fn nothing_that_leaves_carries_a_separator() {
    for held in [
        "/",
        "//",
        "a/b/c",
        "/home/somebody/",
        "\\\\server\\share\\model.gguf",
        "",
    ] {
        let travelled = publishable(held);
        assert!(
            !travelled.as_str().contains('/') && !travelled.as_str().contains('\\'),
            "{held:?} became {:?}, which still names a place",
            travelled.as_str()
        );
    }
}

/// And it never carries a home directory, whoever's it is.
#[test]
fn nothing_that_leaves_carries_a_user_name() {
    let travelled = publishable("/home/gauge/.local/share/mcf/models/x/example.gguf");
    assert!(
        !travelled.as_str().contains("home"),
        "{:?} still names somebody",
        travelled.as_str()
    );
    assert!(!travelled.as_str().contains("gauge"));
}
