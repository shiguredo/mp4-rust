#![allow(missing_docs)]

#[cfg(feature = "std")]
use std::collections::VecDeque;

#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;

use crate::{Result, boxes::SampleEntry};

#[derive(Debug, Clone)]
pub struct Sample {
    pub track_id: u32,
    pub sample_entry: SampleEntry,
    pub timestamp: u64,
    pub duration: u32,
    pub data: Vec<u8>,
}

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

impl Iterator for Mp4FileDemuxer {
    type Item = Result<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
