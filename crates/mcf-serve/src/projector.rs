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

/// The projector beside a model, if one is there: in the model's own directory, or in the
/// one above it, which is where a publisher that keeps each quantization in a folder of its
/// own puts the one projector they all share.
#[must_use]
pub fn beside(model: &Path) -> Option<PathBuf> {
    let directory = model.parent()?;
    std::iter::once(directory)
        .chain(directory.parent())
        .find_map(|directory| in_directory(directory, model))
}

fn in_directory(directory: &Path, model: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let is_gguf = path
                .extension()
                .and_then(|held| held.to_str())
                .is_some_and(|held| held.eq_ignore_ascii_case("gguf"));
            is_gguf && mcf_hub::store::is_a_companion(path) && path != model
        })
        .collect();
    candidates.sort_by_key(|path| (mcf_hub::store::projector_preference(path), path.clone()));
    candidates
        .into_iter()
        .find(|path| declares_a_vision_encoder(path))
}
