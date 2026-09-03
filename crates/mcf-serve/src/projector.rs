//! The projector that turns pictures into what a model reads, and where it is.
//!
//! **One place, because two callers were about to have two answers.** The
//! probe found a model's projector to show it a picture; hosting needs the
//! same file to start the engine with it, so that a caller on the port can
//! send one. Two searches would agree until the day one was corrected
//! (B-072), so the probe and the host now ask here.

use std::path::{Path, PathBuf};

/// Whether this file says it is a vision projector.
///
/// **Read, not assumed from the name.** A projector declares `clip.*` — a
/// vision encoder, a projector type, the geometry of the patches it makes —
/// and that declaration is what makes it one. `mmproj` is a convention every
/// publisher happens to follow, and a convention is a thing that holds until
/// it does not; a file MCF believed because of its name would be a file MCF
/// had not read (A21).
///
/// From a bounded read of the front, because the directory sits there and a
/// projector is a gigabyte nobody needs in memory to answer this.
#[must_use]
pub fn declares_a_vision_encoder(path: &Path) -> bool {
    use std::io::Read as _;
    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [4_u64 << 20, 64 << 20] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        if std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .is_err()
        {
            return false;
        }
        if let Ok(file) = mcf_standin::gguf::parse(&prefix) {
            return file.get("clip.has_vision_encoder").is_some()
                || file.get("clip.has_audio_encoder").is_some()
                || file.get("clip.projector_type").is_some();
        }
        if take >= held {
            return false;
        }
    }
    false
}

/// The projector that belongs to this model, where its repository published one.
///
/// **Beside it, which is where hubs put it and not where MCF wants it.** A
/// projector is half of a multimodal artifact and travels as a separate file,
/// so a model acquired without it is a model that cannot be shown a picture —
/// through no fault of its own. MCF looks in the directory the weights landed
/// in, which is where `mcf pull` of the same repository would have put it.
///
/// The name narrows and the declaration decides: every publisher calls it
/// `mmproj`, so that is a cheap way to avoid reading the front of every model
/// in the directory — but what admits a file is `clip.*`, read out of it.
#[must_use]
pub fn beside(model: &Path) -> Option<PathBuf> {
    let directory = model.parent()?;
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let is_gguf = path
                .extension()
                .and_then(|held| held.to_str())
                .is_some_and(|held| held.eq_ignore_ascii_case("gguf"));
            let is_named_like_one = path
                .file_stem()
                .and_then(|held| held.to_str())
                .is_some_and(|held| held.to_ascii_lowercase().starts_with("mmproj"));
            is_gguf && is_named_like_one && path != model
        })
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .find(|path| declares_a_vision_encoder(path))
}
