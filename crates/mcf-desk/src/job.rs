pub use mcf_tui::job::{Job, refused_because};

#[must_use]
pub fn fraction(job: &Job) -> Option<f32> {
    let (arrived, total) = job.progress()?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "byte counts of a file, far inside f32's exact range at these magnitudes"
    )]
    Some((arrived as f32 / total as f32).clamp(0.0, 1.0))
}
