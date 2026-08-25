//! What the reader claims about a GGUF file, checked against files this test
//! wrote.
//!
//! The writer below is test machinery and deliberately dumb: it emits exactly
//! what the format specifies, byte for byte, so that a reader bug cannot hide
//! behind a writer that shares it. Every damaged case is that writer's output
//! with one thing changed, which is how a test says *this and only this is
//! wrong*.

use super::{DEFAULT_ALIGNMENT, Model, TensorKind, Value, parse};
use mcf_core::failure::Category;

/// Builds a GGUF file.
#[derive(Default)]
pub(crate) struct Writer {
    pub(crate) metadata: Vec<(String, u32, Vec<u8>)>,
    pub(crate) tensors: Vec<(String, Vec<u64>, u32, u64)>,
    pub(crate) version: u32,
    pub(crate) magic: [u8; 4],
}

impl Writer {
    pub(crate) fn new() -> Self {
        Self {
            version: 3,
            magic: *b"GGUF",
            ..Self::default()
        }
    }

    pub(crate) fn text(mut self, key: &str, value: &str) -> Self {
        let mut bytes = length(value.len()).to_vec();
        bytes.extend_from_slice(value.as_bytes());
        self.metadata.push((key.to_owned(), 8, bytes));
        self
    }

    pub(crate) fn integer(mut self, key: &str, value: u32) -> Self {
        self.metadata
            .push((key.to_owned(), 5, value.to_le_bytes().to_vec()));
        self
    }

    pub(crate) fn list_of_text(mut self, key: &str, values: &[&str]) -> Self {
        let mut bytes = 8_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&length(values.len()));
        for value in values {
            bytes.extend_from_slice(&length(value.len()));
            bytes.extend_from_slice(value.as_bytes());
        }
        self.metadata.push((key.to_owned(), 9, bytes));
        self
    }

    pub(crate) fn tensor(mut self, name: &str, dimensions: &[u64], kind: u32, offset: u64) -> Self {
        self.tensors
            .push((name.to_owned(), dimensions.to_vec(), kind, offset));
        self
    }

    pub(crate) fn write(&self) -> Vec<u8> {
        let mut out = self.magic.to_vec();
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&length(self.tensors.len()));
        out.extend_from_slice(&length(self.metadata.len()));
        for (key, kind, value) in &self.metadata {
            out.extend_from_slice(&length(key.len()));
            out.extend_from_slice(key.as_bytes());
            out.extend_from_slice(&kind.to_le_bytes());
            out.extend_from_slice(value);
        }
        for (name, dimensions, kind, offset) in &self.tensors {
            out.extend_from_slice(&length(name.len()));
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&rank(dimensions.len()));
            for dimension in dimensions {
                out.extend_from_slice(&dimension.to_le_bytes());
            }
            out.extend_from_slice(&kind.to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
        }
        out
    }
}

fn length(value: usize) -> [u8; 8] {
    u64::try_from(value).unwrap_or(0).to_le_bytes()
}

/// Whether a failure says something, in its detail or in the context it
/// carries.
///
/// A failure's `Display` is its classification and its detail; the *particulars*
/// — what was expected, where, what was found — travel as context, which is
/// where A2 asks them to be. A test that only read the rendering would be
/// asserting on the summary of the evidence rather than the evidence.
fn says(failure: &mcf_core::failure::Failure, needle: &str) -> bool {
    failure.detail().contains(needle)
        || failure
            .context()
            .iter()
            .any(|entry| entry.value.contains(needle) || entry.key.contains(needle))
}

fn rank(value: usize) -> [u8; 4] {
    u32::try_from(value).unwrap_or(0).to_le_bytes()
}

/// A small, well-formed file: metadata, a vocabulary and two tensors.
pub(crate) fn a_model() -> Vec<u8> {
    Writer::new()
        .text("general.architecture", "llama")
        .integer("llama.context_length", 4096)
        .list_of_text("tokenizer.ggml.tokens", &["<s>", "a", "b"])
        .tensor("token_embd.weight", &[8, 3], 0, 0)
        .tensor("blk.0.attn_q.weight", &[8, 8], 8, 96)
        .write()
}

#[test]
fn a_well_formed_file_reads_back_as_what_was_written() {
    let model: Model = parse(&a_model()).expect("the file is well formed");
    assert_eq!(model.version, 3);
    assert_eq!(model.architecture(), Some("llama"));
    assert_eq!(
        model
            .get("llama.context_length")
            .and_then(Value::as_integer),
        Some(4096)
    );
    assert_eq!(model.alignment, DEFAULT_ALIGNMENT);

    let tokens = model
        .get("tokenizer.ggml.tokens")
        .and_then(Value::as_list)
        .expect("the vocabulary is a list");
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens.first().and_then(Value::as_text), Some("<s>"));

    let embedding = model
        .tensor("token_embd.weight")
        .expect("it is in the file");
    assert_eq!(embedding.dimensions, vec![8, 3]);
    assert_eq!(embedding.kind, TensorKind::F32);
    assert_eq!(embedding.elements(), Some(24));
    assert_eq!(embedding.bytes(), Some(96));

    let quantized = model
        .tensor("blk.0.attn_q.weight")
        .expect("it is in the file");
    assert_eq!(quantized.kind, TensorKind::Q8_0);
    // 64 elements is two blocks of 32, each 34 bytes.
    assert_eq!(quantized.bytes(), Some(68));
}

/// The data begins at the next multiple of the alignment, which every tensor
/// offset is relative to. An off-by-one here reads every weight in the file
/// from the wrong place.
#[test]
fn the_data_offset_is_aligned() {
    let bytes = a_model();
    let model = parse(&bytes).expect("the file is well formed");
    let written = u64::try_from(bytes.len()).expect("a test file fits");
    assert_eq!(model.data_offset % model.alignment, 0);
    assert!(model.data_offset >= written);
    assert!(model.data_offset - written < model.alignment);
}

/// A file MCF does not read is *unsupported*; a GGUF that contradicts itself is
/// *malformed*. The distinction decides what an operator does next, so it is
/// asserted rather than left to the message.
#[test]
fn a_file_that_is_not_gguf_is_unsupported() {
    let failure = parse(b"ONNX\x00\x00\x00\x00").expect_err("not GGUF");
    assert_eq!(failure.category(), Category::ArtifactFormatUnsupported);
}

#[test]
fn a_version_this_reader_does_not_read_is_unsupported() {
    let mut bytes = a_model();
    let _replaced: Vec<u8> = bytes.splice(4..8, 9_u32.to_le_bytes()).collect();
    let failure = parse(&bytes).expect_err("version 9 is not read");
    assert_eq!(failure.category(), Category::ArtifactFormatUnsupported);
    assert!(says(&failure, "version 9"), "{failure}");
}

/// Version 2 differs from 3 in ways this reader does not depend on, and it says
/// so by reading one.
#[test]
fn version_two_reads() {
    let mut writer = Writer::new();
    writer.version = 2;
    let bytes = writer.text("general.architecture", "llama").write();
    assert_eq!(parse(&bytes).expect("version 2 reads").version, 2);
}

#[test]
fn a_truncated_file_says_what_it_wanted_and_where() {
    let whole = a_model();
    for cut in [12, 20, 40, whole.len() - 1] {
        let failure = parse(whole.get(..cut).unwrap_or(&whole)).expect_err("truncated");
        assert_eq!(
            failure.category(),
            Category::ArtifactFormatMalformed,
            "cut at {cut}: {failure}"
        );
        assert!(
            says(&failure, "offset") || says(&failure, "wanted"),
            "cut at {cut}: {failure} does not say where"
        );
    }
}

/// A count no file could satisfy is refused before anything is allocated
/// against it. This is the fuzzer's favourite input and the one that turns a
/// reader into an out-of-memory kill.
#[test]
fn an_impossible_count_is_refused_in_constant_space() {
    let mut bytes = a_model();
    let _replaced: Vec<u8> = bytes.splice(8..16, u64::MAX.to_le_bytes()).collect();
    let failure = parse(&bytes).expect_err("eighteen quintillion tensors");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    assert!(says(&failure, "could hold"), "{failure}");
}

/// The same for an array's length, which is a second place the same lie can be
/// told.
#[test]
fn an_impossible_array_length_is_refused() {
    let mut bytes = Writer::new()
        .list_of_text("tokenizer.ggml.tokens", &["a"])
        .write();
    let at = bytes
        .windows(4)
        .position(|window| window == 8_u32.to_le_bytes())
        .expect("the element type is in there");
    let _replaced: Vec<u8> = bytes
        .splice(at + 4..at + 12, u64::MAX.to_le_bytes())
        .collect();
    let failure = parse(&bytes).expect_err("an impossible array");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

/// One key, one meaning. A file that states a key twice is refused for the
/// reason `mcf_record::json` refuses a duplicate: two readers would disagree
/// about what it says, and a model's metadata decides how its tensors are used.
#[test]
fn a_duplicated_key_is_refused() {
    let bytes = Writer::new()
        .text("general.architecture", "llama")
        .text("general.architecture", "gemma")
        .write();
    let failure = parse(&bytes).expect_err("a key stated twice");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    assert!(says(&failure, "twice"), "{failure}");
}

/// A tensor type this reader does not know does not stop the file being read:
/// A4 keeps the rest, and the unknown one carries the number so an operator can
/// say which scheme is missing.
#[test]
fn an_unknown_tensor_type_is_carried_rather_than_refused() {
    let bytes = Writer::new()
        .text("general.architecture", "llama")
        .tensor("blk.0.ffn_up.weight", &[32], 27, 0)
        .write();
    let model = parse(&bytes).expect("the file is readable");
    let tensor = model.tensor("blk.0.ffn_up.weight").expect("it is there");
    assert_eq!(tensor.kind, TensorKind::Unknown(27));
    assert!(!tensor.kind.is_known());
    // And its size is unknown rather than guessed, so nothing reads its bytes.
    assert_eq!(tensor.bytes(), None);
    assert!(tensor.kind.to_string().contains("27"));
}

/// An alignment that is not a power of two would make every offset wrong, so it
/// is refused rather than rounded.
#[test]
fn an_alignment_that_is_not_a_power_of_two_is_refused() {
    let bytes = Writer::new().integer("general.alignment", 33).write();
    let failure = parse(&bytes).expect_err("33 is not a power of two");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

/// A stated alignment is honoured; the default is only a default.
#[test]
fn a_stated_alignment_is_used() {
    let bytes = Writer::new().integer("general.alignment", 64).write();
    let model = parse(&bytes).expect("64 is a power of two");
    assert_eq!(model.alignment, 64);
    assert_eq!(model.data_offset % 64, 0);
}

/// A string that is not UTF-8 is refused by name. The alternative — replacing
/// the bytes it cannot read — would put a tensor name in the directory that
/// does not match the one the model's code asks for.
#[test]
fn a_string_that_is_not_utf8_is_refused() {
    let mut bytes = Writer::new().text("general.architecture", "llama").write();
    let at = bytes
        .windows(5)
        .position(|window| window == b"llama")
        .expect("the value is in there");
    if let Some(slot) = bytes.get_mut(at) {
        *slot = 0xFF;
    }
    let failure = parse(&bytes).expect_err("not UTF-8");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    assert!(says(&failure, "UTF-8"), "{failure}");
}

/// A metadata type the format does not define is unsupported rather than
/// skipped: skipping it would need its width, which is the thing that is
/// unknown.
#[test]
fn an_unknown_metadata_type_is_unsupported() {
    let mut writer = Writer::new();
    writer.metadata.push(("odd".to_owned(), 42, vec![0; 4]));
    let failure = parse(&writer.write()).expect_err("type 42 is not defined");
    assert_eq!(failure.category(), Category::ArtifactFormatUnsupported);
}

/// The rank the format permits is four. More is refused, because everything
/// after it is read at an offset the rank decides.
#[test]
fn a_rank_beyond_the_format_is_refused() {
    let bytes = Writer::new()
        .tensor("blk.0.weird", &[1, 2, 3, 4, 5], 0, 0)
        .write();
    let failure = parse(&bytes).expect_err("rank 5");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

/// An empty file is refused without a panic, which is the only thing a reader
/// owes an input of no bytes.
#[test]
fn an_empty_file_is_refused_and_not_a_crash() {
    let failure = parse(&[]).expect_err("nothing is not a model");
    assert!(
        matches!(
            failure.category(),
            Category::ArtifactFormatMalformed | Category::ArtifactFormatUnsupported
        ),
        "{failure}"
    );
}

/// The directory has to agree with itself before anything reads bytes at the
/// offsets in it.
///
/// The fuzz tier found this on its first campaign against this reader: a
/// damaged file declared a tensor at an enormous offset whose size overflowed
/// when added to it, and the reader handed the directory back. Nothing
/// downstream could have used it, and the dequantizer would have been the one
/// to find out.
#[test]
fn a_tensor_that_ends_past_addressable_space_is_refused() {
    let bytes = Writer::new()
        .tensor("blk.0.attn_q.weight", &[64], 8, u64::MAX - 8)
        .write();
    let failure = parse(&bytes).expect_err("the tensor ends past the end of everything");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    assert!(says(&failure, "addressable"), "{failure}");
}

/// A shape whose product does not fit is refused for the same reason, one step
/// earlier.
#[test]
fn a_shape_that_does_not_multiply_out_is_refused() {
    let bytes = Writer::new().tensor("huge", &[u64::MAX, 2], 0, 0).write();
    let failure = parse(&bytes).expect_err("the shape overflows");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

/// What the directory accounts for is the end of the furthest tensor, and it is
/// unknown rather than a lower bound when a tensor's size cannot be computed.
#[test]
fn the_data_length_is_the_end_of_the_furthest_tensor() {
    let bytes = Writer::new()
        .tensor("first", &[8, 3], 0, 0)
        .tensor("second", &[8, 8], 8, 96)
        .write();
    let model = parse(&bytes).expect("well formed");
    // The second tensor is two Q8_0 blocks — 68 bytes — starting at 96.
    assert_eq!(model.data_bytes_required(), Some(164));

    let unknown = Writer::new().tensor("odd", &[32], 27, 0).write();
    let model = parse(&unknown).expect("an unknown type is still readable");
    assert_eq!(model.data_bytes_required(), None);
}

/// A file whose directory names more bytes than the file holds is a truncated
/// download, and `read` is where that is caught — `parse` cannot, because a
/// caller may legitimately hold only the head of a file (B-213).
#[test]
fn a_file_shorter_than_its_own_directory_is_refused_by_read() {
    let scratch = std::env::temp_dir().join(format!("mcf-gguf-short-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let path = scratch.join("model.gguf");

    let bytes = Writer::new()
        .tensor("token_embd.weight", &[8, 3], 0, 0)
        .write();
    // The directory says 96 bytes of tensor data follow, and nothing does.
    std::fs::write(&path, &bytes).expect("the file is writable");
    let failure = super::read(&path).expect_err("the file is shorter than it says");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    assert!(says(&failure, "than the file holds"), "{failure}");

    // The same header with the data present reads.
    let mut whole = bytes.clone();
    whole.resize(whole.len() + 200, 0);
    std::fs::write(&path, &whole).expect("the file is writable");
    let model = super::read(&path).expect("the data is there now");
    assert_eq!(model.tensors.len(), 1);

    // And parse, which may hold only the head, still reads the directory.
    assert!(parse(&bytes).is_ok());

    let _removed = std::fs::remove_dir_all(&scratch);
}
