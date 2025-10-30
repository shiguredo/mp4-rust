//! ../../../src/demux.rs の C API を定義するためのモジュール
use std::ffi::{CString, c_char};

use shiguredo_mp4::BaseBox;

use crate::{
    basic_types::Mp4TrackKind,
    boxes::{Mp4SampleEntry, Mp4SampleEntryOwned},
    error::Mp4Error,
};

/// MP4 デマルチプレックス処理中に抽出されたメディアトラックの情報を表す構造体
#[repr(C)]
pub struct Mp4DemuxTrackInfo {
    /// このトラックを識別するための ID
    pub track_id: u32,

    /// トラックの種類（音声または映像）
    pub kind: Mp4TrackKind,

    /// トラックの尺（タイムスケール単位で表現）
    ///
    /// 実際の時間（秒単位）を得るには、この値を `timescale` で除算すること
    pub duration: u64,

    /// このトラック内で使用されているタイムスケール
    ///
    /// タイムスタンプと尺の単位を定義する値で、1 秒間の単位数を表す
    /// 例えば `timescale` が 1000 の場合、タイムスタンプは 1 ms 単位で表現される
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

/// MP4 デマルチプレックス処理によって抽出されたメディアサンプルを表す構造体
///
/// MP4 ファイル内の各サンプル（フレーム単位の音声または映像データ）のメタデータと
/// ファイル内の位置情報を保持する
///
/// この構造体が参照しているポインタのメモリ管理が `Mp4FileDemuxer` が行っており、
/// `Mp4FileDemuxer` インスタンスが破棄されるまでは安全に参照可能である
#[repr(C)]
pub struct Mp4DemuxSample {
    /// サンプルが属するトラックの情報へのポインタ
    ///
    /// このポインタの参照先には `Mp4FileDemuxer` インスタンスが有効な間のみアクセス可能である
    pub track: *const Mp4DemuxTrackInfo,

    /// サンプルの詳細情報（コーデック設定など）へのポインタ
    ///
    /// このポインタの参照先には `Mp4FileDemuxer` インスタンスが有効な間のみアクセス可能である
    pub sample_entry: *const Mp4SampleEntry,

    /// トラック内でユニークなサンプルエントリーのインデックス番号
    ///
    /// この値を使用して、複数のサンプルが同じコーデック設定を使用しているかどうかを
    /// 簡単に判定できる
    pub sample_entry_index: u32,

    /// このサンプルがキーフレームであるかの判定
    ///
    /// `true` の場合、このサンプルはキーフレームであり、このポイントから復号を開始できる
    ///
    /// 音声の場合には、通常はすべてのサンプルがキーフレーム扱いとなる
    pub keyframe: bool,

    /// サンプルのタイムスタンプ（タイムスケール単位）
    ///
    /// 実際の時間（秒単位）を得るには、この値を対応する `Mp4DemuxTrackInfo` の
    /// `timescale` で除算すること
    pub timestamp: u64,

    /// サンプルの尺（タイムスケール単位）
    ///
    /// 実際の時間（秒単位）を得るには、この値を対応する `Mp4DemuxTrackInfo` の
    /// `timescale` で除算すること
    pub duration: u32,

    /// ファイル内におけるサンプルデータの開始位置（バイト単位）
    ///
    /// 実際のサンプルデータへアクセスするには、この位置から `data_size` 分のバイト列を
    /// 入力ファイルから読み込む必要がある
    pub data_offset: u64,

    /// サンプルデータのサイズ（バイト単位）
    ///
    /// `data_offset` から `data_offset + data_size` までの範囲がサンプルデータとなる
    pub data_size: usize,
}

impl Mp4DemuxSample {
    pub fn new(
        sample: shiguredo_mp4::demux::Sample<'_>,
        track: &Mp4DemuxTrackInfo,
        sample_entry: &Mp4SampleEntry,
    ) -> Self {
        Self {
            track,
            sample_entry,
            sample_entry_index: sample.sample_entry_index as u32,
            keyframe: sample.keyframe,
            timestamp: sample.timescaled_timestamp,
            duration: sample.timescaled_duration,
            data_offset: sample.data_offset,
            data_size: sample.data_size,
        }
    }
}

/// MP4 ファイルをデマルチプレックスして、メディアサンプルを時系列順に取得するための構造体
///
/// # 関連関数
///
/// この構造体は、直接ではなく、以下の関数を通して操作する必要がある:
/// - `mp4_file_demuxer_new()`: `Mp4FileDemuxer` インスタンスを生成する
/// - `mp4_file_demuxer_free()`: リソースを解放する
/// - `mp4_file_demuxer_get_required_input()`: 次の処理に必要な入力データの位置とサイズを取得する
/// - `mp4_file_demuxer_handle_input()`: ファイルデータを入力として受け取る
/// - `mp4_file_demuxer_get_tracks()`: MP4 ファイル内のすべてのメディアトラック情報を取得する
/// - `mp4_file_demuxer_next_sample()`: 時系列順に次のサンプルを取得する
/// - `mp4_file_demuxer_get_last_error()`: 最後に発生したエラーのメッセージを取得する
///
/// # Examples
///
/// ```c
/// // デマルチプレックスの初期化
/// Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
///
/// // 入力ファイルデータを供給
/// while (true) {
///     uint64_t required_pos;
///     int32_t required_size;
///     mp4_file_demuxer_get_required_input(demuxer, &required_pos, &required_size);
///     if (required_size == 0) break;
///
///     uint8_t buffer[4096]; // NOTE: 実際には required_size に合わせて動的に確保するべき
///     size_t bytes_read = read_file_data(required_pos, buffer, sizeof(buffer));
///     mp4_file_demuxer_handle_input(demuxer, required_pos, buffer, bytes_read);
/// }
///
/// // トラック情報を取得
/// const Mp4DemuxTrackInfo *tracks;
/// uint32_t track_count;
/// mp4_file_demuxer_get_tracks(demuxer, &tracks, &track_count);
///
/// // サンプルを取得
/// Mp4DemuxSample sample;
/// while (mp4_file_demuxer_next_sample(demuxer, &sample) == MP4_ERROR_OK) {
///     // サンプルを処理
///     // ...
/// }
///
/// // リソース解放
/// mp4_file_demuxer_free(demuxer);
/// ```
#[repr(C)]
pub struct Mp4FileDemuxer {
    _private: [u8; 0],
}

// [NOTE]
// この構造体を直接公開関数で参照すると cbindgen が、
// 隠蔽したい内部フィールドまで C のヘッダーファイルに含めてしまうので、
// 公開用には Mp4FileDemuxer を用意して、実際の実装はこちらで行っている
struct Mp4FileDemuxerImpl {
    inner: shiguredo_mp4::demux::Mp4FileDemuxer,
    tracks: Vec<Mp4DemuxTrackInfo>,
    sample_entries: Vec<(
        shiguredo_mp4::boxes::SampleEntry,
        Mp4SampleEntryOwned,
        // [NOTE]
        // tracks とは異なり sample_entries は途中でサイズが変わる可能性があるので、
        // その際に C 側で保持されているポインタが無効にならないように Box でラップしておく
        Box<Mp4SampleEntry>,
    )>,
    last_error_string: Option<CString>,
}

impl Mp4FileDemuxerImpl {
    fn set_last_error(&mut self, message: &str) {
        self.last_error_string = CString::new(message).ok();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_demuxer_new() -> *mut Mp4FileDemuxer {
    let impl_data = Box::new(Mp4FileDemuxerImpl {
        inner: shiguredo_mp4::demux::Mp4FileDemuxer::new(),
        tracks: Vec::new(),
        sample_entries: Vec::new(),
        last_error_string: None,
    });
    Box::into_raw(impl_data) as *mut Mp4FileDemuxer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_free(demuxer: *mut Mp4FileDemuxer) {
    if !demuxer.is_null() {
        let _ = unsafe { Box::from_raw(demuxer.cast::<Mp4FileDemuxerImpl>()) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_demuxer_get_last_error(
    demuxer: *const Mp4FileDemuxer,
) -> *const c_char {
    if demuxer.is_null() {
        return c"Invalid demuxer: null pointer".as_ptr();
    }

    let demuxer = unsafe { &*demuxer.cast::<Mp4FileDemuxerImpl>() };
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
    let demuxer = unsafe { &mut *demuxer.cast::<Mp4FileDemuxerImpl>() };

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
    let demuxer = unsafe { &mut *demuxer.cast::<Mp4FileDemuxerImpl>() };

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
    let demuxer = unsafe { &mut *demuxer.cast::<Mp4FileDemuxerImpl>() };

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
    let demuxer = unsafe { &mut *demuxer.cast::<Mp4FileDemuxerImpl>() };

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
