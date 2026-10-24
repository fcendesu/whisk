use media_spike::{
    capture::VideoFrame,
    render::{FrameQueue, current_frame, enqueue_latest},
};
use std::sync::Arc;
#[test]
fn slow_viewer_does_not_accumulate_frames() {
    let mut queue = FrameQueue::default();
    assert!(current_frame(&queue).is_none());
    for sequence in 0..180 {
        enqueue_latest(
            &mut queue,
            Arc::new(VideoFrame {
                width: 1,
                height: 1,
                bgra: vec![sequence as u8, 0, 0, 255].into(),
            }),
        );
        assert!(
            queue.len() <= 2,
            "slow viewer queue grew to {}",
            queue.len()
        );
    }
    assert_eq!(current_frame(&queue).unwrap().bgra[0], 179);
}
