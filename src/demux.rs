#![allow(missing_docs)]

use crate::{Result, boxes::SampleEntry};

#[derive(Debug, Clone)]
pub struct Sample<'a> {
    pub track_id: u32,
    pub sample_entry: SampleEntry,
    pub timestamp: u64,
    pub duration: u32,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub enum MaybeSample<'a> {
    Sample(Sample<'a>),
    EndOfFile,
    ActionRequired(Mp4FileDemuxerAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mp4FileDemuxerAction {
    Read { size: usize, position: Option<u64> },
}

#[derive(Debug, Default)]
pub struct Mp4FileDemuxer {}

impl Mp4FileDemuxer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn next_sample(&mut self) -> Result<MaybeSample<'_>> {
        todo!()
    }
}
