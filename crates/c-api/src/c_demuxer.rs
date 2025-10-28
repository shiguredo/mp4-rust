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
