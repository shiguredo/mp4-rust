//! ../../../src/demux.rs の C API を定義するためのモジュール
use crate::{basic_types::Mp4TrackKind, error::Mp4Error};

#[repr(C)]
pub struct Mp4TrackInfo {
    pub track_id: u32,
    pub kind: Mp4TrackKind,
    pub duration: u64,
    pub timescale: u32,
}

impl From<shiguredo_mp4::demux::TrackInfo> for Mp4TrackInfo {
    fn from(track_info: shiguredo_mp4::demux::TrackInfo) -> Self {
        Self {
            track_id: track_info.track_id,
            kind: track_info.kind.into(),
            duration: track_info.timescaled_duration,
            timescale: track_info.timescale.get(),
        }
    }
}

#[repr(C)]
pub struct Mp4Sample {
    pub track: *const Mp4TrackInfo,
    // TODO: sample_entry,
    pub keyframe: bool,
    pub timestamp: u64,
    pub duration: u32,
    pub data_offset: u64,
    pub data_size: usize,
}

impl Mp4Sample {
    pub fn new(sample: shiguredo_mp4::demux::Sample<'_>, track: &Mp4TrackInfo) -> Self {
        Self {
            track,
            keyframe: sample.keyframe,
            timestamp: sample.timescaled_timestamp,
            duration: sample.timescaled_duration,
            data_offset: sample.data_offset,
            data_size: sample.data_size,
        }
    }
}
