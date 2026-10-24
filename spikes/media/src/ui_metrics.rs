//! CPU-side GPUI preparation counters; never GPU or display presentation proof.
use serde::Serialize;
use std::path::{Path, PathBuf};
pub struct UiFrameMetrics {
    mode: &'static str,
    frames: u64,
    bytes: u64,
    max_queue: usize,
    trees: u64,
    preparations: Vec<u32>,
}
#[derive(Serialize)]
pub struct UiFrameReport {
    pub mode: &'static str,
    pub window_ms: u64,
    pub cpu_prepared_frames: u64,
    pub ui_bgra_bytes_copied: u64,
    pub max_queue_depth: usize,
    pub view_tree_builds: u64,
    pub image_preparation_p95_us: Option<u32>,
    pub observed_preparation_fps: Option<f64>,
    pub gpu_presented_frames: Option<u64>,
    pub quality_certified: bool,
}
impl UiFrameMetrics {
    pub fn new(mode: &'static str) -> Self {
        Self {
            mode,
            frames: 0,
            bytes: 0,
            max_queue: 0,
            trees: 0,
            preparations: Vec::new(),
        }
    }
    pub fn prepared(&mut self, bytes: usize, preparation_us: u32, queue_depth: usize) {
        self.frames = self.frames.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes as u64);
        self.max_queue = self.max_queue.max(queue_depth);
        if self.preparations.len() < 100_000 {
            self.preparations.push(preparation_us);
        }
    }
    pub fn tree_built(&mut self) {
        self.trees = self.trees.saturating_add(1);
    }
    pub fn finish(mut self, window_ms: u64) -> UiFrameReport {
        let p95 =
            crate::benchmark::latency_percentiles(&mut self.preparations).map(|values| values[1]);
        UiFrameReport {
            mode: self.mode,
            window_ms,
            cpu_prepared_frames: self.frames,
            ui_bgra_bytes_copied: self.bytes,
            max_queue_depth: self.max_queue,
            view_tree_builds: self.trees,
            image_preparation_p95_us: p95,
            observed_preparation_fps: (window_ms > 0 && self.frames > 0)
                .then(|| self.frames as f64 * 1000.0 / window_ms as f64),
            gpu_presented_frames: None,
            quality_certified: false,
        }
    }
}
pub fn measurement_path(executable: &Path) -> PathBuf {
    if let Some(bundle) = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|ext| ext == "app"))
    {
        let mut name = bundle.file_stem().unwrap_or_default().to_os_string();
        name.push("-preview-metrics.json");
        return bundle.parent().unwrap_or_else(|| Path::new(".")).join(name);
    }
    executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("whisk-preview-metrics.json")
}
