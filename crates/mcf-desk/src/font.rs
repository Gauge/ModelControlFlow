#![allow(
    unsafe_code,
    reason = "stb_truetype is a C interface and rasterising has no safe form"
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a font size and a glyph's box are bounded by the atlas, which is \
              2048 pixels at the largest — exact in f32, and checked against \
              the atlas before anything is written"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Glyph {
    pub x0: i16,
    pub y0: i16,
    pub x1: i16,
    pub y1: i16,
    pub x_off: f32,
    pub y_off: f32,
    pub advance: f32,
}

#[cfg(have_font)]
mod c {
    use super::Glyph;
    use std::ffi::c_int;

    unsafe extern "C" {
        pub(super) fn mcf_font_metrics(
            ttf: *const u8,
            index: c_int,
            pixels: f32,
            ascent: *mut f32,
            descent: *mut f32,
            line_gap: *mut f32,
        ) -> c_int;
        pub(super) fn mcf_font_bake(
            ttf: *const u8,
            index: c_int,
            pixels: f32,
            codepoints: *const u32,
            count: c_int,
            atlas: *mut u8,
            width: c_int,
            height: c_int,
            out: *mut Glyph,
        ) -> c_int;
        pub(super) fn mcf_glyph_bytes() -> c_int;
    }
}

fn repertoire() -> Vec<u32> {
    let mut wanted: Vec<u32> = (0x20..0x7F).collect();
    wanted.extend(0xA0..0x100_u32);
    wanted.extend([
        0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026, 0x2190, 0x2191, 0x2192,
        0x2193, 0x2713, 0x2717, 0x25B2, 0x25B4, 0x25B6, 0x25B8, 0x25BC, 0x25BE, 0x25C0, 0x25C4,
        0x2022, 0x00B7,
    ]);
    wanted.sort_unstable();
    wanted.dedup();
    wanted
}

#[derive(Debug, Clone)]
pub struct Face {
    bytes: Vec<u8>,
    pub source: PathBuf,
}

#[derive(Debug)]
pub struct Atlas {
    pub coverage: Vec<u8>,
    pub width: u32,
    pub height: u32,
    codepoints: Vec<u32>,
    glyphs: Vec<Glyph>,
    pub ascent: f32,
    pub descent: f32,
    pub line: f32,
}

impl Atlas {
    #[must_use]
    pub fn glyph(&self, ch: char) -> Option<Glyph> {
        let point = ch as u32;
        let at = self.codepoints.binary_search(&point).ok()?;
        self.glyphs.get(at).copied()
    }

    #[must_use]
    pub fn advance_of(&self, ch: char) -> f32 {
        self.glyph(ch)
            .map_or_else(|| self.missing_width(), |glyph| glyph.advance)
    }

    #[must_use]
    pub fn missing_width(&self) -> f32 {
        self.glyph('n')
            .map_or(self.ascent * 0.6, |glyph| glyph.advance)
    }

    #[must_use]
    pub fn can_draw(&self, ch: char) -> bool {
        self.glyph(ch).is_some()
    }

    #[must_use]
    pub fn width_of(&self, text: &str) -> f32 {
        text.chars().map(|ch| self.advance_of(ch)).sum()
    }

    #[must_use]
    pub fn elide(&self, text: &str, room: f32) -> String {
        if self.width_of(text) <= room {
            return text.to_owned();
        }
        let dots = self.width_of("…");
        let mut kept = String::new();
        let mut used = 0.0_f32;
        for ch in text.chars() {
            let step = self.advance_of(ch);
            if used + step + dots > room {
                break;
            }
            used += step;
            kept.push(ch);
        }
        kept.push('…');
        kept
    }

    #[must_use]
    pub fn wrap(&self, text: &str, room: f32) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if self.width_of(&candidate) <= room || line.is_empty() {
                line = candidate;
            } else {
                lines.push(std::mem::take(&mut line));
                word.clone_into(&mut line);
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    Regular,
    Bold,
}

#[derive(Debug)]
pub struct Text {
    regular: Face,
    bold: Option<Face>,
    baked: BTreeMap<(Weight, u32), std::sync::Arc<Atlas>>,
}

static BAKED: std::sync::OnceLock<std::sync::Mutex<Baked>> = std::sync::OnceLock::new();

type Baked = BTreeMap<(PathBuf, Weight, u32), std::sync::Arc<Atlas>>;

impl Text {
    pub fn found() -> Result<Self, String> {
        static FOUND: std::sync::OnceLock<Result<(Face, Option<Face>), String>> =
            std::sync::OnceLock::new();
        let (regular, bold) = FOUND
            .get_or_init(|| {
                let regular = discover(Weight::Regular)?;
                Ok((regular, discover(Weight::Bold).ok()))
            })
            .clone()?;
        Ok(Self {
            regular,
            bold,
            baked: BTreeMap::new(),
        })
    }

    #[must_use]
    pub fn from_faces(regular: Face, bold: Option<Face>) -> Self {
        Self {
            regular,
            bold,
            baked: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn source(&self) -> &Path {
        &self.regular.source
    }

    #[must_use]
    pub fn has_bold(&self) -> bool {
        self.bold.is_some()
    }

    pub fn at(&mut self, weight: Weight, size: f32) -> Result<&Atlas, String> {
        let key = (weight, tenths(size));
        if !self.baked.contains_key(&key) {
            let face = match weight {
                Weight::Bold => self.bold.as_ref().unwrap_or(&self.regular),
                Weight::Regular => &self.regular,
            };
            let shared = (face.source.clone(), weight, key.1);
            let held = BAKED
                .get_or_init(|| std::sync::Mutex::new(BTreeMap::new()))
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&shared)
                .cloned();
            let atlas = if let Some(atlas) = held {
                atlas
            } else {
                let atlas = std::sync::Arc::new(bake(face, size)?);
                let _kept = BAKED
                    .get_or_init(|| std::sync::Mutex::new(BTreeMap::new()))
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(shared, std::sync::Arc::clone(&atlas));
                atlas
            };
            let _replaced = self.baked.insert(key, atlas);
        }
        self.baked
            .get(&key)
            .map(std::convert::AsRef::as_ref)
            .ok_or_else(|| "a size was baked and then was not there".to_owned())
    }
}

fn tenths(size: f32) -> u32 {
    let scaled = size * 10.0;
    if scaled.is_finite() && scaled > 0.0 {
        scaled.round() as u32
    } else {
        0
    }
}

fn bake(face: &Face, size: f32) -> Result<Atlas, String> {
    let codepoints = repertoire();
    let mut glyphs = vec![Glyph::default(); codepoints.len()];

    let Ok(count) = i32::try_from(codepoints.len()) else {
        return Err("more characters were wanted than can be counted".to_owned());
    };

    for side in [256_u32, 512, 1024, 2048] {
        let Ok(edge) = i32::try_from(side) else {
            continue;
        };
        let area = (side as usize).saturating_mul(side as usize);
        let mut coverage = vec![0_u8; area];

        let fitted = unsafe {
            c::mcf_font_bake(
                face.bytes.as_ptr(),
                0,
                size,
                codepoints.as_ptr(),
                count,
                coverage.as_mut_ptr(),
                edge,
                edge,
                glyphs.as_mut_ptr(),
            )
        };

        if fitted == 1 {
            let (ascent, descent, line_gap) = metrics(face, size)?;
            return Ok(Atlas {
                coverage,
                width: side,
                height: side,
                codepoints,
                glyphs,
                ascent,
                descent,
                line: ascent - descent + line_gap,
            });
        }
    }
    Err(format!(
        "{} could not be rasterised at {size} pixels, even into a 2048-pixel atlas",
        face.source.display()
    ))
}

fn metrics(face: &Face, size: f32) -> Result<(f32, f32, f32), String> {
    let (mut ascent, mut descent, mut line_gap) = (0.0_f32, 0.0_f32, 0.0_f32);
    let read = unsafe {
        c::mcf_font_metrics(
            face.bytes.as_ptr(),
            0,
            size,
            &raw mut ascent,
            &raw mut descent,
            &raw mut line_gap,
        )
    };
    if read == 1 {
        Ok((ascent, descent, line_gap))
    } else {
        Err(format!(
            "{} is not a font this can read",
            face.source.display()
        ))
    }
}

#[must_use]
pub fn glyph_bytes_in_c() -> usize {
    let bytes = unsafe { c::mcf_glyph_bytes() };
    usize::try_from(bytes).unwrap_or(0)
}

const FAMILIES: &[(&str, &str)] = &[
    ("Inter-Regular", "Inter-Bold"),
    ("Inter_24pt-Regular", "Inter_24pt-Bold"),
    ("Cantarell-Regular", "Cantarell-Bold"),
    ("AdwaitaSans-Regular", "AdwaitaSans-Bold"),
    ("SourceSans3-Regular", "SourceSans3-Bold"),
    ("NotoSans-Regular", "NotoSans-Bold"),
    ("Roboto-Regular", "Roboto-Bold"),
    ("OpenSans-Regular", "OpenSans-Bold"),
    ("Ubuntu-R", "Ubuntu-B"),
    ("SegoeUI", "SegoeUIB"),
    ("segoeui", "segoeuib"),
    ("Helvetica", "Helvetica-Bold"),
    ("DejaVuSans", "DejaVuSans-Bold"),
    ("LiberationSans-Regular", "LiberationSans-Bold"),
    ("Arial", "Arial-Bold"),
    ("arial", "arialbd"),
];

fn font_directories() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share/fonts"));
        roots.push(home.join(".fonts"));
        roots.push(home.join("Library/Fonts"));
    }
    if let Some(windir) = std::env::var_os("WINDIR") {
        roots.push(PathBuf::from(windir).join("Fonts"));
    }
    roots.extend(
        [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/run/host/fonts",
            "/run/host/user-fonts",
            "/run/host/local-fonts",
            "/System/Library/Fonts",
            "/Library/Fonts",
            "C:\\Windows\\Fonts",
        ]
        .iter()
        .map(PathBuf::from),
    );
    roots
}

fn discover(weight: Weight) -> Result<Face, String> {
    let roots = font_directories();
    for (regular, bold) in FAMILIES {
        let stem = match weight {
            Weight::Regular => regular,
            Weight::Bold => bold,
        };
        for root in &roots {
            if let Some(found) = look_in(root, stem, 0) {
                return match std::fs::read(&found) {
                    Ok(bytes) => Ok(Face {
                        bytes,
                        source: found,
                    }),
                    Err(error) => Err(format!("{} could not be read: {error}", found.display())),
                };
            }
        }
    }
    Err(format!(
        "no font was found on this computer. {} were searched, for any of {} families",
        roots.len(),
        FAMILIES.len()
    ))
}

fn look_in(root: &Path, stem: &str, depth: u32) -> Option<PathBuf> {
    if depth > 4 {
        return None;
    }
    let mut directories: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            directories.push(path);
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lowered = name.to_ascii_lowercase();
        let wanted = stem.to_ascii_lowercase();
        if lowered == format!("{wanted}.ttf") || lowered == format!("{wanted}.otf") {
            return Some(path);
        }
    }
    directories
        .into_iter()
        .find_map(|directory| look_in(&directory, stem, depth + 1))
}

impl Face {
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>, source: PathBuf) -> Self {
        Self { bytes, source }
    }
}

#[cfg(not(have_font))]
mod c {
    use super::Glyph;
    use std::ffi::c_int;

    pub(super) unsafe fn mcf_font_metrics(
        _: *const u8,
        _: c_int,
        _: f32,
        _: *mut f32,
        _: *mut f32,
        _: *mut f32,
    ) -> c_int {
        0
    }
    #[expect(clippy::too_many_arguments, reason = "it mirrors the C signature")]
    pub(super) unsafe fn mcf_font_bake(
        _: *const u8,
        _: c_int,
        _: f32,
        _: *const u32,
        _: c_int,
        _: *mut u8,
        _: c_int,
        _: c_int,
        _: *mut Glyph,
    ) -> c_int {
        0
    }
    pub(super) unsafe fn mcf_glyph_bytes() -> c_int {
        0
    }
}
