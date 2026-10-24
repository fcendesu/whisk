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

/// Monotonic application receive times for audible decoded PCM. This is not
/// speaker playback timing or a packet-loss measure.
#[derive(Default)]
pub struct AudioContinuity {
    last_ms: u64,
    max_gap_ms: u64,
    frames: [u64; 2],
    invalid: bool,
}
pub struct AudioContinuityReport {
    pub audible_frames: [u64; 2],
    pub max_gap_ms: u64,
    pub continuous: bool,
}
impl AudioContinuity {
    pub fn observe(&mut self, second_phase: bool, elapsed_ms: u64) {
        if elapsed_ms < self.last_ms {
            self.invalid = true;
        }
        self.max_gap_ms = self.max_gap_ms.max(elapsed_ms.saturating_sub(self.last_ms));
        self.last_ms = elapsed_ms;
        self.frames[usize::from(second_phase)] += 1;
    }
    pub fn finish(mut self, duration_ms: u64) -> AudioContinuityReport {
        if duration_ms < self.last_ms {
            self.invalid = true;
        }
        self.max_gap_ms = self
            .max_gap_ms
            .max(duration_ms.saturating_sub(self.last_ms));
        AudioContinuityReport {
            audible_frames: self.frames,
            max_gap_ms: self.max_gap_ms,
            continuous: !self.invalid
                && self.frames.iter().all(|count| *count > 0)
                && self.max_gap_ms < 250,
        }
    }
}

/// Nearest-rank p50, p95 and maximum for validated application frame ages.
pub fn latency_percentiles(samples: &mut [u32]) -> Option<[u32; 3]> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_unstable();
    let n = samples.len();
    Some([
        samples[(n * 50).div_ceil(100) - 1],
        samples[(n * 95).div_ceil(100) - 1],
        samples[n - 1],
    ])
}
