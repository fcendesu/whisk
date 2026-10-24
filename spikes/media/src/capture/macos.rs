use super::*;
use screencapturekit::{
    cm::{CMSampleBufferExt, SCFrameStatus},
    error::SCStreamErrorCode,
    prelude::*,
};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

pub(super) fn permission() -> Result<(), CaptureError> {
    // Preflight only: never trigger the OS permission prompt automatically.
    if unsafe { CGPreflightScreenCaptureAccess() } {
        Ok(())
    } else {
        Err(CaptureError::PermissionDenied)
    }
}

pub(super) fn map_error(error: SCError) -> CaptureError {
    if error.stream_error_code() == Some(SCStreamErrorCode::UserDeclined) || permission().is_err() {
        CaptureError::PermissionDenied
    } else {
        CaptureError::Backend
    }
}

pub(super) fn sources() -> Result<Vec<CaptureSource>, CaptureError> {
    let content = SCShareableContent::get().map_err(map_error)?;
    let mut sources = Vec::new();
    for display in content.displays() {
        sources.push(CaptureSource {
            id: SourceId::Display(display.display_id()),
            label: format!(
                "Display {} · {} × {}",
                display.display_id(),
                display.width(),
                display.height()
            ),
        });
    }
    for window in content.windows() {
        if let Some(title) = window.title().filter(|title| !title.trim().is_empty()) {
            sources.push(CaptureSource {
                id: SourceId::Window(window.window_id()),
                label: title,
            });
        }
    }
    Ok(sources)
}

pub(super) fn start(id: SourceId) -> Result<FrameStream, CaptureError> {
    let content = SCShareableContent::get().map_err(map_error)?;
    let filter = match id {
        SourceId::Display(id) => {
            let display = content
                .displays()
                .into_iter()
                .find(|d| d.display_id() == id)
                .ok_or(CaptureError::SourceGone)?;
            SCContentFilter::create()
                .with_display(&display)
                .with_excluding_windows(&[])
                .build()
        }
        SourceId::Window(id) => {
            let window = content
                .windows()
                .into_iter()
                .find(|w| w.window_id() == id)
                .ok_or(CaptureError::SourceGone)?;
            SCContentFilter::create().with_window(&window).build()
        }
    }
    .map_err(map_error)?;
    let config = SCStreamConfiguration::new()
        .with_width(1280)
        .with_height(720)
        .with_pixel_format(PixelFormat::BGRA)
        .with_minimum_frame_interval(&CMTime::new(1, 30))
        .with_queue_depth(2)
        .with_shows_cursor(true);
    let (sender, frames) = frame_channel();
    let errors = sender.clone();
    let delegate = ErrorHandler::new(move |error: SCError| {
        errors.send(FrameEvent::Error(map_error(error)));
    });
    let mut stream = SCStream::new_with_delegate(&filter, &config, delegate).map_err(map_error)?;
    stream
        .add_output_handler(
            move |sample: CMSampleBuffer, kind: SCStreamOutputType| {
                if kind != SCStreamOutputType::Screen
                    || sample.frame_status() != Some(SCFrameStatus::Complete)
                {
                    return;
                }
                let Some(buffer) = sample.pixel_buffer() else {
                    return;
                };
                let Ok(lock) = buffer.lock_read_only() else {
                    return;
                };
                // The read lock and retained sample keep this mapping alive during copy.
                let Some(data) = (unsafe { lock.as_slice() }) else {
                    return;
                };
                let frame = copy_bgra_frame(
                    lock.width() as u32,
                    lock.height() as u32,
                    lock.bytes_per_row(),
                    data,
                );
                match frame {
                    Ok(frame) => {
                        sender.send(FrameEvent::Frame(Arc::new(frame)));
                    }
                    Err(error) => {
                        sender.send(FrameEvent::Error(error));
                    }
                }
            },
            SCStreamOutputType::Screen,
        )
        .map_err(map_error)?;
    stream.start_capture().map_err(map_error)?;
    Ok(FrameStream {
        native: stream,
        frames,
    })
}
