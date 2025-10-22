#![allow(missing_docs)]

use core::time::Duration;

use crate::{
    Error,
    aux::SampleTableAccessor,
    boxes::{MoovBox, SampleEntry, StblBox},
};

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
pub struct TrackState {
    table: SampleTableAccessor<StblBox>,
}

#[derive(Debug)]
pub enum DemuxError {
    DecodeError(Error),
    NeedInput { position: u64, size: usize },
}

#[derive(Debug)]
enum Phase {
    ReadFtypBox,
    Initialized,
}

#[derive(Debug)]
pub struct Mp4FileDemuxer {
    phase: Phase,
    tracks: Vec<TrackState>,
}

impl Mp4FileDemuxer {
    pub fn new() -> Self {
        Self {
            phase: Phase::ReadFtypBox,
            tracks: Vec::new(),
        }
    }

    pub fn handle_input(&mut self, _input: &Input) -> Result<(), DemuxError> {
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

    fn next_sample(&mut self) -> Result<Option<Sample>, DemuxError> {
        self.initialize_if_need()?;
        todo!()
    }

    fn initialize_if_need(&mut self) -> Result<(), DemuxError> {
        match self.phase {
            Phase::ReadFtypBox => todo!(),
            Phase::Initialized => Ok(()),
        }
    }
}

impl Iterator for Mp4FileDemuxer {
    type Item = Result<Sample, DemuxError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_sample().transpose()
    }
}
