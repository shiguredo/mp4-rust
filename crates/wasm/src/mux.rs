//! shiguredo_mp4::mux の wasm FFI

use std::num::NonZeroU32;

use crate::demux::Mp4Error;

// c-api の型を re-export
pub use c_api::basic_types::Mp4TrackKind;
pub use c_api::mux::Mp4MuxSample;

struct Output {
    offset: u64,
    data: Vec<u8>,
}

/// MP4 ファイルマルチプレクサ
pub struct Mp4WasmFileMuxer {
    options: shiguredo_mp4::mux::Mp4FileMuxerOptions,
    inner: Option<shiguredo_mp4::mux::Mp4FileMuxer>,
    output_list: Vec<Output>,
    next_output_index: usize,
    last_error: Option<Vec<u8>>,
}

impl Mp4WasmFileMuxer {
    fn set_last_error(&mut self, message: &str) {
        self.last_error = Some(message.as_bytes().to_vec());
    }
}

/// moov ボックスの最大サイズを見積もる
#[unsafe(no_mangle)]
pub extern "C" fn mp4_wasm_estimate_maximum_moov_box_size(
    audio_sample_count: u32,
    video_sample_count: u32,
) -> u32 {
    shiguredo_mp4::mux::estimate_maximum_moov_box_size(&[
        audio_sample_count as usize,
        video_sample_count as usize,
    ]) as u32
}

/// 新しい Mp4WasmFileMuxer を作成する
#[unsafe(no_mangle)]
pub extern "C" fn mp4_wasm_muxer_new() -> *mut Mp4WasmFileMuxer {
    let muxer = Box::new(Mp4WasmFileMuxer {
        options: shiguredo_mp4::mux::Mp4FileMuxerOptions::default(),
        inner: None,
        output_list: Vec::new(),
        next_output_index: 0,
        last_error: None,
    });
    Box::into_raw(muxer)
}

/// Mp4WasmFileMuxer を解放する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_free(muxer: *mut Mp4WasmFileMuxer) {
    if !muxer.is_null() {
        let _ = unsafe { Box::from_raw(muxer) };
    }
}

/// 最後のエラーメッセージを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_get_last_error(
    muxer: *const Mp4WasmFileMuxer,
) -> *const Vec<u8> {
    if muxer.is_null() {
        return std::ptr::null();
    }

    let muxer = unsafe { &*muxer };
    match &muxer.last_error {
        Some(error) => error as *const Vec<u8>,
        None => std::ptr::null(),
    }
}

/// faststart 用の moov ボックスサイズを設定する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_set_reserved_moov_box_size(
    muxer: *mut Mp4WasmFileMuxer,
    size: u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    let muxer = unsafe { &mut *muxer };
    muxer.options.reserved_moov_box_size = size as usize;

    Mp4Error::MP4_ERROR_OK
}

/// マルチプレックス処理を初期化する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_initialize(muxer: *mut Mp4WasmFileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer };

    if muxer.inner.is_some() {
        muxer.set_last_error("Muxer has already been initialized");
        return Mp4Error::MP4_ERROR_INVALID_STATE;
    }

    match shiguredo_mp4::mux::Mp4FileMuxer::with_options(muxer.options.clone()) {
        Ok(inner) => {
            let initial = inner.initial_boxes_bytes();
            muxer.output_list.push(Output {
                offset: 0,
                data: initial.to_vec(),
            });
            muxer.inner = Some(inner);
            Mp4Error::MP4_ERROR_OK
        }
        Err(e) => {
            muxer.set_last_error(&format!("Failed to initialize muxer: {e}"));
            Mp4Error::MP4_ERROR_INVALID_DATA
        }
    }
}

/// 次の出力データがあるかどうかを確認する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_has_output(muxer: *const Mp4WasmFileMuxer) -> u32 {
    if muxer.is_null() {
        return 0;
    }
    let muxer = unsafe { &*muxer };

    if muxer.next_output_index < muxer.output_list.len() {
        1
    } else {
        0
    }
}

/// 次の出力データのオフセットを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_get_output_offset(muxer: *const Mp4WasmFileMuxer) -> u64 {
    if muxer.is_null() {
        return 0;
    }
    let muxer = unsafe { &*muxer };

    muxer
        .output_list
        .get(muxer.next_output_index)
        .map(|o| o.offset)
        .unwrap_or(0)
}

/// 次の出力データのサイズを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_get_output_size(muxer: *const Mp4WasmFileMuxer) -> u32 {
    if muxer.is_null() {
        return 0;
    }
    let muxer = unsafe { &*muxer };

    muxer
        .output_list
        .get(muxer.next_output_index)
        .map(|o| o.data.len() as u32)
        .unwrap_or(0)
}

/// 次の出力データのポインタを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_get_output_ptr(
    muxer: *const Mp4WasmFileMuxer,
) -> *const u8 {
    if muxer.is_null() {
        return std::ptr::null();
    }
    let muxer = unsafe { &*muxer };

    muxer
        .output_list
        .get(muxer.next_output_index)
        .map(|o| o.data.as_ptr())
        .unwrap_or(std::ptr::null())
}

/// 次の出力データに進む
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_advance_output(muxer: *mut Mp4WasmFileMuxer) {
    if muxer.is_null() {
        return;
    }
    let muxer = unsafe { &mut *muxer };

    if muxer.next_output_index < muxer.output_list.len() {
        muxer.next_output_index += 1;
    }
}

/// サンプルを追加する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_append_sample(
    muxer: *mut Mp4WasmFileMuxer,
    sample: *const Mp4MuxSample,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer };

    if sample.is_null() {
        muxer.set_last_error("sample is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let sample = unsafe { &*sample };

    let Some(timescale) = NonZeroU32::new(sample.timescale) else {
        muxer.set_last_error("Timescale must be greater than 0");
        return Mp4Error::MP4_ERROR_INVALID_INPUT;
    };

    let sample_entry = if sample.sample_entry.is_null() {
        None
    } else {
        match unsafe { (&*sample.sample_entry).to_sample_entry() } {
            Ok(entry) => Some(entry),
            Err(_) => {
                muxer.set_last_error("Invalid sample entry");
                return Mp4Error::MP4_ERROR_INVALID_INPUT;
            }
        }
    };

    let Some(inner) = &mut muxer.inner else {
        muxer.set_last_error("Muxer has not been initialized");
        return Mp4Error::MP4_ERROR_INVALID_STATE;
    };

    let mux_sample = shiguredo_mp4::mux::Sample {
        track_kind: sample.track_kind.into(),
        sample_entry,
        keyframe: sample.keyframe,
        timescale,
        duration: sample.duration,
        data_offset: sample.data_offset,
        data_size: sample.data_size as usize,
    };

    if let Err(e) = inner.append_sample(&mux_sample) {
        muxer.set_last_error(&format!("Failed to append sample: {e}"));
        Mp4Error::MP4_ERROR_INVALID_DATA
    } else {
        Mp4Error::MP4_ERROR_OK
    }
}

/// マルチプレックス処理を完了する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_muxer_finalize(muxer: *mut Mp4WasmFileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer };

    let Some(inner) = &mut muxer.inner else {
        muxer.set_last_error("Muxer has not been initialized");
        return Mp4Error::MP4_ERROR_INVALID_STATE;
    };

    match inner.finalize() {
        Ok(finalized_boxes) => {
            for (offset, bytes) in finalized_boxes.offset_and_bytes_pairs() {
                muxer.output_list.push(Output {
                    offset,
                    data: bytes.to_vec(),
                });
            }
            Mp4Error::MP4_ERROR_OK
        }
        Err(e) => {
            muxer.set_last_error(&format!("Failed to finalize muxer: {e}"));
            Mp4Error::MP4_ERROR_INVALID_DATA
        }
    }
}
