//! Work that takes longer than a frame: the console's job, and the one thing
//! the window draws from it that the console does not — a bar.

/// The job, as the console runs it.
pub use mcf_tui::job::{Job, refused_because};

/// How far along, between nothing and one, where that is knowable.
///
/// `None` rather than zero where it is not: a bar drawn at zero says the
/// work has not started, and *MCF cannot say how far along this is* is a
/// different thing (A7).
#[must_use]
pub fn fraction(job: &Job) -> Option<f32> {
    let (arrived, total) = job.progress()?;
    // Both are counts of bytes and neither is near f32's limits at any size
    // a model comes in.
    #[expect(
        clippy::cast_precision_loss,
        reason = "byte counts of a file, far inside f32's exact range at these magnitudes"
    )]
    Some((arrived as f32 / total as f32).clamp(0.0, 1.0))
}
