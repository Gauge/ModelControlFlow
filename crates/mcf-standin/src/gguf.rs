//! Reading a GGUF file: what is in it, and what MCF refuses to guess about.
//!
//! GGUF is the format §XII's reference model is published in, and the one the
//! candidate engines in [vendored.md] read. A file is: a magic number and a
//! version, a count of tensors and a count of metadata entries, the metadata as
//! typed key/value pairs, a directory of tensors naming each one's shape, type
//! and offset, and then the tensor data at an alignment the metadata declares.
//!
//! **Every byte here is untrusted** (§3.7). A model file arrives from a
//! repository MCF does not control, and the fuzz tier examines this reader for
//! that reason. So the rules are the ones A2 and A7 give: what cannot be read
//! is a classified failure naming what was expected and where, never a
//! plausible reconstruction, and a length this reader cannot honour is refused
//! before anything is allocated against it.
//!
//! **What it declines, stated rather than discovered.** It reads little-endian
//! files, which is every GGUF published; a big-endian one is refused by name
//! rather than byte-swapped on a guess. It reads versions 2 and 3, which differ
//! in the width of their counts, and refuses a version it does not know rather
//! than assuming the layout continued (§7.30's habit, applied to somebody
//! else's schema).
//!
//! **It does not read the tensor data.** The directory says where each tensor
//! is and how big; loading is the dequantizer's job and belongs with the
//! scheme that interprets the bytes. A reader that eagerly loaded would make
//! *inspecting* a 40 GiB file cost 40 GiB, and B-213's pre-acquisition fitment
//! is exactly the case that must not.
//!
//! [vendored.md]: ../../../doc/vendored.md

use std::collections::BTreeMap;
use std::path::Path;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-standin::gguf");

/// `GGUF`, little-endian.
const MAGIC: [u8; 4] = *b"GGUF";

/// The versions this reader claims.
///
/// Version 1 used 32-bit counts and is not published any more; a file claiming
/// it is refused rather than read on the assumption that the rest of the layout
/// matches. 2 and 3 differ in ways this reader does not depend on.
const READABLE_VERSIONS: [u32; 2] = [2, 3];

/// The alignment a file gets when its metadata does not say.
///
/// The format's own default. Stated as a constant rather than inlined because
/// a file that omits the key is relying on it, and a reader that quietly used a
/// different number would read every tensor from the wrong offset.
pub const DEFAULT_ALIGNMENT: u64 = 32;

/// What a GGUF file says about itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The format version the file claims.
    pub version: u32,
    /// The metadata, by key, in the file's own order-independent form.
    pub metadata: BTreeMap<String, Value>,
    /// Every tensor the directory names, in the order it names them.
    pub tensors: Vec<Tensor>,
    /// Where the tensor data begins, in bytes from the start of the file.
    pub data_offset: u64,
    /// The alignment tensor offsets are relative to.
    pub alignment: u64,
}

impl Model {
    /// The metadata value at a key, if the file has one.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// The tensor of that name, if the directory names one.
    #[must_use]
    pub fn tensor(&self, name: &str) -> Option<&Tensor> {
        self.tensors.iter().find(|tensor| tensor.name == name)
    }

    /// What every tensor would weigh dequantized to `f32`, in bytes.
    ///
    /// The stand-in engine dequantizes on load (D38's legibility trade), so
    /// this is what *running* the file costs in memory before a single token —
    /// and it is arithmetic on the directory alone, so a surface can refuse a
    /// file that cannot fit without reading any tensor data (B-372). `None`
    /// when a tensor's extent is not computable, because a lower bound wearing
    /// the name of a total is what A7 forbids.
    #[must_use]
    pub fn dequantized_bytes(&self) -> Option<u64> {
        let mut total: u64 = 0;
        for tensor in &self.tensors {
            total = total.checked_add(tensor.elements()?.checked_mul(4)?)?;
        }
        Some(total)
    }

    /// Refuses this file where its dequantized weight exceeds what is free.
    ///
    /// Pure arithmetic on the directory against a number the caller observed:
    /// the machine's memory is read at the surface, so that this is the same
    /// judgement everywhere and testable without a machine of any particular
    /// size (§3.15). A directory whose extent is not computable passes — an
    /// unknown is not a refusal (A7) — and the loader will refuse it for the
    /// tensor it cannot read.
    ///
    /// # Errors
    ///
    /// `resource.memory.exhausted`, naming both numbers (B-372).
    pub fn fits_dequantized(&self, available: u64) -> Result<()> {
        let Some(needs) = self.dequantized_bytes() else {
            return Ok(());
        };
        if needs <= available {
            return Ok(());
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "display figures in gigabytes, two significant digits"
        )]
        Err(Failure::new(
            Category::ResourceMemoryExhausted,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "dequantized for the stand-in engine, this model is larger than the memory this \
             machine has free",
        )
        .with_context(
            "dequantized",
            format!("{needs} bytes ({:.1} GB)", needs as f64 / 1e9),
        )
        .with_context(
            "available",
            format!("{available} bytes ({:.1} GB)", available as f64 / 1e9),
        )
        .with_context(
            "what_to_do",
            "this is D38's dequantize-on-load ceiling, not a property of the model: a \
             vendored or provisioned engine (B-320, B-367) runs the same file in its \
             quantized form",
        ))
    }

    /// How many bytes of tensor data the directory accounts for.
    ///
    /// The end of the furthest tensor, which is what a caller needs in order to
    /// know whether it has the whole file. `None` when any tensor's size is not
    /// computable — an unknown quantization scheme, for instance — because the
    /// answer would then be a lower bound wearing the name of a total (A7).
    #[must_use]
    pub fn data_bytes_required(&self) -> Option<u64> {
        let mut furthest = 0;
        for tensor in &self.tensors {
            let extent = tensor.offset.checked_add(tensor.bytes()?)?;
            furthest = furthest.max(extent);
        }
        Some(furthest)
    }

    /// The architecture the file declares, which is what decides how the
    /// tensors are to be used.
    ///
    /// `None` rather than a default: an architecture MCF guessed at would be an
    /// inference run producing confident nonsense (A7).
    #[must_use]
    pub fn architecture(&self) -> Option<&str> {
        self.get("general.architecture").and_then(Value::as_text)
    }
}

/// One tensor, as the directory describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tensor {
    /// Its name, which is how a model's code refers to it.
    pub name: String,
    /// Its shape, fastest-varying dimension first, as GGUF writes it.
    pub dimensions: Vec<u64>,
    /// How its bytes are encoded.
    pub kind: TensorKind,
    /// Where it starts, relative to [`Model::data_offset`].
    pub offset: u64,
}

impl Tensor {
    /// How many elements it holds.
    ///
    /// `None` on an overflow rather than a wrapped product: a shape whose
    /// element count does not fit is a file this reader will not act on (A2).
    #[must_use]
    pub fn elements(&self) -> Option<u64> {
        self.dimensions
            .iter()
            .try_fold(1_u64, |total, dimension| total.checked_mul(*dimension))
    }

    /// How many bytes its data occupies, if that is computable.
    #[must_use]
    pub fn bytes(&self) -> Option<u64> {
        let elements = self.elements()?;
        let block = self.kind.block_size();
        if block == 0 || elements % block != 0 {
            // A quantized tensor's element count is a whole number of blocks by
            // construction. One that is not is a file that disagrees with
            // itself, and a rounded answer would read a neighbouring tensor's
            // bytes as this one's.
            return None;
        }
        elements
            .checked_div(block)?
            .checked_mul(self.kind.bytes_per_block())
    }
}

/// How a tensor's bytes are encoded.
///
/// The variants MCF's stand-in can *read*; the ones it can decode are the
/// dequantizer's business, and a type here is not a promise that it can. A
/// number this reader does not know is [`TensorKind::Unknown`] carrying the
/// number, because A7 forbids the plausible substitute and a file with one
/// unreadable tensor is still a file whose other tensors are legible (A4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[allow(
    non_camel_case_types,
    reason = "these are the names the format uses — Q4_K is what a file, a model \
              card and every tool call it, and a reader whose type names differ \
              from the format's would make every table a translation"
)]
pub enum TensorKind {
    /// 32-bit floating point, one element per element.
    F32,
    /// 16-bit floating point.
    F16,
    /// The 4-bit scheme with one scale per block of 32.
    Q4_0,
    /// The 4-bit scheme with a scale and a minimum per block of 32.
    Q4_1,
    /// Thirty-two five-bit offsets from sixteen: four low bits in nibbles and
    /// the fifth bit gathered into one word, against one half-precision scale.
    Q5_0,
    /// `Q5_0` with a minimum as well as a scale.
    Q5_1,
    /// The 8-bit scheme with one scale per block of 32.
    Q8_0,
    /// The 2-bit super-block scheme: 256 values, sixteen sub-blocks with
    /// 4-bit scales and minimums, one pair of half-precision multipliers.
    Q2_K,
    /// The 3-bit super-block scheme: 256 values, a high bit held apart from
    /// the low two, and six-bit scales.
    Q3_K,
    /// The 4-bit super-block scheme: 256 values in eight sub-blocks, each with
    /// a six-bit scale and a six-bit minimum against shared multipliers. This
    /// is what most of a `Q4_K_M` file is made of.
    Q4_K,
    /// The 5-bit super-block scheme: `Q4_K` with a fifth bit per value held in
    /// a separate plane.
    Q5_K,
    /// The 6-bit super-block scheme: 256 values, four low bits and two high
    /// bits held apart, with eight-bit signed scales. `Q4_K_M` uses it for the
    /// tensors it will not round further.
    Q6_K,
    /// The 4-bit non-linear scheme: codes are indices into a fixed sixteen-value
    /// table rather than multiples of a scale, over blocks of 32.
    IQ4_NL,
    /// The same table over a super-block of 256, with a six-bit scale per
    /// sub-block split across two planes. Most of what an *Unsloth Dynamic*
    /// quantization is made of.
    IQ4_XS,
    /// The 3-bit scheme that indexes a five-hundred-and-twelve entry grid of
    /// four values each, with a plane of sign bits and four-bit scales.
    IQ3_S,
    /// A type this reader does not know, carrying the number the file used.
    Unknown(u32),
}

impl TensorKind {
    /// The kind a written type number names.
    #[must_use]
    pub const fn from_number(number: u32) -> Self {
        match number {
            0 => Self::F32,
            1 => Self::F16,
            2 => Self::Q4_0,
            3 => Self::Q4_1,
            6 => Self::Q5_0,
            7 => Self::Q5_1,
            8 => Self::Q8_0,
            10 => Self::Q2_K,
            11 => Self::Q3_K,
            12 => Self::Q4_K,
            13 => Self::Q5_K,
            14 => Self::Q6_K,
            20 => Self::IQ4_NL,
            21 => Self::IQ3_S,
            23 => Self::IQ4_XS,
            other => Self::Unknown(other),
        }
    }

    /// How many elements share one block of encoded bytes.
    ///
    /// One for the unquantized kinds. Zero for an unknown kind, which is what
    /// makes [`Tensor::bytes`] refuse rather than compute a size for something
    /// it cannot interpret.
    #[must_use]
    pub const fn block_size(self) -> u64 {
        match self {
            Self::F32 | Self::F16 => 1,
            // `IQ4_NL` is here rather than beside the other non-linear scheme:
            // it shares the four-bit table with `IQ4_XS` and the block of 32
            // with `Q4_0`, and the block size is what this function is about.
            Self::Q4_0 | Self::Q4_1 | Self::Q5_0 | Self::Q5_1 | Self::Q8_0 | Self::IQ4_NL => 32,
            // The K-schemes share a super-block of 256, which is what makes
            // their per-sub-block scales worth their bytes.
            Self::Q2_K
            | Self::Q3_K
            | Self::Q4_K
            | Self::Q5_K
            | Self::Q6_K
            | Self::IQ4_XS
            | Self::IQ3_S => 256,
            Self::Unknown(_) => 0,
        }
    }

    /// How many bytes one block occupies.
    #[must_use]
    #[allow(
        clippy::match_same_arms,
        reason = "two schemes whose blocks happen to be the same size are still two \
                  schemes, and merging their arms would put one scheme's arithmetic \
                  under another's comment"
    )]
    pub const fn bytes_per_block(self) -> u64 {
        match self {
            Self::F32 => 4,
            Self::F16 => 2,
            // A scale in half precision, then 32 four-bit values.
            // One multiplier and sixteen bytes of four-bit codes, whichever way
            // the codes are read: `Q4_0`'s are offsets from eight, `IQ4_NL`'s
            // are indices into a table, and the block is the same size.
            Self::Q4_0 | Self::IQ4_NL => 2 + 16,
            // A scale and a minimum, then 32 four-bit values.
            Self::Q4_1 => 2 + 2 + 16,
            // A scale, four bytes holding the fifth bit of every value, then
            // the nibbles; `Q5_1` carries a minimum before the fifth bits.
            Self::Q5_0 => 2 + 4 + 16,
            Self::Q5_1 => 2 + 2 + 4 + 16,
            // A scale in half precision, then 32 signed bytes.
            Self::Q8_0 => 2 + 32,
            // 16 packed 4-bit scale/minimum pairs, 64 bytes of 2-bit values,
            // and the two half-precision multipliers.
            Self::Q2_K => 16 + 64 + 2 + 2,
            // A 32-byte high-bit plane, 64 bytes of low bits, 12 bytes of
            // packed 6-bit scales, and one multiplier.
            Self::Q3_K => 32 + 64 + 12 + 2,
            // Two multipliers, 12 bytes of packed 6-bit scales and minimums,
            // and 128 bytes of 4-bit values.
            Self::Q4_K => 2 + 2 + 12 + 128,
            // The same, with a 32-byte plane for the fifth bit.
            Self::Q5_K => 2 + 2 + 12 + 32 + 128,
            // 128 bytes of low nibbles, 64 of high pairs, 16 signed scales and
            // one multiplier.
            Self::Q6_K => 128 + 64 + 16 + 2,
            // A multiplier, six-bit scales split across a sixteen-bit plane and
            // four bytes, then 128 bytes of indices.
            Self::IQ4_XS => 2 + 2 + 4 + 128,
            // A multiplier, 64 bytes of grid indices, 8 of high bits, 32 of
            // signs and 4 of scales.
            Self::IQ3_S => 2 + 64 + 8 + 32 + 4,
            Self::Unknown(_) => 0,
        }
    }

    /// Whether this reader knows what the bytes mean.
    #[must_use]
    pub const fn is_known(self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

impl core::fmt::Display for TensorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::F32 => f.write_str("f32"),
            Self::F16 => f.write_str("f16"),
            Self::Q2_K => f.write_str("Q2_K"),
            Self::Q3_K => f.write_str("Q3_K"),
            Self::Q4_K => f.write_str("Q4_K"),
            Self::Q5_K => f.write_str("Q5_K"),
            Self::Q6_K => f.write_str("Q6_K"),
            Self::IQ4_NL => f.write_str("IQ4_NL"),
            Self::IQ4_XS => f.write_str("IQ4_XS"),
            Self::IQ3_S => f.write_str("IQ3_S"),
            Self::Q4_0 => f.write_str("q4_0"),
            Self::Q4_1 => f.write_str("q4_1"),
            Self::Q5_0 => f.write_str("q5_0"),
            Self::Q5_1 => f.write_str("q5_1"),
            Self::Q8_0 => f.write_str("q8_0"),
            Self::Unknown(number) => write!(f, "unknown type {number}"),
        }
    }
}

/// A metadata value, in the shapes GGUF permits.
///
/// Integers arrive in six widths and are kept as `i64` with the width they were
/// written in discarded, because nothing MCF does with a context length depends
/// on whether it was written as a `u32` or a `u64` — and an unsigned 64-bit
/// value too large for `i64` is refused rather than wrapped (A2).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// An integer of any of the format's widths.
    Integer(i64),
    /// A 32- or 64-bit float, kept at the width it arrived in.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A string.
    Text(String),
    /// An array of values, all of one type.
    List(Vec<Value>),
}

impl Value {
    /// The text, if this is one.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The integer, if this is one.
    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// The value as a flag, where it is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The list, if this is one.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }
}

/// Reads a model file's structure.
///
/// # Errors
///
/// `artifact.unreadable` when the file cannot be opened or read;
/// `artifact.format.unsupported` when it is not GGUF or uses a construct this
/// reader does not read; `artifact.format.malformed` when it is GGUF and
/// disagrees with itself. Every one of them names what was expected and where,
/// because a model that will not load is a thing an operator has to act on
/// (A2).
///
/// The two format categories are a real distinction and not a shade: *this is
/// not something MCF reads* is answered by acquiring a different file or by
/// MCF growing a reader, and *this is a GGUF that contradicts itself* is
/// answered by re-fetching it.
pub fn read(path: &Path) -> Result<Model> {
    let bytes = std::fs::read(path).map_err(|error| {
        Failure::new(
            Category::ArtifactUnreadable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the model file could not be read",
        )
        .with_context("path", path.display().to_string())
        .with_context("os_error", error.to_string())
    })?;
    let model = parse(&bytes)
        .map_err(|failure| failure.with_context("path", path.display().to_string()))?;

    // `parse` guarantees the directory agrees with itself; a file on disk can
    // also be checked against its own length. The two are separate on purpose:
    // a caller holding only the head of a download — B-213's pre-acquisition
    // fitment is exactly that — can read the directory of a file it does not
    // have all of, and a *file* that is short is a truncated download and is
    // refused here.
    // `required > 0` matters: a file with no tensors needs no data region, and
    // its alignment padding is not something the writer has to have emitted.
    // The laboratory found this on the scenario for a model that declares an
    // architecture and nothing else — a real file that a stricter reading
    // refused for the wrong reason.
    if let Some(required) = model.data_bytes_required()
        && required > 0
    {
        let held = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let needed = model.data_offset.saturating_add(required);
        if needed > held {
            return Err(malformed(
                "the file's directory names more bytes than the file holds",
                &format!("needs {needed} bytes and the file is {held}"),
            )
            .with_context("path", path.display().to_string()));
        }
    }
    Ok(model)
}

/// The same, from bytes already in hand.
///
/// Separated so that the fuzz tier and the tests can hand it damaged input
/// without a file, which is the shape `Watch::judge` and `Zone::parse` already
/// use for the same reason (D26).
///
/// # Errors
///
/// As [`read`].
pub fn parse(bytes: &[u8]) -> Result<Model> {
    let mut cursor = Cursor::new(bytes);

    let magic = cursor.take(4, "the magic number")?;
    if magic != MAGIC {
        return Err(unsupported(
            "the file does not begin with GGUF",
            &format!("{magic:?}"),
        ));
    }

    let version = cursor.u32("the format version")?;
    if !READABLE_VERSIONS.contains(&version) {
        return Err(unsupported(
            "the file claims a GGUF version this reader does not read",
            &format!("version {version}, and this reader reads {READABLE_VERSIONS:?}"),
        ));
    }

    let tensor_count = cursor.u64("the tensor count")?;
    let metadata_count = cursor.u64("the metadata count")?;

    // Both counts are checked against what the file could possibly hold before
    // anything is allocated. A file claiming eighteen quintillion tensors is
    // not a large model; it is a header a fuzzer produced, and MCF must refuse
    // it in constant space.
    let remaining = u64::try_from(bytes.len().saturating_sub(cursor.at)).unwrap_or(u64::MAX);
    if metadata_count > remaining || tensor_count > remaining {
        return Err(malformed(
            "the file claims more entries than its own length could hold",
            &format!(
                "{metadata_count} metadata entries and {tensor_count} tensors in {remaining} remaining bytes"
            ),
        ));
    }

    let mut metadata = BTreeMap::new();
    for index in 0..metadata_count {
        let key = cursor.string(&format!("metadata key {index}"))?;
        let kind = cursor.u32(&format!("the type of metadata {key}"))?;
        let value = cursor.value(kind, &key)?;
        if metadata.insert(key.clone(), value).is_some() {
            // The same discipline `mcf_record::json` applies to a duplicate
            // key: two readers would disagree about what the file says, and a
            // model's metadata decides how its tensors are used.
            return Err(malformed("the file states one metadata key twice", &key));
        }
    }

    let mut tensors = Vec::new();
    for index in 0..tensor_count {
        tensors.push(cursor.tensor(index)?);
    }

    let alignment = metadata
        .get("general.alignment")
        .and_then(Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(DEFAULT_ALIGNMENT);
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(malformed(
            "the file declares an alignment that is not a power of two",
            &alignment.to_string(),
        ));
    }

    // The directory has to be arithmetically consistent with itself before it
    // is handed to anything that will read bytes at those offsets. The fuzz
    // tier found this on its first campaign: a damaged file declared a tensor
    // at offset 1.5 × 10^19 of a size that overflowed when added to it, and the
    // reader accepted the directory. Nothing downstream could have used it
    // safely, and a dequantizer would have been the one to discover that.
    for tensor in &tensors {
        if tensor.elements().is_none() {
            return Err(malformed(
                "a tensor's shape multiplies out past what can be counted",
                &format!("{}: {:?}", tensor.name, tensor.dimensions),
            ));
        }
        if let Some(bytes) = tensor.bytes()
            && tensor.offset.checked_add(bytes).is_none()
        {
            return Err(malformed(
                "a tensor ends past the end of addressable space",
                &format!(
                    "{}: {} bytes at offset {}",
                    tensor.name, bytes, tensor.offset
                ),
            ));
        }
    }

    let at = u64::try_from(cursor.at).unwrap_or(u64::MAX);
    let data_offset = at.checked_next_multiple_of(alignment).ok_or_else(|| {
        malformed(
            "the tensor data would begin past the end of addressable space",
            &at.to_string(),
        )
    })?;

    Ok(Model {
        version,
        metadata,
        tensors,
        data_offset,
        alignment,
    })
}

/// A position in the bytes, and the readers that move it.
///
/// A struct rather than an index passed around, so that every read is bounds-
/// checked in one place and a reader that forgot to check is one that does not
/// exist.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// The next `count` bytes, or a failure naming what was being read.
    fn take(&mut self, count: usize, what: &str) -> Result<&'a [u8]> {
        let end = self.at.checked_add(count).ok_or_else(|| {
            malformed(
                "a length in the file is larger than the file could hold",
                what,
            )
        })?;
        let slice = self.bytes.get(self.at..end).ok_or_else(|| {
            malformed(
                "the file ends before something it says is there",
                &format!("{what}: wanted {count} bytes at offset {}", self.at),
            )
        })?;
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self, what: &str) -> Result<u32> {
        let bytes = self.take(4, what)?;
        Ok(u32::from_le_bytes([
            bytes.first().copied().unwrap_or(0),
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
        ]))
    }

    fn u64(&mut self, what: &str) -> Result<u64> {
        let bytes = self.take(8, what)?;
        let mut value = [0_u8; 8];
        for (slot, byte) in value.iter_mut().zip(bytes.iter()) {
            *slot = *byte;
        }
        Ok(u64::from_le_bytes(value))
    }

    /// A length-prefixed string.
    fn string(&mut self, what: &str) -> Result<String> {
        let length = self.u64(&format!("the length of {what}"))?;
        let length = usize::try_from(length).map_err(|_| {
            malformed(
                "a string length does not fit this machine's addressing",
                what,
            )
        })?;
        let bytes = self.take(length, what)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| {
            malformed(
                "a string in the file is not UTF-8",
                &format!("{what}: {error}"),
            )
        })
    }

    /// One metadata value of a stated type.
    fn value(&mut self, kind: u32, key: &str) -> Result<Value> {
        // The type numbers are the format's, and the widths with them. An
        // unsigned 64-bit value that does not fit `i64` is refused rather than
        // wrapped: a wrapped context length is a negative context length.
        match kind {
            0 => Ok(Value::Integer(i64::from(i8::from_le_bytes([first(
                self.take(1, key)?,
            )])))),
            1 => Ok(Value::Integer(i64::from(first(self.take(1, key)?)))),
            2 => {
                let raw = self.take(2, key)?;
                Ok(Value::Integer(i64::from(i16::from_le_bytes([
                    first(raw),
                    raw.get(1).copied().unwrap_or(0),
                ]))))
            }
            3 => {
                let raw = self.take(2, key)?;
                Ok(Value::Integer(i64::from(u16::from_le_bytes([
                    first(raw),
                    raw.get(1).copied().unwrap_or(0),
                ]))))
            }
            4 => {
                let value = self.u32(key)?;
                Ok(Value::Integer(i64::from(i32::from_le_bytes(
                    value.to_le_bytes(),
                ))))
            }
            5 => Ok(Value::Integer(i64::from(self.u32(key)?))),
            6 => {
                let value = self.u32(key)?;
                Ok(Value::Float(f64::from(f32::from_bits(value))))
            }
            7 => Ok(Value::Bool(first(self.take(1, key)?) != 0)),
            8 => Ok(Value::Text(self.string(key)?)),
            9 => self.list(key),
            10 => {
                let value = self.u64(key)?;
                Ok(Value::Integer(i64::from_le_bytes(value.to_le_bytes())))
            }
            11 => {
                let value = self.u64(key)?;
                i64::try_from(value).map(Value::Integer).map_err(|_| {
                    malformed(
                        "an unsigned value in the file is too large to represent",
                        &format!("{key}: {value}"),
                    )
                })
            }
            12 => {
                let value = self.u64(key)?;
                Ok(Value::Float(f64::from_bits(value)))
            }
            other => Err(unsupported(
                "the file uses a metadata type this reader does not know",
                &format!("{key}: type {other}"),
            )),
        }
    }

    /// An array: a type, a count, and that many values.
    fn list(&mut self, key: &str) -> Result<Value> {
        let kind = self.u32(&format!("the element type of {key}"))?;
        if kind == 9 {
            // The format permits it and nothing publishes it; a reader that
            // supported it would be carrying recursion depth for a case that
            // does not occur, and an unbounded one at that (§3.13).
            return Err(unsupported("the file nests an array inside an array", key));
        }
        let count = self.u64(&format!("the length of {key}"))?;
        let remaining = u64::try_from(self.bytes.len().saturating_sub(self.at)).unwrap_or(u64::MAX);
        if count > remaining {
            return Err(malformed(
                "an array claims more elements than the file could hold",
                &format!("{key}: {count} elements in {remaining} remaining bytes"),
            ));
        }
        let count = usize::try_from(count).map_err(|_| {
            malformed(
                "an array length does not fit this machine's addressing",
                key,
            )
        })?;
        let mut values = Vec::new();
        for index in 0..count {
            values.push(self.value(kind, &format!("{key}[{index}]"))?);
        }
        Ok(Value::List(values))
    }

    /// One entry of the tensor directory.
    fn tensor(&mut self, index: u64) -> Result<Tensor> {
        let name = self.string(&format!("the name of tensor {index}"))?;
        let rank = self.u32(&format!("the rank of {name}"))?;
        // Four is the format's own maximum. A file claiming more is refused
        // rather than read, because the dimensions that follow decide every
        // offset after them.
        if rank > 4 {
            return Err(malformed(
                "a tensor claims more dimensions than the format permits",
                &format!("{name}: rank {rank}"),
            ));
        }
        let mut dimensions = Vec::new();
        for axis in 0..rank {
            dimensions.push(self.u64(&format!("dimension {axis} of {name}"))?);
        }
        let kind = TensorKind::from_number(self.u32(&format!("the type of {name}"))?);
        let offset = self.u64(&format!("the offset of {name}"))?;
        Ok(Tensor {
            name,
            dimensions,
            kind,
            offset,
        })
    }
}

fn first(bytes: &[u8]) -> u8 {
    bytes.first().copied().unwrap_or(0)
}

fn unsupported(detail: &str, found: &str) -> Failure {
    Failure::new(
        Category::ArtifactFormatUnsupported,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        detail,
    )
    .with_context("found", found.to_owned())
}

fn malformed(detail: &str, found: &str) -> Failure {
    Failure::new(
        Category::ArtifactFormatMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        detail,
    )
    .with_context("found", found.to_owned())
}

#[cfg(test)]
mod tests;
