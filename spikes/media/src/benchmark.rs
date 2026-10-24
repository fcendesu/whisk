//! Counter validation only; this does not certify a codec or hardware profile.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FrameSample {
    pub elapsed_ms: Option<u64>,
    pub captured: Option<u64>,
    pub rendered: Option<u64>,
}
#[derive(Debug, PartialEq, Serialize)]
pub struct BenchmarkSummary {
    pub duration_ms: u64,
    pub captured_frames: u64,
    pub rendered_frames: u64,
    pub capture_fps: f64,
    pub render_fps: f64,
    pub dropped_frames: u64,
    pub meets_minimum_duration: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum MetricsError {
    MissingData,
    InvalidSamples,
}
pub fn summarize(samples: &[FrameSample]) -> Result<BenchmarkSummary, MetricsError> {
    if samples.is_empty() {
        return Err(MetricsError::MissingData);
    }
    let read = |sample: &FrameSample| -> Result<(u64, u64, u64), MetricsError> {
        Ok((
            sample.elapsed_ms.ok_or(MetricsError::MissingData)?,
            sample.captured.ok_or(MetricsError::MissingData)?,
            sample.rendered.ok_or(MetricsError::MissingData)?,
        ))
    };
    let first = read(&samples[0])?;
    let mut last = first;
    for sample in &samples[1..] {
        let current = read(sample)?;
        if current.0 < last.0 || current.1 < last.1 || current.2 < last.2 || current.2 > current.1 {
            return Err(MetricsError::InvalidSamples);
        }
        last = current;
    }
    let duration_ms = last.0 - first.0;
    let captured_frames = last.1 - first.1;
    let rendered_frames = last.2 - first.2;
    if samples.len() < 2
        || first.2 > first.1
        || duration_ms == 0
        || captured_frames == 0
        || rendered_frames > captured_frames
    {
        return Err(MetricsError::InvalidSamples);
    }
    Ok(BenchmarkSummary {
        duration_ms,
        captured_frames,
        rendered_frames,
        capture_fps: captured_frames as f64 * 1000. / duration_ms as f64,
        render_fps: rendered_frames as f64 * 1000. / duration_ms as f64,
        dropped_frames: captured_frames - rendered_frames,
        meets_minimum_duration: duration_ms >= 600_000,
    })
}
