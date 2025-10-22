#![allow(missing_docs)]

#[cfg(feature = "std")]
use std::collections::VecDeque;

#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mp4FileDemuxerAction {
    Read { size: usize, position: Option<u64> },
}

#[derive(Debug, Default)]
pub struct Mp4FileDemuxer {
    action_queue: VecDeque<Mp4FileDemuxerAction>,
}

impl Mp4FileDemuxer {
    pub fn new() -> Self {
        Self {
            action_queue: VecDeque::new(),
        }
    }

    pub fn next_action(&mut self) -> Option<Mp4FileDemuxerAction> {
        self.action_queue.pop_front()
    }
}
