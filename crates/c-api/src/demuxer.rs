//! ../../../src/demuxer.rs の C API を定義するためのモジュール
use shiguredo_mp4::{
    TrackKind,
    demux::{DemuxError, Input, Mp4FileDemuxer, RequiredInput, Sample, TrackInfo},
};

#[repr(C)]
pub enum Mp4DemuxError {
    Ok = 0,
    DecodeError = 1,
    SampleTableError = 2,
    InputRequired = 3,
}

impl From<DemuxError> for Mp4DemuxError {
    fn from(e: DemuxError) -> Self {
        match e {
            DemuxError::DecodeError(_) => Self::DecodeError,
            DemuxError::SampleTableError(_) => Self::SampleTableError,
            DemuxError::RequiredInput(_) => Self::RequiredInput,
        }
    }
}
