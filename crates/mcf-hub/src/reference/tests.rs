//! Every way of naming a model, and every way of getting it wrong.
//!
//! B7's commitment is *no unhandled outcomes*, so the interesting half of this
//! file is the refusals: each one is a string somebody could plausibly type or
//! paste, and each reaches a named answer rather than a panic, a hang or a
//! silent reinterpretation.

use super::{Reference, parse};
use mcf_core::failure::Category;

fn read(written: &str) -> Reference {
    parse(written).unwrap_or_else(|failure| panic!("{written:?} should read: {failure}"))
}

/// The three shapes a person types.
#[test]
fn the_shapes_people_type() {
    let plain = read("meta-llama/Llama-3.1-8B");
    assert_eq!(plain.owner, "meta-llama");
    assert_eq!(plain.name, "Llama-3.1-8B");
    assert_eq!(
        plain.revision, None,
        "a default revision is the hub's to name"
    );
    assert_eq!(plain.file, None);

    let pinned = read("meta-llama/Llama-3.1-8B@v1.0");
    assert_eq!(pinned.revision.as_deref(), Some("v1.0"));

    let quantization = read("TheBloke/Llama-2-7B-GGUF:llama-2-7b.Q4_K_M.gguf");
    assert_eq!(quantization.name, "Llama-2-7B-GGUF");
    assert_eq!(
        quantization.file.as_deref(),
        Some("llama-2-7b.Q4_K_M.gguf"),
        "naming a file is how a quantization is chosen"
    );
}

/// A revision and a file together, which is the fully specified form.
#[test]
fn a_revision_and_a_file_together() {
    let both = read("owner/name@abc123:models/weights.gguf");
    assert_eq!(both.revision.as_deref(), Some("abc123"));
    assert_eq!(both.file.as_deref(), Some("models/weights.gguf"));
    assert_eq!(both.to_string(), "owner/name@abc123:models/weights.gguf");
    assert_eq!(both.repository(), "owner/name");
}

/// The shapes people paste.
#[test]
fn the_shapes_people_paste() {
    for written in [
        "https://huggingface.co/owner/name",
        "http://huggingface.co/owner/name",
        "huggingface.co/owner/name",
        "hf.co/owner/name",
        "https://hf.co/owner/name/",
    ] {
        let reference = read(written);
        assert_eq!(reference.repository(), "owner/name", "{written}");
        assert_eq!(reference.revision, None, "{written}");
    }

    let blob = read("https://huggingface.co/owner/name/blob/main/model.gguf");
    assert_eq!(blob.revision.as_deref(), Some("main"));
    assert_eq!(blob.file.as_deref(), Some("model.gguf"));

    let resolve = read("https://huggingface.co/owner/name/resolve/v2/sub/model.gguf");
    assert_eq!(resolve.revision.as_deref(), Some("v2"));
    assert_eq!(
        resolve.file.as_deref(),
        Some("sub/model.gguf"),
        "a file inside a directory keeps its path"
    );
}

/// Surrounding whitespace is a paste, not a different reference.
#[test]
fn whitespace_around_a_reference_is_ignored() {
    assert_eq!(read("  owner/name\t").repository(), "owner/name");
}

/// Everything that is not a reference reaches a named refusal. None of these
/// is exotic: each is something a person types or a script produces.
#[test]
fn everything_else_is_refused_by_name() {
    for written in [
        "",
        "   ",
        "owner",
        "/name",
        "owner/",
        "owner/name/extra",
        "owner//name",
        "owner/name@",
        "owner/name:",
        "owner/name@one@two",
        "owner/name:one:two",
        "../etc/passwd",
        "owner/../escape",
        "owner/name:../escape.gguf",
        "owner/name:/absolute.gguf",
        "owner/name:sub/../escape",
        "owner/name:back\\slash",
        "file:///etc/passwd",
        "ftp://huggingface.co/owner/name",
        "https://example.com/owner/name",
        "example.com/owner/name",
        "https://huggingface.co/owner",
        "https://huggingface.co/owner/name/tree/main",
        "https://huggingface.co/owner/name/resolve/main",
        "owner/name\nowner/other",
    ] {
        let failure = parse(written)
            .err()
            .unwrap_or_else(|| panic!("{written:?} was accepted as a reference"));
        assert_eq!(
            failure.category(),
            Category::HubRefNotFound,
            "{written:?} produced {failure}"
        );
        assert!(
            !failure.detail().is_empty(),
            "{written:?} was refused without saying why"
        );
    }
}

/// An owner cannot contain a dot, which is what tells an owner from a host.
///
/// The fuzz tier found the consequence of leaving that implicit:
/// `hf.co/name.co/repo` was accepted with `name.co` as the owner, and the
/// rendered reference then read back as a reference to another host.
#[test]
fn an_owner_with_a_dot_in_it_is_refused() {
    assert!(parse("hf.co/name.co/repo").is_err());
    assert!(parse("owner.co/name").is_err());
    // A repository name may contain dots — most published models do.
    assert_eq!(read("meta/Llama-3.1-8B").name, "Llama-3.1-8B");
}

/// A file whose name contains the written form's separators is refused, because
/// a reference MCF cannot write back is one it cannot record.
///
/// The fuzz tier found this: a URL can carry such a name, and the rendered
/// reference then would not re-read. C5's habit is that a written identifier is
/// one somebody can type again.
#[test]
fn a_name_that_cannot_be_written_back_is_refused() {
    assert!(parse("https://huggingface.co/o/n/resolve/main/we:ird.gguf").is_err());
    assert!(parse("https://huggingface.co/o/n/resolve/ma:in/f.gguf").is_err());
    assert!(parse("https://huggingface.co/o/n/resolve/main/we@ird.gguf").is_err());
}

/// A control character in a name is refused rather than carried into a path or
/// a request. §3.7: the reference is untrusted input, and this is the first
/// place it is stopped.
#[test]
fn control_characters_are_refused() {
    assert!(parse("owner/na\u{7}me").is_err());
    assert!(parse("owner/name@re\u{0}v").is_err());
    assert!(parse("owner/name:fi\u{1b}le.gguf").is_err());
}

/// The refusal says what it saw, so an operator can see which part of what they
/// typed was the problem (A2).
#[test]
fn a_refusal_carries_what_it_saw() {
    let failure = parse("https://example.com/owner/name").expect_err("not the hub");
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("example.com")),
        "the refusal does not name the host it refused"
    );
}

/// Reading a reference and rendering it produces the same reference again,
/// which is what makes the rendered form usable in a record (C5's habit: a
/// written identifier is one somebody can type back).
#[test]
fn a_reference_round_trips_through_its_rendering() {
    for written in [
        "owner/name",
        "owner/name@main",
        "owner/name:file.gguf",
        "owner/name@v1:dir/file.gguf",
    ] {
        let once = read(written);
        let twice = read(&once.to_string());
        assert_eq!(once, twice, "{written}");
        assert_eq!(once.to_string(), written);
    }
}

/// A URL and the typed form of the same thing are the same reference, which is
/// what "without special-casing" means in practice (§6.3).
#[test]
fn the_spellings_agree() {
    assert_eq!(
        read("https://huggingface.co/owner/name/resolve/main/f.gguf"),
        read("owner/name@main:f.gguf")
    );
    assert_eq!(read("hf.co/owner/name"), read("owner/name"));
}
