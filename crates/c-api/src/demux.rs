//! ../../../src/demuxer.rs の C API を定義するためのモジュール
use shiguredo_mp4::{
    TrackKind,
    demux::{DemuxError, Input, Mp4FileDemuxer, RequiredInput, Sample, TrackInfo},
};

use crate::basic_types::Mp4TrackKind;

#[repr(C)]
pub enum Mp4DemuxError {
    Ok = 0,
    DecodeError = 1,
    SampleTableError = 2,
    InputRequired = 3,
    Unknown = 4,
}

impl From<DemuxError> for Mp4DemuxError {
    fn from(e: DemuxError) -> Self {
        match e {
            DemuxError::DecodeError(_) => Self::DecodeError,
            DemuxError::SampleTableError(_) => Self::SampleTableError,
            DemuxError::InputRequired(_) => Self::InputRequired,
            _ => Self::Unknown,
        }
    }
}

#[repr(C)]
pub struct Mp4TrackInfo {
    pub track_id: u32,
    pub kind: Mp4TrackKind,
    pub duration: u64,
    pub timescale: u32,
}

impl From<TrackInfo> for Mp4TrackInfo {
    fn from(track_info: TrackInfo) -> Self {
        Self {
            track_id: track_info.track_id,
            kind: track_info.kind.into(),
            duration: track_info.timescaled_duration,
            timescale: track_info.timescale.get(),
        }
    }
}
