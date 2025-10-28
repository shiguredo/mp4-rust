//! ../../../src/demux.rs の C API を定義するためのモジュール
use crate::{basic_types::Mp4TrackKind, error::Mp4Error};
use std::ffi::CString;

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

pub struct Mp4FileDemuxer {
    inner: shiguredo_mp4::demux::Mp4FileDemuxer,
    last_error_string: Option<CString>,
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_demuxer_new() -> *mut Mp4FileDemuxer {
    let demuxer = Mp4FileDemuxer {
        inner: shiguredo_mp4::demux::Mp4FileDemuxer::new(),
        last_error_string: None,
    };
    Box::into_raw(Box::new(demuxer))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_free(demuxer: *mut Mp4FileDemuxer) {
    if !demuxer.is_null() {
        let _ = unsafe { Box::from_raw(demuxer) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_get_last_error(
    demuxer: *const Mp4FileDemuxer,
) -> *const u8 {
    if demuxer.is_null() {
        return core::ptr::null();
    }

    let demuxer = unsafe { &*demuxer };
    let Some(e) = &demuxer.last_error_string else {
        return core::ptr::null();
    };
    e.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_get_required_input(
    demuxer: *mut Mp4FileDemuxer,
    out_required_input_position: *mut u64,
    out_required_input_size: *mut u32,
) -> Mp4Error {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_handle_input(
    demuxer: *mut Mp4FileDemuxer,
    input_position: u64,
    input_data: *const u8,
    input_data_size: u32,
) -> Mp4Error {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_get_tracks(
    demuxer: *mut Mp4FileDemuxer,
    out_tracks: *mut *const Mp4TrackInfo,
    out_track_count: *mut u32,
) -> Mp4Error {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_next_sample(
    demuxer: *mut Mp4FileDemuxer,
    out_sample: *mut Mp4Sample,
) -> Mp4Error {
    todo!()
}
