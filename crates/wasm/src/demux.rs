//! shiguredo_mp4::demux の wasm FFI

use shiguredo_mp4::BaseBox;

use crate::boxes::{Mp4SampleEntry, Mp4SampleEntryOwned};

/// エラーコード
#[repr(u32)]
pub enum Mp4WasmError {
    Ok = 0,
    NullPointer = 1,
    InputRequired = 2,
    NoMoreSamples = 3,
    InvalidState = 4,
    DecodeError = 5,
    Unsupported = 6,
}

/// トラックの種類
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum Mp4WasmTrackKind {
    Video = 0,
    Audio = 1,
}

impl From<shiguredo_mp4::TrackKind> for Mp4WasmTrackKind {
    fn from(kind: shiguredo_mp4::TrackKind) -> Self {
        match kind {
            shiguredo_mp4::TrackKind::Video => Mp4WasmTrackKind::Video,
            shiguredo_mp4::TrackKind::Audio => Mp4WasmTrackKind::Audio,
        }
    }
}

/// トラック情報
#[repr(C)]
pub struct Mp4WasmTrackInfo {
    pub track_id: u32,
    pub kind: Mp4WasmTrackKind,
    pub duration: u64,
    pub timescale: u32,
}

impl From<shiguredo_mp4::demux::TrackInfo> for Mp4WasmTrackInfo {
    fn from(track_info: shiguredo_mp4::demux::TrackInfo) -> Self {
        Self {
            track_id: track_info.track_id,
            kind: track_info.kind.into(),
            duration: track_info.duration,
            timescale: track_info.timescale.get(),
        }
    }
}

/// サンプル情報
#[repr(C)]
pub struct Mp4WasmSample {
    pub track_id: u32,
    pub sample_entry: *const Mp4SampleEntry,
    pub keyframe: u32,
    pub timestamp: u64,
    pub duration: u32,
    pub data_offset: u64,
    pub data_size: u32,
}

/// MP4 ファイルデマルチプレクサ
pub struct Mp4WasmFileDemuxer {
    inner: shiguredo_mp4::demux::Mp4FileDemuxer,
    tracks: Vec<Mp4WasmTrackInfo>,
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
pub extern "C" fn mp4_wasm_demuxer_new() -> *mut Mp4WasmFileDemuxer {
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
pub unsafe extern "C" fn mp4_wasm_demuxer_free(demuxer: *mut Mp4WasmFileDemuxer) {
    if !demuxer.is_null() {
        let _ = unsafe { Box::from_raw(demuxer) };
    }
}

/// 最後のエラーメッセージを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_demuxer_get_last_error(
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
pub unsafe extern "C" fn mp4_wasm_demuxer_get_required_input_position(
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
pub unsafe extern "C" fn mp4_wasm_demuxer_get_required_input_size(
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
pub unsafe extern "C" fn mp4_wasm_demuxer_handle_input(
    demuxer: *mut Mp4WasmFileDemuxer,
    position: u64,
    data: *const u8,
    data_size: u32,
) -> Mp4WasmError {
    if demuxer.is_null() {
        return Mp4WasmError::NullPointer;
    }
    let demuxer = unsafe { &mut *demuxer };

    if data.is_null() {
        demuxer.set_last_error("data is null");
        return Mp4WasmError::NullPointer;
    }

    let data = unsafe { std::slice::from_raw_parts(data, data_size as usize) };
    let input = shiguredo_mp4::demux::Input { position, data };
    demuxer.inner.handle_input(input);

    Mp4WasmError::Ok
}

/// トラック数を取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_demuxer_get_track_count(demuxer: *mut Mp4WasmFileDemuxer) -> i32 {
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
pub unsafe extern "C" fn mp4_wasm_demuxer_get_track(
    demuxer: *const Mp4WasmFileDemuxer,
    index: u32,
) -> *const Mp4WasmTrackInfo {
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

/// 次のサンプルを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_wasm_demuxer_next_sample(
    demuxer: *mut Mp4WasmFileDemuxer,
    out_sample: *mut Mp4WasmSample,
) -> Mp4WasmError {
    if demuxer.is_null() {
        return Mp4WasmError::NullPointer;
    }
    let demuxer = unsafe { &mut *demuxer };

    if out_sample.is_null() {
        demuxer.set_last_error("out_sample is null");
        return Mp4WasmError::NullPointer;
    }

    // トラック情報が未取得なら取得する
    if demuxer.tracks.is_empty() {
        match demuxer.inner.tracks() {
            Ok(tracks) => {
                demuxer.tracks = tracks.iter().map(|t| t.clone().into()).collect();
            }
            Err(shiguredo_mp4::demux::DemuxError::InputRequired(_)) => {
                return Mp4WasmError::InputRequired;
            }
            Err(e) => {
                demuxer.set_last_error(&format!("{e}"));
                return Mp4WasmError::DecodeError;
            }
        }
    }

    match demuxer.inner.next_sample() {
        Ok(Some(sample)) => {
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
                        return Mp4WasmError::Unsupported;
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
                *out_sample = Mp4WasmSample {
                    track_id: sample.track.track_id,
                    sample_entry: sample_entry_ptr,
                    keyframe: if sample.keyframe { 1 } else { 0 },
                    timestamp: sample.timestamp,
                    duration: sample.duration,
                    data_offset: sample.data_offset,
                    data_size: sample.data_size as u32,
                };
            }

            Mp4WasmError::Ok
        }
        Ok(None) => Mp4WasmError::NoMoreSamples,
        Err(shiguredo_mp4::demux::DemuxError::InputRequired(_)) => Mp4WasmError::InputRequired,
        Err(e) => {
            demuxer.set_last_error(&format!("{e}"));
            Mp4WasmError::DecodeError
        }
    }
}
