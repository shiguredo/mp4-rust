//! ../../../src/mux.rs の C API を定義するためのモジュール
use std::ffi::{CString, c_char};

use crate::error::Mp4Error;

#[repr(C)]
pub struct Mp4FileMuxer {
    options: shiguredo_mp4::mux::Mp4FileMuxerOptions,
    inner: Option<shiguredo_mp4::mux::Mp4FileMuxer>,
    last_error_string: Option<CString>,
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_muxer_new() -> *mut Mp4FileMuxer {
    let muxer = Mp4FileMuxer {
        options: shiguredo_mp4::mux::Mp4FileMuxerOptions::default(),
        inner: None,
        last_error_string: None,
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

/*
#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_muxer_new_with_reserved_moov_size(
    reserved_moov_box_size: usize,
) -> *mut Mp4FileMuxer {
    let options = shiguredo_mp4::mux::Mp4FileMuxerOptions {
        reserved_moov_box_size,
        ..Default::default()
    };
    match shiguredo_mp4::mux::Mp4FileMuxer::with_options(options) {
        Ok(inner) => {
            let muxer = Mp4FileMuxer {
                inner,
                last_error_string: None,
            };
            Box::into_raw(Box::new(muxer))
        }
        Err(_) => core::ptr::null_mut(),
    }
}



#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_initial_boxes_bytes(
    muxer: *const Mp4FileMuxer,
    out_bytes: *mut *const u8,
    out_size: *mut usize,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &*muxer };

    if out_bytes.is_null() || out_size.is_null() {
        return Mp4Error::NullPointer;
    }

    let bytes = muxer.inner.initial_boxes_bytes();
    unsafe {
        *out_bytes = bytes.as_ptr();
        *out_size = bytes.len();
    }

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_append_sample(
    muxer: *mut Mp4FileMuxer,
    track_kind: u32, // 0 = Audio, 1 = Video
    keyframe: bool,
    duration_us: u64,
    data_offset: u64,
    data_size: usize,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &mut *muxer };

    let track_kind = match track_kind {
        0 => shiguredo_mp4::TrackKind::Audio,
        1 => shiguredo_mp4::TrackKind::Video,
        _ => {
            muxer.last_error_string =
                CString::new("Invalid track_kind: must be 0 (Audio) or 1 (Video)").ok();
            return Mp4Error::InvalidInput;
        }
    };

    let sample = shiguredo_mp4::mux::Sample {
        track_kind,
        sample_entry: None,
        keyframe,
        duration: std::time::Duration::from_micros(duration_us),
        data_offset,
        data_size,
    };

    match muxer.inner.append_sample(&sample) {
        Ok(()) => Mp4Error::Ok,
        Err(e) => {
            muxer.last_error_string = CString::new(format!("{e}")).ok();
            e.into()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalize(muxer: *mut Mp4FileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &mut *muxer };

    match muxer.inner.finalize() {
        Ok(_) => Mp4Error::Ok,
        Err(e) => {
            muxer.last_error_string = CString::new(format!("{e}")).ok();
            e.into()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalized_boxes_moov_offset(
    muxer: *const Mp4FileMuxer,
    out_offset: *mut u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &*muxer };

    if out_offset.is_null() {
        return Mp4Error::NullPointer;
    }

    let Some(finalized) = muxer.inner.finalized_boxes() else {
        return Mp4Error::InvalidState;
    };

    unsafe {
        *out_offset = finalized.moov_box_offset;
    }

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalized_boxes_moov_bytes(
    muxer: *const Mp4FileMuxer,
    out_bytes: *mut *const u8,
    out_size: *mut usize,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &*muxer };

    if out_bytes.is_null() || out_size.is_null() {
        return Mp4Error::NullPointer;
    }

    let Some(finalized) = muxer.inner.finalized_boxes() else {
        return Mp4Error::InvalidState;
    };

    unsafe {
        *out_bytes = finalized.moov_box_bytes.as_ptr();
        *out_size = finalized.moov_box_bytes.len();
    }

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalized_boxes_mdat_offset(
    muxer: *const Mp4FileMuxer,
    out_offset: *mut u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &*muxer };

    if out_offset.is_null() {
        return Mp4Error::NullPointer;
    }

    let Some(finalized) = muxer.inner.finalized_boxes() else {
        return Mp4Error::InvalidState;
    };

    unsafe {
        *out_offset = finalized.mdat_box_offset;
    }

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalized_boxes_mdat_header_bytes(
    muxer: *const Mp4FileMuxer,
    out_bytes: *mut *const u8,
    out_size: *mut usize,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &*muxer };

    if out_bytes.is_null() || out_size.is_null() {
        return Mp4Error::NullPointer;
    }

    let Some(finalized) = muxer.inner.finalized_boxes() else {
        return Mp4Error::InvalidState;
    };

    unsafe {
        *out_bytes = finalized.mdat_box_header_bytes.as_ptr();
        *out_size = finalized.mdat_box_header_bytes.len();
    }

    Mp4Error::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_is_faststart_enabled(
    muxer: *const Mp4FileMuxer,
    out_enabled: *mut bool,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::NullPointer;
    }
    let muxer = unsafe { &*muxer };

    if out_enabled.is_null() {
        return Mp4Error::NullPointer;
    }

    let Some(finalized) = muxer.inner.finalized_boxes() else {
        return Mp4Error::InvalidState;
    };

    unsafe {
        *out_enabled = finalized.is_faststart_enabled();
    }

    Mp4Error::Ok
}
*/
