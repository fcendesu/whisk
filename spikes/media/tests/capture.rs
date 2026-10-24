use media_spike::capture::{
    CaptureBackend, CaptureError, CaptureService, CaptureSource, FrameStream, SourceId,
    copy_bgra_frame,
};

struct Denied;
impl CaptureBackend for Denied {
    fn permission(&self) -> Result<(), CaptureError> {
        Err(CaptureError::PermissionDenied)
    }
    fn sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
        panic!("must not enumerate without permission")
    }
    fn start(&self, _: SourceId) -> Result<FrameStream, CaptureError> {
        panic!("must not create a phantom stream")
    }
}

#[test]
fn denied_capture_permission_is_safe() {
    let service = CaptureService(Denied);
    assert!(matches!(
        service.available_sources(),
        Err(CaptureError::PermissionDenied)
    ));
    assert!(matches!(
        service.open_source(SourceId::Display(1)),
        Err(CaptureError::PermissionDenied)
    ));
}

#[test]
fn frame_copy_respects_padded_native_rows() {
    let frame = copy_bgra_frame(
        1,
        2,
        8,
        &[1, 2, 3, 4, 99, 99, 99, 99, 5, 6, 7, 8, 99, 99, 99, 99],
    )
    .unwrap();
    assert_eq!((frame.width, frame.height), (1, 2));
    assert_eq!(frame.bgra.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn invalid_native_frame_dimensions_are_rejected() {
    for (width, height, stride, data) in [
        (0, 1, 4, &[0u8; 4][..]),
        (1, 0, 4, &[][..]),
        (2, 1, 4, &[0u8; 8][..]),
        (1, 2, 8, &[0u8; 8][..]),
        (u32::MAX, u32::MAX, usize::MAX, &[][..]),
    ] {
        assert!(matches!(
            copy_bgra_frame(width, height, stride, data),
            Err(CaptureError::InvalidFrame)
        ));
    }
}

#[test]
fn native_stop_error_cannot_be_hidden_by_a_late_frame() {
    use media_spike::capture::{FrameEvent, VideoFrame, frame_channel};
    use std::sync::Arc;
    let (sender, receiver) = frame_channel();
    sender.send(FrameEvent::Error(CaptureError::PermissionDenied));
    sender.send(FrameEvent::Frame(Arc::new(VideoFrame {
        width: 1,
        height: 1,
        bgra: vec![0; 4].into(),
    })));
    assert!(matches!(
        *receiver.borrow(),
        FrameEvent::Error(CaptureError::PermissionDenied)
    ));
}
