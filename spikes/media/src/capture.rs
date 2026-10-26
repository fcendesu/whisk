//! Native screen capture runs on a worker, never the UI thread.
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::watch;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureError {
    PermissionDenied,
    UnsupportedPlatform,
    SourceGone,
    Backend,
    InvalidFrame,
}
impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PermissionDenied => "Screen recording access is not granted. Enable it in System Settings > Privacy & Security > Screen & System Audio Recording, then retry.",
            Self::UnsupportedPlatform => "Native capture is not implemented on this platform yet.",
            Self::SourceGone => "The selected screen or window is no longer available. Refresh sources.",
            Self::Backend => "Native capture failed. Stop and retry the selected source.",
            Self::InvalidFrame => "The native source returned an invalid frame.",
        })
    }
}
impl std::error::Error for CaptureError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum SourceId {
    Display(u32),
    Window(u32),
}
#[derive(Debug, Clone, Serialize)]
pub struct CaptureSource {
    pub id: SourceId,
    pub label: String,
}
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub bgra: Arc<[u8]>,
}
#[derive(Debug, Clone)]
pub enum FrameEvent {
    Pending,
    Frame(Arc<VideoFrame>),
    Error(CaptureError),
    Finished(String),
}

#[derive(Default)]
pub struct CaptureCounters {
    callbacks: std::sync::atomic::AtomicU64,
    complete: std::sync::atomic::AtomicU64,
    copied: std::sync::atomic::AtomicU64,
    bytes: std::sync::atomic::AtomicU64,
    size: std::sync::atomic::AtomicU64,
}
#[derive(Serialize)]
pub struct CaptureSnapshot {
    pub screen_callbacks: u64,
    pub complete_callbacks: u64,
    pub copied_frames: u64,
    pub bgra_bytes_copied: u64,
    pub lifetime_ms: u64,
    pub native_queue_depth_requested: usize,
    pub last_captured_size: [u32; 2],
    pub latest_frame_capacity: usize,
    pub capture_drop_count: Option<u64>,
}
impl CaptureCounters {
    pub fn callback(&self, complete: bool) {
        use std::sync::atomic::Ordering::Relaxed;
        self.callbacks.fetch_add(1, Relaxed);
        if complete {
            self.complete.fetch_add(1, Relaxed);
        }
    }
    pub fn copied(&self, width: u32, height: u32, bytes: usize) {
        self.size.store(
            ((width as u64) << 32) | height as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        use std::sync::atomic::Ordering::Relaxed;
        self.copied.fetch_add(1, Relaxed);
        self.bytes.fetch_add(bytes as u64, Relaxed);
    }
    pub fn snapshot(&self, lifetime_ms: u64) -> CaptureSnapshot {
        use std::sync::atomic::Ordering::Relaxed;
        CaptureSnapshot {
            screen_callbacks: self.callbacks.load(Relaxed),
            complete_callbacks: self.complete.load(Relaxed),
            copied_frames: self.copied.load(Relaxed),
            bgra_bytes_copied: self.bytes.load(Relaxed),
            lifetime_ms,
            native_queue_depth_requested: 2,
            last_captured_size: [
                (self.size.load(Relaxed) >> 32) as u32,
                self.size.load(Relaxed) as u32,
            ],
            latest_frame_capacity: 1,
            capture_drop_count: None,
        }
    }
}
pub struct FrameStream {
    pub counters: Arc<CaptureCounters>,
    pub started: std::time::Instant,
    pub frames: watch::Receiver<FrameEvent>,
    #[cfg(target_os = "macos")]
    native: screencapturekit::stream::sc_stream::SCStream,
}
impl FrameStream {
    /// Stop on the same worker that owns the native stream.
    pub fn stop(&self) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
            self.native.stop_capture().map_err(macos::map_error)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::UnsupportedPlatform)
        }
    }
}

/// Platform boundary: permission must be checked before discovery and start.
pub trait CaptureBackend {
    fn permission(&self) -> Result<(), CaptureError>;
    fn sources(&self) -> Result<Vec<CaptureSource>, CaptureError>;
    fn start(&self, id: SourceId) -> Result<FrameStream, CaptureError>;
}
pub struct CaptureService<B>(pub B);
impl<B: CaptureBackend> CaptureService<B> {
    pub fn available_sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
        self.0.permission()?;
        self.0.sources()
    }
    pub fn open_source(&self, id: SourceId) -> Result<FrameStream, CaptureError> {
        self.0.permission()?;
        self.0.start(id)
    }
}

pub struct NativeCapture;
impl CaptureBackend for NativeCapture {
    fn permission(&self) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
            macos::permission()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::UnsupportedPlatform)
        }
    }
    fn sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
        #[cfg(target_os = "macos")]
        {
            macos::sources()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::UnsupportedPlatform)
        }
    }
    fn start(&self, _id: SourceId) -> Result<FrameStream, CaptureError> {
        #[cfg(target_os = "macos")]
        {
            macos::start(_id)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::UnsupportedPlatform)
        }
    }
}
pub fn available_sources() -> Result<Vec<CaptureSource>, CaptureError> {
    CaptureService(NativeCapture).available_sources()
}
pub fn open_source(id: SourceId) -> Result<FrameStream, CaptureError> {
    CaptureService(NativeCapture).open_source(id)
}

/// CPU copy for the spike. Native rows may contain padding; never flatten it.
pub fn copy_bgra_frame(
    width: u32,
    height: u32,
    stride: usize,
    data: &[u8],
) -> Result<VideoFrame, CaptureError> {
    let row = (width as usize)
        .checked_mul(4)
        .ok_or(CaptureError::InvalidFrame)?;
    let required = stride
        .checked_mul(height as usize)
        .ok_or(CaptureError::InvalidFrame)?;
    let packed = row
        .checked_mul(height as usize)
        .ok_or(CaptureError::InvalidFrame)?;
    if width == 0
        || height == 0
        || stride < row
        || required > data.len()
        || packed > 64 * 1024 * 1024
    {
        return Err(CaptureError::InvalidFrame);
    }
    let mut bytes = Vec::with_capacity(packed);
    for y in 0..height as usize {
        bytes.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    Ok(VideoFrame {
        width,
        height,
        bgra: bytes.into(),
    })
}

#[derive(Clone)]
pub struct FrameSender(watch::Sender<FrameEvent>);
pub fn frame_channel() -> (FrameSender, watch::Receiver<FrameEvent>) {
    let (sender, receiver) = watch::channel(FrameEvent::Pending);
    (FrameSender(sender), receiver)
}
impl FrameSender {
    pub fn send(&self, event: FrameEvent) {
        self.0.send_if_modified(|current| {
            if matches!(current, FrameEvent::Error(_) | FrameEvent::Finished(_)) {
                return false;
            }
            *current = event;
            true
        });
    }
}

pub fn next_frame_event(receiver: &mut watch::Receiver<FrameEvent>) -> Option<FrameEvent> {
    match receiver.has_changed() {
        Ok(true) => Some(receiver.borrow_and_update().clone()),
        Ok(false) => None,
        Err(_) => match receiver.borrow_and_update().clone() {
            terminal @ (FrameEvent::Finished(_) | FrameEvent::Error(_)) => Some(terminal),
            _ => Some(FrameEvent::Error(CaptureError::Backend)),
        },
    }
}

/// Experimental profile; original default API remains720p30. Permission preflight precedes start.
pub fn open_source_profile(
    id: SourceId,
    run: crate::capture_benchmark::CaptureRun,
) -> Result<FrameStream, CaptureError> {
    NativeCapture.permission()?;
    #[cfg(target_os = "macos")]
    {
        macos::start_profile(id, run)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (id, run);
        Err(CaptureError::UnsupportedPlatform)
    }
}

pub fn available_owned_test_sources() -> Result<Vec<CaptureSource>, CaptureError> {
    NativeCapture.permission()?;
    #[cfg(target_os = "macos")]
    {
        macos::owned_sources()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(CaptureError::UnsupportedPlatform)
    }
}
pub fn open_owned_test_source(
    id: SourceId,
    run: crate::capture_benchmark::CaptureRun,
) -> Result<FrameStream, CaptureError> {
    NativeCapture.permission()?;
    #[cfg(target_os = "macos")]
    {
        macos::start_owned_profile(id, run)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (id, run);
        Err(CaptureError::UnsupportedPlatform)
    }
}
