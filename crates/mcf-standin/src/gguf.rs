use std::collections::BTreeMap;
use std::path::Path;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-standin::gguf");

const MAGIC: [u8; 4] = *b"GGUF";

const READABLE_VERSIONS: [u32; 2] = [2, 3];

pub const DEFAULT_ALIGNMENT: u64 = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub version: u32,
    pub metadata: BTreeMap<String, Value>,
    pub tensors: Vec<Tensor>,
    pub data_offset: u64,
    pub alignment: u64,
}

impl Model {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    #[must_use]
    pub fn tensor(&self, name: &str) -> Option<&Tensor> {
        self.tensors.iter().find(|tensor| tensor.name == name)
    }

    #[must_use]
    pub fn dequantized_bytes(&self) -> Option<u64> {
        let mut total: u64 = 0;
        for tensor in &self.tensors {
            total = total.checked_add(tensor.elements()?.checked_mul(4)?)?;
        }
        Some(total)
    }

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

    #[must_use]
    pub fn data_bytes_required(&self) -> Option<u64> {
        let mut furthest = 0;
        for tensor in &self.tensors {
            let extent = tensor.offset.checked_add(tensor.bytes()?)?;
            furthest = furthest.max(extent);
        }
        Some(furthest)
    }

    #[must_use]
    pub fn architecture(&self) -> Option<&str> {
        self.get("general.architecture").and_then(Value::as_text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tensor {
    pub name: String,
    pub dimensions: Vec<u64>,
    pub kind: TensorKind,
    pub offset: u64,
}

impl Tensor {
    #[must_use]
    pub fn elements(&self) -> Option<u64> {
        self.dimensions
            .iter()
            .try_fold(1_u64, |total, dimension| total.checked_mul(*dimension))
    }

    #[must_use]
    pub fn bytes(&self) -> Option<u64> {
        let elements = self.elements()?;
        let block = self.kind.block_size();
        if block == 0 || elements % block != 0 {
            return None;
        }
        elements
            .checked_div(block)?
            .checked_mul(self.kind.bytes_per_block())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[allow(
    non_camel_case_types,
    reason = "these are the names the format uses — Q4_K is what a file, a model \
              card and every tool call it, and a reader whose type names differ \
              from the format's would make every table a translation"
)]
pub enum TensorKind {
    F32,
    F16,
    BF16,
    Q4_0,
    Q4_1,
    Q5_0,
    Q5_1,
    Q8_0,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
    IQ4_NL,
    IQ4_XS,
    IQ3_S,
    MXFP4,
    Unknown(u32),
}

impl TensorKind {
    #[must_use]
    pub const fn from_number(number: u32) -> Self {
        match number {
            0 => Self::F32,
            1 => Self::F16,
            30 => Self::BF16,
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
            39 => Self::MXFP4,
            other => Self::Unknown(other),
        }
    }

    #[must_use]
    pub const fn block_size(self) -> u64 {
        match self {
            Self::F32 | Self::F16 | Self::BF16 => 1,
            Self::Q4_0
            | Self::Q4_1
            | Self::Q5_0
            | Self::Q5_1
            | Self::Q8_0
            | Self::IQ4_NL
            | Self::MXFP4 => 32,
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
            Self::F16 | Self::BF16 => 2,
            Self::Q4_0 | Self::IQ4_NL => 2 + 16,
            Self::Q4_1 => 2 + 2 + 16,
            Self::Q5_0 => 2 + 4 + 16,
            Self::Q5_1 => 2 + 2 + 4 + 16,
            Self::Q8_0 => 2 + 32,
            Self::Q2_K => 16 + 64 + 2 + 2,
            Self::Q3_K => 32 + 64 + 12 + 2,
            Self::Q4_K => 2 + 2 + 12 + 128,
            Self::Q5_K => 2 + 2 + 12 + 32 + 128,
            Self::Q6_K => 128 + 64 + 16 + 2,
            Self::IQ4_XS => 2 + 2 + 4 + 128,
            Self::IQ3_S => 2 + 64 + 8 + 32 + 4,
            Self::MXFP4 => 1 + 16,
            Self::Unknown(_) => 0,
        }
    }

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
            Self::BF16 => f.write_str("bf16"),
            Self::Q2_K => f.write_str("Q2_K"),
            Self::Q3_K => f.write_str("Q3_K"),
            Self::Q4_K => f.write_str("Q4_K"),
            Self::Q5_K => f.write_str("Q5_K"),
            Self::Q6_K => f.write_str("Q6_K"),
            Self::IQ4_NL => f.write_str("IQ4_NL"),
            Self::IQ4_XS => f.write_str("IQ4_XS"),
            Self::IQ3_S => f.write_str("IQ3_S"),
            Self::MXFP4 => f.write_str("MXFP4"),
            Self::Q4_0 => f.write_str("q4_0"),
            Self::Q4_1 => f.write_str("q4_1"),
            Self::Q5_0 => f.write_str("q5_0"),
            Self::Q5_1 => f.write_str("q5_1"),
            Self::Q8_0 => f.write_str("q8_0"),
            Self::Unknown(number) => write!(f, "unknown type {number}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    List(Vec<Value>),
}

impl Value {
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }
}

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

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

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

    fn value(&mut self, kind: u32, key: &str) -> Result<Value> {
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

    fn list(&mut self, key: &str) -> Result<Value> {
        let kind = self.u32(&format!("the element type of {key}"))?;
        if kind == 9 {
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

    fn tensor(&mut self, index: u64) -> Result<Tensor> {
        let name = self.string(&format!("the name of tensor {index}"))?;
        let rank = self.u32(&format!("the rank of {name}"))?;
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
