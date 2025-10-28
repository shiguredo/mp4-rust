//! C 言語から利用するためのインタフェースを定義するためのモジュール
use crate::TrackKind;
use crate::demux::{DemuxError, Input, Mp4FileDemuxer, RequiredInput, Sample, TrackInfo};

/// C言語用のデマルチプレックスエラー型
#[repr(C)]
pub enum CDemuxError {
    /// デコードエラー
    DecodeError = 1,
    /// サンプルテーブルエラー
    SampleTableError = 2,
    /// 入力が必要
    RequiredInput = 3,
}

impl CDemuxError {
    fn from_demux_error(error: &DemuxError) -> Self {
        match error {
            DemuxError::DecodeError(_) => CDemuxError::DecodeError,
            DemuxError::SampleTableError(_) => CDemuxError::SampleTableError,
            DemuxError::RequiredInput(_) => CDemuxError::RequiredInput,
        }
    }
}

/// C言語用のトラック種別型
#[repr(C)]
pub enum CTrackKind {
    /// 映像トラック
    Video = 0,
    /// 音声トラック
    Audio = 1,
}

impl CTrackKind {
    fn from_track_kind(kind: &TrackKind) -> Self {
        match kind {
            TrackKind::Video => CTrackKind::Video,
            TrackKind::Audio => CTrackKind::Audio,
        }
    }
}

/// C言語用のトラック情報構造体
#[repr(C)]
pub struct CTrackInfo {
    /// トラックID
    pub track_id: u32,
    /// トラックの種類
    pub kind: CTrackKind,
    /// トラックの尺（タイムスケール単位）
    pub timescaled_duration: u64,
    /// トラックで使用されているタイムスケール
    pub timescale: u32,
}

impl CTrackInfo {
    fn from_track_info(info: &TrackInfo) -> Self {
        Self {
            track_id: info.track_id,
            kind: CTrackKind::from_track_kind(&info.kind),
            timescaled_duration: info.timescaled_duration,
            timescale: info.timescale.get(),
        }
    }
}

/// C言語用のサンプル情報構造体
#[repr(C)]
pub struct CSample {
    /// サンプルが属するトラックID
    pub track_id: u32,
    /// キーフレームであるかの判定
    pub keyframe: bool,
    /// サンプルのタイムスタンプ（タイムスケール単位）
    pub timescaled_timestamp: u64,
    /// サンプルの尺（タイムスケール単位）
    pub timescaled_duration: u32,
    /// ファイル内におけるサンプルデータの開始位置（バイト単位）
    pub data_offset: u64,
    /// サンプルデータのサイズ（バイト単位）
    pub data_size: usize,
}

impl CSample {
    fn from_sample(sample: &Sample) -> Self {
        Self {
            track_id: sample.track.track_id,
            keyframe: sample.keyframe,
            timescaled_timestamp: sample.timescaled_timestamp,
            timescaled_duration: sample.timescaled_duration,
            data_offset: sample.data_offset,
            data_size: sample.data_size,
        }
    }
}

/// C言語用の必要な入力情報構造体
#[repr(C)]
pub struct CRequiredInput {
    /// 必要なデータの開始位置（バイト単位）
    pub position: u64,
    /// 必要なデータのサイズ（バイト単位、0 = EOF まで）
    pub size: usize,
}

impl CRequiredInput {
    fn from_required_input(required: &RequiredInput) -> Self {
        Self {
            position: required.position,
            size: required.size.unwrap_or(0),
        }
    }
}

/// C言語用のデマルチプレックス入力構造体
#[repr(C)]
pub struct CInput {
    /// バッファ内のデータがファイル内で始まる位置（バイト単位）
    pub position: u64,
    /// ファイルデータのバッファ
    pub data: *const u8,
    /// バッファサイズ
    pub data_len: usize,
}

/// C言語用のMP4デマルチプレックス構造体（不透明型）
pub struct CMp4FileDemuxer {
    inner: Mp4FileDemuxer,
    last_error: Option<DemuxError>,
    /// TODO: doc
    pub last_required_input: Option<CRequiredInput>,
}

/// 新しいデマルチプレックスを作成する
///
/// # Safety
///
/// 返されたポインタはCの^G呼び出し元によって管理される必要があります。
/// 使い終わったら `c_mp4_demuxer_free()` で解放してください。
#[unsafe(no_mangle)]
pub extern "C" fn c_mp4_demuxer_new() -> *mut CMp4FileDemuxer {
    let demuxer = CMp4FileDemuxer {
        inner: Mp4FileDemuxer::new(),
        last_error: None,
        last_required_input: None,
    };
    Box::into_raw(Box::new(demuxer))
}

/// デマルチプレックスを解放する
///
/// # Safety
///
/// `demuxer` は `c_mp4_demuxer_new()` で作成されたポインタである必要があります。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_free(demuxer: *mut CMp4FileDemuxer) {
    if !demuxer.is_null() {
        let _ = unsafe { Box::from_raw(demuxer) };
    }
}

/// 次に必要な入力情報を取得する
///
/// デマルチプレックスが初期化済みの場合は0を返します。
///
/// # Safety
///
/// `demuxer` は有効なポインタである必要があります。
/// `out_required_input` は有効なポインタである必要があります。
///
/// # Returns
///
/// 必要な入力がある場合は1、ない場合は0を返します。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_required_input(
    demuxer: *mut CMp4FileDemuxer,
    out_required_input: *mut CRequiredInput,
) -> i32 {
    if demuxer.is_null() || out_required_input.is_null() {
        return -1;
    }

    let demuxer = unsafe { &mut *demuxer };
    if let Some(required) = demuxer.inner.required_input() {
        unsafe { *out_required_input = CRequiredInput::from_required_input(&required) };
        1
    } else {
        0
    }
}

/// 入力データを処理する
///
/// # Safety
///
/// `demuxer` は有効なポインタである必要があります。
/// `input.data` が `input.data_len` バイト以上指す有効なメモリを指す必要があります。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_handle_input(demuxer: *mut CMp4FileDemuxer, input: CInput) {
    if demuxer.is_null() || input.data.is_null() {
        return;
    }

    let demuxer = unsafe { &mut *demuxer };
    let data = unsafe { core::slice::from_raw_parts(input.data, input.data_len) };
    let input_obj = Input {
        position: input.position,
        data,
    };
    demuxer.inner.handle_input(input_obj);
}

/// トラック数を取得する
///
/// エラーが発生した場合は負の値を返します。
///
/// # Safety
///
/// `demuxer` は有効なポインタである必要があります。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_track_count(demuxer: *mut CMp4FileDemuxer) -> i32 {
    if demuxer.is_null() {
        return -1;
    }

    let demuxer = unsafe { &mut *demuxer };
    match demuxer.inner.tracks() {
        Ok(tracks) => tracks.len() as i32,
        Err(e) => {
            demuxer.last_error = Some(e);
            -1
        }
    }
}

/// 指定したインデックスのトラック情報を取得する
///
/// # Safety
///
/// `demuxer` は有効なポインタである必要があります。
/// `out_track_info` は有効なポインタである必要があります。
///
/// # Returns
///
/// 成功時は0、エラー時は-1を返します。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_get_track(
    demuxer: *mut CMp4FileDemuxer,
    index: u32,
    out_track_info: *mut CTrackInfo,
) -> i32 {
    if demuxer.is_null() || out_track_info.is_null() {
        return -1;
    }

    let demuxer = unsafe { &mut *demuxer };
    match demuxer.inner.tracks() {
        Ok(tracks) => {
            if let Some(track) = tracks.get(index as usize) {
                unsafe { *out_track_info = CTrackInfo::from_track_info(track) };
                0
            } else {
                -1
            }
        }
        Err(e) => {
            demuxer.last_error = Some(e);
            -1
        }
    }
}

/// 次のサンプルを取得する
///
/// # Safety
///
/// `demuxer` は有効なポインタである必要があります。
/// `out_sample` が有効なポインタである必要があります。
///
/// # Returns
///
/// サンプルが存在する場合は1、存在しない場合は0、エラー時は-1を返します。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_next_sample(
    demuxer: *mut CMp4FileDemuxer,
    out_sample: *mut CSample,
) -> i32 {
    if demuxer.is_null() || out_sample.is_null() {
        return -1;
    }

    let demuxer = unsafe { &mut *demuxer };
    match demuxer.inner.next_sample() {
        Ok(Some(sample)) => {
            unsafe { *out_sample = CSample::from_sample(&sample) };
            1
        }
        Ok(None) => 0,
        Err(e) => {
            demuxer.last_error = Some(e);
            -1
        }
    }
}

/// 最後に発生したエラーの種類を取得する
///
/// # Safety
///
/// `demuxer` は有効なポインタである必要があります。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_mp4_demuxer_last_error(demuxer: *mut CMp4FileDemuxer) -> i32 {
    if demuxer.is_null() {
        return -1;
    }

    let demuxer = unsafe { &mut *demuxer };
    match &demuxer.last_error {
        Some(e) => CDemuxError::from_demux_error(e) as i32,
        None => 0,
    }
}
