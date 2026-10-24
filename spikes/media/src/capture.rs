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
}

pub struct FrameStream {
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
            if matches!(current, FrameEvent::Error(_)) {
                return false;
            }
            *current = event;
            true
        });
    }
}
