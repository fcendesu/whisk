use crate::capture::VideoFrame;
use std::{collections::VecDeque, sync::Arc};
#[derive(Default)]
pub struct FrameQueue(VecDeque<Arc<VideoFrame>>);
impl FrameQueue {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
pub fn enqueue_latest(queue: &mut FrameQueue, frame: Arc<VideoFrame>) {
    if queue.0.len() == 2 {
        queue.0.pop_front();
    }
    queue.0.push_back(frame);
}
pub fn current_frame(queue: &FrameQueue) -> Option<Arc<VideoFrame>> {
    queue.0.back().cloned()
}
