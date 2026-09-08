use super::{Reference, parse};
use mcf_core::failure::Category;

fn read(written: &str) -> Reference {
    parse(written).unwrap_or_else(|failure| panic!("{written:?} should read: {failure}"))
}

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

#[test]
fn a_revision_and_a_file_together() {
    let both = read("owner/name@abc123:models/weights.gguf");
    assert_eq!(both.revision.as_deref(), Some("abc123"));
    assert_eq!(both.file.as_deref(), Some("models/weights.gguf"));
    assert_eq!(both.to_string(), "owner/name@abc123:models/weights.gguf");
    assert_eq!(both.repository(), "owner/name");
}

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

#[test]
fn whitespace_around_a_reference_is_ignored() {
    assert_eq!(read("  owner/name\t").repository(), "owner/name");
}

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

#[test]
fn an_owner_with_a_dot_in_it_is_refused() {
    assert!(parse("hf.co/name.co/repo").is_err());
    assert!(parse("owner.co/name").is_err());
    assert_eq!(read("meta/Llama-3.1-8B").name, "Llama-3.1-8B");
}

#[test]
fn a_name_that_cannot_be_written_back_is_refused() {
    assert!(parse("https://huggingface.co/o/n/resolve/main/we:ird.gguf").is_err());
    assert!(parse("https://huggingface.co/o/n/resolve/ma:in/f.gguf").is_err());
    assert!(parse("https://huggingface.co/o/n/resolve/main/we@ird.gguf").is_err());
}

#[test]
fn control_characters_are_refused() {
    assert!(parse("owner/na\u{7}me").is_err());
    assert!(parse("owner/name@re\u{0}v").is_err());
    assert!(parse("owner/name:fi\u{1b}le.gguf").is_err());
}

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

#[test]
fn the_spellings_agree() {
    assert_eq!(
        read("https://huggingface.co/owner/name/resolve/main/f.gguf"),
        read("owner/name@main:f.gguf")
    );
    assert_eq!(read("hf.co/owner/name"), read("owner/name"));
}
