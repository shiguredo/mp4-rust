//! ../../../src/mux.rs の C API を定義するためのモジュール
use std::{
    ffi::{CString, c_char},
    time::Duration,
};

use crate::{basic_types::Mp4TrackKind, error::Mp4Error};

#[repr(C)]
pub struct Mp4MuxSample {
    pub track_kind: Mp4TrackKind,
    pub sample_entry: *const Mp4SampleEntry,
    pub keyframe: bool,
    pub duration_micros: u64,
    pub data_offset: u64,
    pub data_size: u32,
}

#[repr(C)]
pub struct Mp4SampleEntry {
    // This would contain the sample entry data
    // For now, we'll use an opaque representation
    _opaque: [u8; 0],
}

struct Output {
    offset: u64,
    data: Vec<u8>,
}

#[repr(C)]
pub struct Mp4FileMuxer {
    options: shiguredo_mp4::mux::Mp4FileMuxerOptions,
    inner: Option<shiguredo_mp4::mux::Mp4FileMuxer>,
    last_error_string: Option<CString>,
    output_list: Vec<Output>,
    next_output_index: usize,
}

impl Mp4FileMuxer {
    fn set_last_error(&mut self, message: &str) {
        self.last_error_string = CString::new(message).ok();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_estimate_maximum_moov_box_size(
    audio_sample_count: u32,
    video_sample_count: u32,
) -> u32 {
    shiguredo_mp4::mux::estimate_maximum_moov_box_size(&[
        audio_sample_count as usize,
        video_sample_count as usize,
    ]) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_muxer_new() -> *mut Mp4FileMuxer {
    let muxer = Mp4FileMuxer {
        options: shiguredo_mp4::mux::Mp4FileMuxerOptions::default(),
        inner: None,
        last_error_string: None,
        output_list: Vec::new(),
        next_output_index: 0,
    };
    Box::into_raw(Box::new(muxer))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_free(muxer: *mut Mp4FileMuxer) {
    if !muxer.is_null() {
        let _ = unsafe { Box::from_raw(muxer) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_get_last_error(
    muxer: *const Mp4FileMuxer,
) -> *const c_char {
    if muxer.is_null() {
        return c"Invalid muxer: null pointer".as_ptr();
    }

    let muxer = unsafe { &*muxer };
    let Some(e) = &muxer.last_error_string else {
        return core::ptr::null();
    };
    e.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_set_reserved_moov_box_size(
    muxer: *mut Mp4FileMuxer,
    size: u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }

    let muxer = unsafe { &mut *muxer };
    muxer.options.reserved_moov_box_size = size as usize;

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_set_creation_timestamp(
    muxer: *mut Mp4FileMuxer,
    timestamp_micros: u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }

    let muxer = unsafe { &mut *muxer };
    let timestamp = Duration::from_micros(timestamp_micros);
    muxer.options.creation_timestamp = timestamp;

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_initialize(muxer: *mut Mp4FileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &mut *muxer };

    if muxer.inner.is_some() {
        muxer.set_last_error("[mp4_file_muxer_initialize] Muxer has already been initialized");
        return Mp4Error::InvalidState;
    }

    match shiguredo_mp4::mux::Mp4FileMuxer::with_options(muxer.options.clone()) {
        Ok(inner) => {
            let initial = inner.initial_boxes_bytes();
            muxer.output_list.push(Output {
                offset: 0,
                data: initial.to_vec(),
            });
            muxer.inner = Some(inner);
            Mp4Error::Ok
        }
        Err(e) => {
            muxer.set_last_error(&format!(
                "[mp4_file_muxer_initialize] Failed to initialize muxer: {e}",
            ));
            e.into()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_next_output(
    muxer: *mut Mp4FileMuxer,
    out_output_offset: *mut u64,
    out_output_size: *mut u32,
    out_output_data: *mut *const u8,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &mut *muxer };

    if out_output_offset.is_null() {
        muxer.set_last_error("[mp4_file_muxer_next_output] out_output_offset is null");
        return Mp4Error::NullPointer;
    }
    if out_output_size.is_null() {
        muxer.set_last_error("[mp4_file_muxer_next_output] out_output_size is null");
        return Mp4Error::NullPointer;
    }
    if out_output_data.is_null() {
        muxer.set_last_error("[mp4_file_muxer_next_output] out_output_data is null");
        return Mp4Error::NullPointer;
    }

    if let Some(output) = muxer.output_list.get(muxer.next_output_index) {
        unsafe {
            *out_output_offset = output.offset;
            *out_output_size = output.data.len() as u32;
            *out_output_data = output.data.as_ptr();
        }
        muxer.next_output_index += 1;
    } else {
        unsafe {
            *out_output_offset = 0;
            *out_output_size = 0;
            *out_output_data = core::ptr::null_mut();
        }
    }

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_append_sample(
    muxer: *mut Mp4FileMuxer,
    sample: *const Mp4MuxSample,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &mut *muxer };

    if muxer.next_output_index < muxer.output_list.len() {
        // TODO: set last error
        return Mp4Error::OutputRequired;
    }

    if sample.is_null() {
        muxer.set_last_error("[mp4_file_muxer_append_sample] sample is null");
        return Mp4Error::NullPointer;
    }
    let sample = unsafe { &*sample };

    let duration = Duration::from_micros(sample.duration_micros);
    let sample_entry = if sample.sample_entry.is_null() {
        None
    } else {
        None // TODO: Implement conversion
    };

    let Some(inner) = &mut muxer.inner else {
        muxer.set_last_error("[mp4_file_muxer_append_sample] Muxer has not been initialized");
        return Mp4Error::InvalidState;
    };

    let sample = shiguredo_mp4::mux::Sample {
        track_kind: sample.track_kind.into(),
        sample_entry,
        keyframe: sample.keyframe,
        duration,
        data_offset: sample.data_offset,
        data_size: sample.data_size as usize,
    };

    if let Err(e) = inner.append_sample(&sample) {
        muxer.set_last_error(&format!(
            "[mp4_file_muxer_append_sample] Failed to append sample: {e}"
        ));
        e.into()
    } else {
        Mp4Error::Ok
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalize(muxer: *mut Mp4FileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &mut *muxer };

    if muxer.next_output_index < muxer.output_list.len() {
        // TODO: set last error
        return Mp4Error::OutputRequired;
    }

    let Some(inner) = &mut muxer.inner else {
        muxer.set_last_error("[mp4_file_muxer_finalize] Muxer has not been initialized");
        return Mp4Error::InvalidState;
    };

    if let Err(e) = inner.finalize() {
        muxer.set_last_error(&format!(
            "[mp4_file_muxer_finalize] Failed to finalize muxer: {e}"
        ));
        e.into()
    } else {
        Mp4Error::Ok
    }
}
