//! ../../../src/demux.rs の C API を定義するためのモジュール
use std::ffi::{CString, c_char};

use shiguredo_mp4::BaseBox;

use crate::{
    basic_types::Mp4TrackKind,
    boxes::{Mp4SampleEntry, Mp4SampleEntryOwned},
    error::Mp4Error,
};

#[repr(C)]
pub struct Mp4DemuxTrackInfo {
    pub track_id: u32,
    pub kind: Mp4TrackKind,
    pub duration: u64,
    pub timescale: u32,
}

impl From<shiguredo_mp4::demux::TrackInfo> for Mp4DemuxTrackInfo {
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
pub struct Mp4DemuxSample {
    pub track: *const Mp4DemuxTrackInfo,
    pub sample_entry: *const Mp4SampleEntry,
    pub keyframe: bool,
    pub timestamp: u64,
    pub duration: u32,
    pub data_offset: u64,
    pub data_size: usize,
}

impl Mp4DemuxSample {
    pub fn new(
        sample: shiguredo_mp4::demux::Sample<'_>,
        track: &Mp4DemuxTrackInfo,
        sample_entry: &Box<Mp4SampleEntry>,
    ) -> Self {
        Self {
            track,
            sample_entry: &**sample_entry, // 途中でアドレスが変わらないように `Box` の参照先を渡す
            keyframe: sample.keyframe,
            timestamp: sample.timescaled_timestamp,
            duration: sample.timescaled_duration,
            data_offset: sample.data_offset,
            data_size: sample.data_size,
        }
    }
}

/// cbindgen:no-export
pub struct Mp4FileDemuxer {
    inner: shiguredo_mp4::demux::Mp4FileDemuxer,
    tracks: Vec<Mp4DemuxTrackInfo>, // NOTE: sample_entries とは異なりサイズが途中で変わらないので `Box` は不要
    sample_entries: Vec<(
        shiguredo_mp4::boxes::SampleEntry,
        Mp4SampleEntryOwned,
        Box<Mp4SampleEntry>, // アドレスが途中で変わらないように `Box` でラップする
    )>,
    last_error_string: Option<CString>,
}

impl Mp4FileDemuxer {
    fn set_last_error(&mut self, message: &str) {
        self.last_error_string = CString::new(message).ok();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_demuxer_new() -> *mut Mp4FileDemuxer {
    let demuxer = Mp4FileDemuxer {
        inner: shiguredo_mp4::demux::Mp4FileDemuxer::new(),
        tracks: Vec::new(),
        sample_entries: Vec::new(),
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
) -> *const c_char {
    if demuxer.is_null() {
        return c"Invalid demuxer: null pointer".as_ptr();
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
    out_required_input_size: *mut i32,
) -> Mp4Error {
    if demuxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let demuxer = unsafe { &mut *demuxer };

    if out_required_input_position.is_null() {
        demuxer.set_last_error(
            "[mp4_file_demuxer_get_required_input] out_required_input_position is null",
        );
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    if out_required_input_size.is_null() {
        demuxer.set_last_error(
            "[mp4_file_demuxer_get_required_input] out_required_input_size is null",
        );
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    unsafe {
        if let Some(required) = demuxer.inner.required_input() {
            *out_required_input_position = required.position;
            *out_required_input_size = required.size.map(|n| n as i32).unwrap_or(-1);
        } else {
            *out_required_input_position = 0;
            *out_required_input_size = 0;
        }
    }

    Mp4Error::MP4_ERROR_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_handle_input(
    demuxer: *mut Mp4FileDemuxer,
    input_position: u64,
    input_data: *const u8,
    input_data_size: u32,
) -> Mp4Error {
    if demuxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let demuxer = unsafe { &mut *demuxer };

    if input_data.is_null() {
        demuxer.set_last_error("[mp4_file_demuxer_handle_input] input_data is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    let input_data = unsafe { std::slice::from_raw_parts(input_data, input_data_size as usize) };
    let input = shiguredo_mp4::demux::Input {
        position: input_position,
        data: input_data,
    };
    demuxer.inner.handle_input(input);

    Mp4Error::MP4_ERROR_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_get_tracks(
    demuxer: *mut Mp4FileDemuxer,
    out_tracks: *mut *const Mp4DemuxTrackInfo,
    out_track_count: *mut u32,
) -> Mp4Error {
    if demuxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let demuxer = unsafe { &mut *demuxer };

    if out_tracks.is_null() {
        demuxer.set_last_error("[mp4_file_demuxer_get_tracks] out_tracks is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    if out_track_count.is_null() {
        demuxer.set_last_error("[mp4_file_demuxer_get_tracks] out_track_count is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    match demuxer.inner.tracks() {
        Ok(tracks) => {
            demuxer.tracks = tracks.iter().map(|t| t.clone().into()).collect();
            unsafe {
                *out_tracks = demuxer.tracks.as_ptr();
                *out_track_count = demuxer.tracks.len() as u32;
            }
            Mp4Error::MP4_ERROR_OK
        }
        Err(e) => {
            demuxer.set_last_error(&format!("[mp4_file_demuxer_get_tracks] {e}"));
            e.into()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_next_sample(
    demuxer: *mut Mp4FileDemuxer,
    out_sample: *mut Mp4DemuxSample,
) -> Mp4Error {
    if demuxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let demuxer = unsafe { &mut *demuxer };

    if out_sample.is_null() {
        demuxer.set_last_error("[mp4_file_demuxer_next_sample] out_sample is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    match demuxer.inner.next_sample() {
        Ok(Some(sample)) => {
            let Some(track_info) = demuxer
                .tracks
                .iter()
                .find(|t| t.track_id == sample.track.track_id)
            else {
                demuxer.set_last_error(
                    "[mp4_file_demuxer_next_sample] track info not found for sample",
                );
                return Mp4Error::MP4_ERROR_INVALID_STATE;
            };

            let sample_entry_box_type = sample.sample_entry.box_type();
            let sample_entry = if let Some(entry) = demuxer
                .sample_entries
                .iter()
                .find_map(|entry| (entry.0 == *sample.sample_entry).then_some(&entry.2))
            {
                entry
            } else {
                let Some(entry_owned) = Mp4SampleEntryOwned::new(sample.sample_entry.clone())
                else {
                    demuxer.set_last_error(&format!(
                        "[mp4_file_demuxer_next_sample] Unsupported sample entry box type: {sample_entry_box_type}",
                    ));
                    return Mp4Error::MP4_ERROR_UNSUPPORTED;
                };
                let entry = Box::new(entry_owned.to_mp4_sample_entry());
                demuxer
                    .sample_entries
                    .push((sample.sample_entry.clone(), entry_owned, entry));
                demuxer
                    .sample_entries
                    .last()
                    .map(|entry| &entry.2)
                    .expect("infallible")
            };

            unsafe {
                *out_sample = Mp4DemuxSample::new(sample, track_info, sample_entry);
            }

            Mp4Error::MP4_ERROR_OK
        }
        Ok(None) => Mp4Error::MP4_ERROR_NO_MORE_SAMPLES,
        Err(e) => {
            demuxer.set_last_error(&format!("[mp4_file_demuxer_next_sample] {e}"));
            e.into()
        }
    }
}
