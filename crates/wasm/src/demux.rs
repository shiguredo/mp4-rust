//! shiguredo_mp4::demux の wasm バインディング

use shiguredo_mp4::BaseBox;

use crate::boxes::{Mp4SampleEntry, Mp4SampleEntryOwned, ToJson};

// c-api の型を re-export
pub use c_api::basic_types::Mp4TrackKind;
pub use c_api::demux::{Mp4DemuxSample, Mp4DemuxTrackInfo};
pub use c_api::error::Mp4Error;

/// MP4 ファイルデマルチプレクサ
pub struct Mp4WasmFileDemuxer {
    inner: shiguredo_mp4::demux::Mp4FileDemuxer,
    tracks: Vec<Mp4DemuxTrackInfo>,
    sample_entries: Vec<(
        shiguredo_mp4::boxes::SampleEntry,
        Mp4SampleEntryOwned,
        Box<Mp4SampleEntry>,
    )>,
    last_error: Option<Vec<u8>>,
}

impl Mp4WasmFileDemuxer {
    fn set_last_error(&mut self, message: &str) {
        self.last_error = Some(message.as_bytes().to_vec());
    }
}

/// 新しい Mp4WasmFileDemuxer を作成する
#[unsafe(no_mangle)]
pub extern "C" fn mp4_demuxer_new() -> *mut Mp4WasmFileDemuxer {
    let demuxer = Box::new(Mp4WasmFileDemuxer {
        inner: shiguredo_mp4::demux::Mp4FileDemuxer::new(),
        tracks: Vec::new(),
        sample_entries: Vec::new(),
        last_error: None,
    });
    Box::into_raw(demuxer)
}

/// Mp4WasmFileDemuxer を解放する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_free(demuxer: *mut Mp4WasmFileDemuxer) {
    if !demuxer.is_null() {
        let _ = unsafe { Box::from_raw(demuxer) };
    }
}

/// 最後のエラーメッセージを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_get_last_error(
    demuxer: *const Mp4WasmFileDemuxer,
) -> *const Vec<u8> {
    if demuxer.is_null() {
        return std::ptr::null();
    }

    let demuxer = unsafe { &*demuxer };
    match &demuxer.last_error {
        Some(error) => error as *const Vec<u8>,
        None => std::ptr::null(),
    }
}

/// 次の処理に必要な入力の位置を取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_get_required_input_position(
    demuxer: *const Mp4WasmFileDemuxer,
) -> u64 {
    if demuxer.is_null() {
        return 0;
    }

    let demuxer = unsafe { &*demuxer };
    demuxer
        .inner
        .required_input()
        .map(|r| r.position)
        .unwrap_or(0)
}

/// 次の処理に必要な入力のサイズを取得する
///
/// 0: 入力不要
/// -1: ファイル末尾まで必要
/// その他: 必要なバイト数
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_get_required_input_size(
    demuxer: *const Mp4WasmFileDemuxer,
) -> i32 {
    if demuxer.is_null() {
        return 0;
    }

    let demuxer = unsafe { &*demuxer };
    match demuxer.inner.required_input() {
        Some(required) => required.size.map(|n| n as i32).unwrap_or(-1),
        None => 0,
    }
}

/// 入力データを供給する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_handle_input(
    demuxer: *mut Mp4WasmFileDemuxer,
    position: u64,
    data: *const u8,
    data_size: u32,
) -> Mp4Error {
    if demuxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let demuxer = unsafe { &mut *demuxer };

    if data.is_null() {
        demuxer.set_last_error("data is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    let data = unsafe { std::slice::from_raw_parts(data, data_size as usize) };
    let input = shiguredo_mp4::demux::Input { position, data };
    demuxer.inner.handle_input(input);

    Mp4Error::MP4_ERROR_OK
}

/// トラック数を取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_get_track_count(demuxer: *mut Mp4WasmFileDemuxer) -> i32 {
    if demuxer.is_null() {
        return -1;
    }
    let demuxer = unsafe { &mut *demuxer };

    if demuxer.tracks.is_empty() {
        match demuxer.inner.tracks() {
            Ok(tracks) => {
                demuxer.tracks = tracks.iter().map(|t| t.clone().into()).collect();
            }
            Err(shiguredo_mp4::demux::DemuxError::InputRequired(_)) => {
                return -2;
            }
            Err(e) => {
                demuxer.set_last_error(&format!("{e}"));
                return -3;
            }
        }
    }

    demuxer.tracks.len() as i32
}

/// トラック情報を取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_get_track(
    demuxer: *const Mp4WasmFileDemuxer,
    index: u32,
) -> *const Mp4DemuxTrackInfo {
    if demuxer.is_null() {
        return std::ptr::null();
    }
    let demuxer = unsafe { &*demuxer };

    demuxer
        .tracks
        .get(index as usize)
        .map(|t| t as *const _)
        .unwrap_or(std::ptr::null())
}

/// サンプルエントリを JSON として取得する
///
/// sample_entry ポインタからJSON文字列を返す。
/// TypeScript 側で mp4_vec_ptr/mp4_vec_len を使って読み取る。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_sample_entry_to_json(
    demuxer: *const Mp4WasmFileDemuxer,
    sample_entry: *const Mp4SampleEntry,
) -> *const Vec<u8> {
    if demuxer.is_null() || sample_entry.is_null() {
        return std::ptr::null();
    }
    let demuxer = unsafe { &*demuxer };

    // sample_entry のアドレスから対応する Mp4SampleEntryOwned を探す
    for (_, owned, boxed) in &demuxer.sample_entries {
        if std::ptr::eq(&**boxed, sample_entry) {
            let json = owned.to_json();
            return Box::into_raw(Box::new(json.into_bytes()));
        }
    }

    std::ptr::null()
}

/// 次のサンプルを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demuxer_next_sample(
    demuxer: *mut Mp4WasmFileDemuxer,
    out_sample: *mut Mp4DemuxSample,
) -> Mp4Error {
    if demuxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let demuxer = unsafe { &mut *demuxer };

    if out_sample.is_null() {
        demuxer.set_last_error("out_sample is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    // トラック情報が未取得なら取得する
    if demuxer.tracks.is_empty() {
        match demuxer.inner.tracks() {
            Ok(tracks) => {
                demuxer.tracks = tracks.iter().map(|t| t.clone().into()).collect();
            }
            Err(shiguredo_mp4::demux::DemuxError::InputRequired(_)) => {
                return Mp4Error::MP4_ERROR_INPUT_REQUIRED;
            }
            Err(e) => {
                demuxer.set_last_error(&format!("{e}"));
                return Mp4Error::MP4_ERROR_INVALID_DATA;
            }
        }
    }

    match demuxer.inner.next_sample() {
        Ok(Some(sample)) => {
            // トラック情報を取得
            let track_ptr = demuxer
                .tracks
                .iter()
                .find(|t| t.track_id == sample.track.track_id)
                .map(|t| t as *const _)
                .unwrap_or(std::ptr::null());

            let sample_entry_ptr = if let Some(sample_entry) = sample.sample_entry {
                let sample_entry_box_type = sample_entry.box_type();
                if let Some(entry) = demuxer
                    .sample_entries
                    .iter()
                    .find_map(|entry| (entry.0 == *sample_entry).then_some(&entry.2))
                {
                    &**entry as *const _
                } else {
                    let Some(entry_owned) = Mp4SampleEntryOwned::new(sample_entry.clone()) else {
                        demuxer.set_last_error(&format!(
                            "Unsupported sample entry box type: {sample_entry_box_type}"
                        ));
                        return Mp4Error::MP4_ERROR_UNSUPPORTED;
                    };
                    let entry = Box::new(entry_owned.to_mp4_sample_entry());
                    demuxer
                        .sample_entries
                        .push((sample_entry.clone(), entry_owned, entry));
                    demuxer
                        .sample_entries
                        .last()
                        .map(|entry| &*entry.2 as *const _)
                        .unwrap_or(std::ptr::null())
                }
            } else {
                std::ptr::null()
            };

            unsafe {
                *out_sample = Mp4DemuxSample {
                    track: track_ptr,
                    sample_entry: sample_entry_ptr,
                    keyframe: sample.keyframe,
                    timestamp: sample.timestamp,
                    duration: sample.duration,
                    data_offset: sample.data_offset,
                    data_size: sample.data_size,
                };
            }

            Mp4Error::MP4_ERROR_OK
        }
        Ok(None) => Mp4Error::MP4_ERROR_NO_MORE_SAMPLES,
        Err(shiguredo_mp4::demux::DemuxError::InputRequired(_)) => Mp4Error::MP4_ERROR_INPUT_REQUIRED,
        Err(e) => {
            demuxer.set_last_error(&format!("{e}"));
            Mp4Error::MP4_ERROR_INVALID_DATA
        }
    }
}
