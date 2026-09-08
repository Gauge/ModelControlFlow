use std::path::{Path, PathBuf};

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
