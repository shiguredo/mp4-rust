#![allow(missing_docs)]

use core::time::Duration;

use crate::{Error, boxes::SampleEntry};

#[derive(Debug, Clone)]
pub enum TrackKind {
    Audio,
    Video,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_id: u32,
    pub kind: TrackKind,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub track_id: u32,
    pub sample_entry: Option<SampleEntry>,
    pub keyframe: bool,
    pub timestamp: Duration,
    pub duration: Duration,
    pub data_offset: u64,
    pub data_size: usize,
}

#[derive(Debug)]
pub struct Input<'a> {
    pub position: u64,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub enum DemuxError {
    DecodeError(Error),
    NeedInput { position: u64, size: usize },
}

#[derive(Debug, Default)]
pub struct Mp4FileDemuxer {}

impl Mp4FileDemuxer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_input(&mut self, _input: &Input) {
        todo!()
    }

    pub fn tracks(&self) -> Result<&[TrackInfo], DemuxError> {
        todo!()
    }

    pub fn seek(&mut self, _timestamp: Duration) -> Result<(), DemuxError> {
        todo!()
    }

    /// 指定のタイムスタンプ以下で、一番タイムスタンプが大きいキーフレームにシークする
    pub fn seek_to_keyframe(&mut self, _timestamp: Duration) -> Result<(), DemuxError> {
        todo!()
    }
}

impl Iterator for Mp4FileDemuxer {
    type Item = Result<Sample, DemuxError>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
