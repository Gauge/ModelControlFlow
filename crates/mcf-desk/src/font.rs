//! Real type: an outline font off this machine, rasterised into an atlas.
//!
//! **The window's first design drew text with SDL's 8×8 debug font**, because
//! that font costs no component and the console's layout was being reused at a
//! different scale. What it produced was a terminal in a window. Proportional
//! type at real sizes is the difference between an application and a grid, so
//! this module exists and `csrc/font.c` exists under it.
//!
//! Nothing here bakes eagerly. A size is rasterised the first time something
//! asks to draw at it, which keeps startup to the one font file that had to be
//! read anyway and means an unused size costs nothing.

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

/// Where one glyph landed in the atlas, and what it does to the pen.
///
/// `#[repr(C)]`, and the mirror of `mcf_glyph` in `csrc/font.c`. The two sizes
/// are compared in a test rather than assumed to agree.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Glyph {
    /// The glyph's box within the atlas.
    pub x0: i16,
    /// The glyph's box within the atlas.
    pub y0: i16,
    /// The glyph's box within the atlas.
    pub x1: i16,
    /// The glyph's box within the atlas.
    pub y1: i16,
    /// Where to put that box, relative to the pen.
    pub x_off: f32,
    /// Where to put that box, relative to the pen.
    pub y_off: f32,
    /// How far the pen then moves.
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

/// Every character MCF's own interface can put on the screen.
///
/// Not a contiguous run, which is why stb's own baker is not the one used: the
/// degree sign, the middot and the multiplication sign are wanted and the two
/// hundred accented letters between them are not. Latin-1 is carried whole
/// because model names and file paths come off somebody else's machine and are
/// not MCF's to restrict; the arrows, dashes and quotation marks after it are
/// MCF's own furniture.
fn repertoire() -> Vec<u32> {
    let mut wanted: Vec<u32> = (0x20..0x7F).collect();
    wanted.extend(0xA0..0x100_u32);
    wanted.extend([
        0x2013, 0x2014, // en and em dash
        0x2018, 0x2019, 0x201C, 0x201D, // quotation marks
        0x2022, 0x2026, // bullet, ellipsis
        0x2190, 0x2191, 0x2192, 0x2193, // arrows
        0x2713, 0x2717, // check, cross
        // The furniture of a control: the triangle on a dropdown, and the
        // markers a list uses. Without these a chevron is simply absent, and
        // a control that looks like a field but opens like a menu is one
        // nobody presses.
        0x25B2, 0x25B4, 0x25B6, 0x25B8, 0x25BC, 0x25BE, 0x25C0, 0x25C4, 0x2022,
        0x00B7, // bullet, middot
    ]);
    wanted.sort_unstable();
    wanted.dedup();
    wanted
}

/// A font file, held in memory because stb reads from it on every bake.
#[derive(Debug)]
pub struct Face {
    bytes: Vec<u8>,
    /// The file this came from, so that what MCF is drawing with can be said
    /// out loud rather than guessed at.
    pub source: PathBuf,
}

/// One size of one face, rasterised.
#[derive(Debug)]
pub struct Atlas {
    /// Eight-bit coverage, `width × height`, row-major.
    pub coverage: Vec<u8>,
    /// The atlas dimensions.
    pub width: u32,
    /// The atlas dimensions.
    pub height: u32,
    codepoints: Vec<u32>,
    glyphs: Vec<Glyph>,
    /// Distance from the baseline to the top of the tallest letter.
    pub ascent: f32,
    /// Distance from the baseline downward. Negative, as fonts report it.
    pub descent: f32,
    /// What one line of this size occupies, top to top.
    pub line: f32,
}

impl Atlas {
    /// The glyph for a character, or `None` if this face has nothing for it.
    #[must_use]
    pub fn glyph(&self, ch: char) -> Option<Glyph> {
        let point = ch as u32;
        let at = self.codepoints.binary_search(&point).ok()?;
        self.glyphs.get(at).copied()
    }

    /// How wide one character is, including one this face cannot draw.
    ///
    /// **A character with no glyph is not a character of no width.** The face
    /// MCF found covers Latin and a little furniture; a model named in
    /// Japanese, Arabic, Devanagari or Han has characters it has never seen.
    /// Skipping them drew `モデル-7B` as `-7B` — a name silently shorter than
    /// the name, and two different models that render identically (A1, A2).
    /// Each one takes the width of the box drawn in its place.
    #[must_use]
    pub fn advance_of(&self, ch: char) -> f32 {
        self.glyph(ch)
            .map_or_else(|| self.missing_width(), |glyph| glyph.advance)
    }

    /// How wide the box standing in for a character this face cannot draw.
    ///
    /// Measured off a character the face certainly has, so the box is in
    /// proportion at every size rather than a constant that is right at one.
    #[must_use]
    pub fn missing_width(&self) -> f32 {
        self.glyph('n')
            .map_or(self.ascent * 0.6, |glyph| glyph.advance)
    }

    /// Whether this face can draw a character at all.
    #[must_use]
    pub fn can_draw(&self, ch: char) -> bool {
        self.glyph(ch).is_some()
    }

    /// How wide a string is, in pixels, laid out at this size.
    ///
    /// The same walk the drawing does, so a measurement and a drawing cannot
    /// disagree about where text ends — which is what centring, right
    /// alignment and every hit test depend on.
    #[must_use]
    pub fn width_of(&self, text: &str) -> f32 {
        text.chars().map(|ch| self.advance_of(ch)).sum()
    }

    /// The longest prefix of `text` that fits in `room`, and whether anything
    /// was left over.
    ///
    /// Returns the prefix with an ellipsis already appended when it had to cut,
    /// because a label that has been shortened must say so — the alternative is
    /// a truncated word that reads as a different, shorter fact (A1).
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

    /// Breaks `text` into lines that each fit in `room`, at word boundaries.
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

/// Which cut of a face is wanted. Two, because an interface needs emphasis and
/// does not need seven weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    /// Body text.
    Regular,
    /// Headings, figures, the word on a button.
    Bold,
}

/// The faces MCF found, and everything baked from them so far.
#[derive(Debug)]
pub struct Text {
    regular: Face,
    bold: Option<Face>,
    /// Keyed by weight and by size in tenths of a pixel, so that a size is
    /// rasterised once however many times it is asked for.
    baked: BTreeMap<(Weight, u32), Atlas>,
}

impl Text {
    /// Finds a face on this machine and prepares to draw with it.
    ///
    /// # Errors
    ///
    /// Returns a sentence naming the directories searched when there is no
    /// usable font here. The window then says that instead of drawing —
    /// falling back to the bitmap font would be falling back to exactly the
    /// thing this module was written to replace, and doing it quietly (A2).
    pub fn found() -> Result<Self, String> {
        let regular = discover(Weight::Regular)?;
        let bold = discover(Weight::Bold).ok();
        Ok(Self {
            regular,
            bold,
            baked: BTreeMap::new(),
        })
    }

    /// Builds from faces already in hand. For tests, and for a caller that
    /// knows better than the search which file it wants.
    #[must_use]
    pub fn from_faces(regular: Face, bold: Option<Face>) -> Self {
        Self {
            regular,
            bold,
            baked: BTreeMap::new(),
        }
    }

    /// The file the body text is drawn from.
    #[must_use]
    pub fn source(&self) -> &Path {
        &self.regular.source
    }

    /// Whether a real bold cut was found, or emphasis is being synthesised.
    #[must_use]
    pub fn has_bold(&self) -> bool {
        self.bold.is_some()
    }

    /// The atlas for a weight and size, rasterising it if this is the first
    /// time it has been asked for.
    ///
    /// # Errors
    ///
    /// Returns a sentence if the font cannot be rasterised at that size.
    pub fn at(&mut self, weight: Weight, size: f32) -> Result<&Atlas, String> {
        let key = (weight, tenths(size));
        if !self.baked.contains_key(&key) {
            let face = match weight {
                Weight::Bold => self.bold.as_ref().unwrap_or(&self.regular),
                Weight::Regular => &self.regular,
            };
            let atlas = bake(face, size)?;
            let _replaced = self.baked.insert(key, atlas);
        }
        self.baked
            .get(&key)
            .ok_or_else(|| "a size was baked and then was not there".to_owned())
    }
}

/// A size as a whole number of tenths of a pixel, for use as a map key —
/// floats do not make keys, and a tenth of a pixel is finer than any
/// difference that can be seen.
fn tenths(size: f32) -> u32 {
    let scaled = size * 10.0;
    if scaled.is_finite() && scaled > 0.0 {
        // `as` saturates at the bounds and cannot trap here: the value is
        // finite and positive, and a font size is nowhere near u32's ceiling.
        scaled.round() as u32
    } else {
        0
    }
}

/// Rasterises one face at one size, growing the atlas until everything fits.
///
/// The sizes are tried in order rather than computed. A computed bound would
/// have to model shelf packing's waste, and getting that wrong shows up as
/// text with holes in it; asking the packer is exact.
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

        // SAFETY: every pointer below is to a live allocation whose length is
        // passed alongside it, and the C side writes within those lengths —
        // `coverage` is `side × side` bytes and `glyphs` has one entry per
        // codepoint. The call does not retain any of them.
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

/// Vertical metrics for a face at a size.
fn metrics(face: &Face, size: f32) -> Result<(f32, f32, f32), String> {
    let (mut ascent, mut descent, mut line_gap) = (0.0_f32, 0.0_f32, 0.0_f32);
    // SAFETY: the bytes outlive the call and the three outputs are live locals.
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

/// The size of the C struct, for the test that checks the mirror.
#[must_use]
pub fn glyph_bytes_in_c() -> usize {
    // SAFETY: a call with no arguments returning a constant.
    let bytes = unsafe { c::mcf_glyph_bytes() };
    usize::try_from(bytes).unwrap_or(0)
}

/// Families to look for, best first.
///
/// Ordered by how a modern interface should look and then by how likely the
/// file is to be there, which is why it ends with faces nobody would choose on
/// looks alone: a window that opens in Liberation Sans is a window that opens.
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

/// Where fonts live, on each of the systems MCF runs on.
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
            // Where a Flatpak sees the host's fonts: the system's, the
            // person's, and /usr/local's, in that order.
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

/// Looks for the best available face of a weight.
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

/// Depth-first search for `<stem>.ttf` or `<stem>.otf`, case-insensitively.
///
/// Bounded, because a font directory is somebody else's tree and a symlink
/// loop in it must not be a hang (A2).
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
    /// A face from bytes already in hand.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>, source: PathBuf) -> Self {
        Self { bytes, source }
    }
}

#[cfg(not(have_font))]
mod c {
    //! No C compiler was found when this was built, so there is no text stack.
    //! Every entry point says so rather than drawing something else.
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
