//! ../../../src/mux.rs の C API を定義するためのモジュール
use std::{
    ffi::{CString, c_char},
    time::Duration,
};

use crate::{basic_types::Mp4TrackKind, boxes::Mp4SampleEntry, error::Mp4Error};

/// MP4 ファイルに追加（マルチプレックス）するメディアサンプルを表す構造体
///
/// # 使用例
///
/// ```c
/// // H.264 ビデオサンプルを作成
/// Mp4MuxSample video_sample = {
///     .track_kind = MP4_TRACK_KIND_VIDEO,
///     .sample_entry = &avc1_entry,
///     .keyframe = true,
///     .duration_micros = 33333,  // 33.333 ms (30 fps の場合)
///     .data_offset = 1024,
///     .data_size = 4096,
/// };
///
/// // Opus 音声サンプルを作成
/// Mp4MuxSample audio_sample = {
///     .track_kind = MP4_TRACK_KIND_AUDIO,
///     .sample_entry = &opus_entry,
///     .keyframe = true,  // 音声では通常は常に true
///     .duration_micros = 20000,  // 20 ms
///     .data_offset = 5120,
///     .data_size = 256,
/// };
/// ```
#[repr(C)]
pub struct Mp4MuxSample {
    /// サンプルが属するトラックの種別
    pub track_kind: Mp4TrackKind,

    /// サンプルの詳細情報（コーデック種別など）へのポインタ
    ///
    /// 最初のサンプルでは必須
    ///
    /// 以降は省略可能で、NULL が渡された場合は前のサンプルと同じ値が使用される
    pub sample_entry: *const Mp4SampleEntry,

    /// キーフレームであるかどうか
    ///
    /// `true` の場合、このサンプルはキーフレームであり、
    /// このポイントから復号（再生）を開始できることを意味する
    pub keyframe: bool,

    /// サンプルの尺（マイクロ秒単位）
    ///
    /// # 時間単位について
    ///
    /// MP4 ファイル自体の仕様では、任意の時間単位が指定できるが
    /// `Mp4FileMuxer` では、簡単のためにマイクロ秒固定となっている
    ///
    /// TODO: 別 PR でマイクロ秒固定はやめる
    ///
    /// # サンプルのタイムスタンプについて
    ///
    /// MP4 ではサンプルのタイムスタンプを直接指定する方法がなく、
    /// あるサンプルのタイムスタンプは「それ以前のサンプルの尺の累積」として表現される
    ///
    /// そのため、映像および音声サンプルの冒頭ないし途中でタイムスタンプのギャップが発生する場合には
    /// 利用側で以下のような対処が求められる:
    /// - 映像:
    ///   - 黒画像などを生成してギャップ分を補完するか、サンプルの尺を調整する
    ///   - たとえば、ギャップが発生した直前のサンプルの尺にギャップ期間分を加算する
    /// - 音声:
    ///   - 無音などを生成してギャップ分を補完する
    ///   - 音声はサンプルデータに対する尺の長さが固定なので、映像のように MP4 レイヤーで尺の調整はできない
    ///
    /// なお、MP4 の枠組みでもギャップを表現するためのボックスは存在するが
    /// プレイヤーの対応がまちまちであるため `Mp4FileMuxer` では現状サポートしておらず、
    /// 上述のような個々のプレイヤーの実装への依存性が低い方法を推奨している
    pub duration_micros: u64,

    /// 出力ファイル内におけるサンプルデータの開始位置（バイト単位）
    pub data_offset: u64,

    /// サンプルデータのサイズ（バイト単位）
    pub data_size: u32,
}

struct Output {
    offset: u64,
    data: Vec<u8>,
}

/// メディアトラック（音声・映像）を含んだ MP4 ファイルの構築（マルチプレックス）処理を行うための構造体
///
/// # 関連関数
///
/// この構造体は、以下の関数を通して操作する必要がある:
/// - `mp4_file_muxer_new()`: `Mp4FileMuxer` インスタンスを生成する
/// - `mp4_file_muxer_free()`: リソースを解放する
/// - `mp4_file_muxer_set_reserved_moov_box_size()`: faststart 用に事前確保する moov ボックスのサイズを設定する
/// - `mp4_file_muxer_set_creation_timestamp()`: ファイル作成時刻を設定する
/// - `mp4_file_muxer_initialize()`: マルチプレックス処理を初期化する
/// - `mp4_file_muxer_append_sample()`: サンプルを追加する
/// - `mp4_file_muxer_next_output()`: 出力データを取得する
/// - `mp4_file_muxer_finalize()`: マルチプレックス処理を完了する
/// - `mp4_file_muxer_get_last_error()`: 最後に発生したエラーのメッセージを取得する
///
/// # 使用例
///
/// ```c
/// #include <stdio.h>
/// #include <stdlib.h>
/// #include <stdint.h>
/// #include <string.h>
/// #include "mp4.h"
///
/// int main() {
///     // 1. Mp4FileMuxer インスタンスを生成
///     Mp4FileMuxer *muxer = mp4_file_muxer_new();
///
///     // ファイルをオープン
///     FILE *fp = fopen("output.mp4", "wb");
///     if (fp == NULL) {
///         fprintf(stderr, "Failed to open output file\n");
///         mp4_file_muxer_free(muxer);
///         return 1;
///     }
///
///     // 2. オプション設定（必要に応じて）
///     mp4_file_muxer_set_reserved_moov_box_size(muxer, 8192);
///     mp4_file_muxer_set_creation_timestamp(muxer, 0);  // 0 = UNIX エポック
///
///     // 3. マルチプレックス処理を初期化
///     Mp4Error ret = mp4_file_muxer_initialize(muxer);
///     if (ret != MP4_ERROR_OK) {
///         fprintf(stderr, "初期化失敗: %s\n", mp4_file_muxer_get_last_error(muxer));
///         mp4_file_muxer_free(muxer);
///         fclose(fp);
///         return 1;
///     }
///
///     // 4. 初期出力データをファイルに書き込む
///     uint64_t output_offset;
///     uint32_t output_size;
///     const uint8_t *output_data;
///     while (mp4_file_muxer_next_output(muxer, &output_offset, &output_size, &output_data) == MP4_ERROR_OK) {
///         if (output_size > 0) {
///             fseek(fp, output_offset, SEEK_SET);
///             fwrite(output_data, 1, output_size, fp);
///         } else {
///             break;
///         }
///     }
///
///     // 5. サンプルを追加
///
///     // サンプルデータを準備（例：4096 バイトのダミー VP8 フレームデータ）
///     uint8_t video_sample_data[4096];
///     memset(video_sample_data, 0, sizeof(video_sample_data));
///
///     // サンプルデータをファイルに書き込み
///     fwrite(video_sample_data, 1, sizeof(video_sample_data), fp);
///
///     // VP08（VP8）サンプルエントリーを作成
///     Mp4SampleEntryVp08 vp08_data = {
///         .width = 1920,
///         .height = 1080,
///         .bit_depth = 8,
///         .chroma_subsampling = 1,  // 4:2:0
///         .video_full_range_flag = false,
///         .colour_primaries = 1,     // BT.709
///         .transfer_characteristics = 1,  // BT.709
///         .matrix_coefficients = 1,  // BT.709
///     };
///
///     Mp4SampleEntryData sample_entry_data;
///     sample_entry_data.vp08 = vp08_data;
///
///     Mp4SampleEntry sample_entry = {
///         .kind = MP4_SAMPLE_ENTRY_KIND_VP08,
///         .data = sample_entry_data,
///     };
///
///     Mp4MuxSample video_sample = {
///         .track_kind = MP4_TRACK_KIND_VIDEO,
///         .sample_entry = &sample_entry,
///         .keyframe = true,
///         .duration_micros = 33333,  // ~30 fps
///         .data_offset = output_offset + output_size,
///         .data_size = sizeof(video_sample_data),
///     };
///     ret = mp4_file_muxer_append_sample(muxer, &video_sample);
///     if (ret != MP4_ERROR_OK) {
///         fprintf(stderr, "Failed to append sample: %s\n", mp4_file_muxer_get_last_error(muxer));
///         mp4_file_muxer_free(muxer);
///         fclose(fp);
///         return 1;
///     }
///
///     // 6. マルチプレックス処理を完了
///     ret = mp4_file_muxer_finalize(muxer);
///     if (ret != MP4_ERROR_OK) {
///         fprintf(stderr, "ファイナライズ失敗: %s\n", mp4_file_muxer_get_last_error(muxer));
///         mp4_file_muxer_free(muxer);
///         fclose(fp);
///         return 1;
///     }
///
///     // 7. ファイナライズ後のボックスデータをファイルに書き込む
///     while (mp4_file_muxer_next_output(muxer, &output_offset, &output_size, &output_data) == MP4_ERROR_OK) {
///         if (output_size > 0) {
///             fseek(fp, output_offset, SEEK_SET);
///             fwrite(output_data, 1, output_size, fp);
///         } else {
///             break;
///         }
///     }
///
///     // 8. リソース解放
///     mp4_file_muxer_free(muxer);
///     fclose(fp);
///
///     printf("MP4 file created successfully: output.mp4\n");
///     return 0;
/// }
/// ```
#[repr(C)]
pub struct Mp4FileMuxer {
    _private: [u8; 0],
}

// [NOTE]
// この構造体を直接公開関数で参照すると cbindgen が、
// 隠蔽したい内部フィールドまで C のヘッダーファイルに含めてしまうので、
// 公開用には Mp4FileMuxer を用意して、実際の実装はこちらで行っている
struct Mp4FileMuxerImpl {
    options: shiguredo_mp4::mux::Mp4FileMuxerOptions,
    inner: Option<shiguredo_mp4::mux::Mp4FileMuxer>,
    last_error_string: Option<CString>,
    output_list: Vec<Output>,
    next_output_index: usize,
}

impl Mp4FileMuxerImpl {
    fn set_last_error(&mut self, message: &str) {
        self.last_error_string = CString::new(message).ok();
    }
}

/// 構築する MP4 ファイルの moov ボックスの最大サイズを見積もるための関数
///
/// この関数を使うことで `mp4_file_muxer_set_reserved_moov_box_size()` で指定する値を簡易的に決定することができる
///
/// # 引数
///
/// - `audio_sample_count`: 音声トラック内の予想サンプル数
/// - `video_sample_count`: 映像トラック内の予想サンプル数
///
/// # 戻り値
///
/// moov ボックスに必要な最大バイト数を返す
///
/// # 使用例
///
/// ```c
/// // 音声 1000 サンプル、映像 3000 フレームの場合
/// uint32_t required_size = mp4_estimate_maximum_moov_box_size(1000, 3000);
/// mp4_file_muxer_set_reserved_moov_box_size(muxer, required_size);
/// ```
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

/// 新しい `Mp4FileMuxer` インスタンスを作成して、それへのポインタを返す
///
/// 返されたポインタは、使用後に `mp4_file_muxer_free()` で破棄する必要がある
///
/// # 戻り値
///
/// 新しく作成された `Mp4FileMuxer` インスタンスへのポインタ
/// （現在の実装では NULL ポインタが返されることはない）
///
/// # 関連関数
///
/// - `mp4_file_muxer_free()`: インスタンスを破棄してリソースを解放する
/// - `mp4_file_muxer_initialize()`: マルチプレックス処理を初期化する
/// - `mp4_file_muxer_set_reserved_moov_box_size()`: faststart 用に moov ボックスサイズを設定する
/// - `mp4_file_muxer_set_creation_timestamp()`: ファイル作成時刻を設定する
///
/// # 使用例
///
/// ```c
/// // Mp4FileMuxer インスタンスを生成
/// Mp4FileMuxer *muxer = mp4_file_muxer_new();
///
/// // オプションを設定
/// mp4_file_muxer_set_reserved_moov_box_size(muxer, 8192);
///
/// // マルチプレックス処理を初期化
/// Mp4Error ret = mp4_file_muxer_initialize(muxer);
/// if (ret != MP4_ERROR_OK) {
///     fprintf(stderr, "初期化失敗: %s\n", mp4_file_muxer_get_last_error(muxer));
///     mp4_file_muxer_free(muxer);
///     return 1;
/// }
///
/// // サンプルを追加...（省略）
///
/// // マルチプレックス処理を完了
/// ret = mp4_file_muxer_finalize(muxer);
/// if (ret != MP4_ERROR_OK) {
///     fprintf(stderr, "ファイナライズ失敗: %s\n", mp4_file_muxer_get_last_error(muxer));
///     mp4_file_muxer_free(muxer);
///     return 1;
/// }
///
/// // リソース解放
/// mp4_file_muxer_free(muxer);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn mp4_file_muxer_new() -> *mut Mp4FileMuxer {
    let impl_data = Box::new(Mp4FileMuxerImpl {
        options: shiguredo_mp4::mux::Mp4FileMuxerOptions::default(),
        inner: None,
        last_error_string: None,
        output_list: Vec::new(),
        next_output_index: 0,
    });
    Box::into_raw(impl_data).cast()
}

/// `Mp4FileMuxer` インスタンスを破棄して、割り当てられたリソースを解放する
///
/// この関数は、`mp4_file_muxer_new()` で作成された `Mp4FileMuxer` インスタンスを破棄し、
/// その内部で割り当てられたすべてのメモリを解放する
///
/// # 引数
///
/// - `muxer`: 破棄する `Mp4FileMuxer` インスタンスへのポインタ
///   - NULL ポインタが渡された場合、この関数は何もしない
///
/// # 使用例
///
/// ```c
/// // Mp4FileMuxer インスタンスを生成
/// Mp4FileMuxer *muxer = mp4_file_muxer_new();
///
/// // マルチプレックス処理を実行（省略）...
///
/// // リソース解放
/// mp4_file_muxer_free(muxer);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_free(muxer: *mut Mp4FileMuxer) {
    if !muxer.is_null() {
        let _ = unsafe { Box::from_raw(muxer.cast::<Mp4FileMuxerImpl>()) };
    }
}

/// `Mp4FileMuxer` で最後に発生したエラーのメッセージを取得する
///
/// このメソッドは、マルチプレックス処理中に発生した最後のエラーのメッセージ（NULL 終端）を返す
///
/// エラーが発生していない場合は、空文字列へのポインタを返す
///
/// # 引数
///
/// - `muxer`: `Mp4FileMuxer` インスタンスへのポインタ
///   - NULL ポインタが渡された場合、NULL 終端の空文字列へのポインタを返す
///
/// # 戻り値
///
/// - メッセージが存在する場合: NULL 終端のエラーメッセージへのポインタ
/// - メッセージが存在しない場合: NULL 終端の空文字列へのポインタ
/// - `muxer` 引数が NULL の場合: NULL 終端の空文字列へのポインタ
///
/// # 使用例
///
/// ```c
/// Mp4FileMuxer *muxer = mp4_file_muxer_new();
///
/// Mp4Error ret = mp4_file_muxer_initialize(muxer);
///
/// // エラーが発生した場合、メッセージを取得
/// if (ret != MP4_ERROR_OK) {
///     const char *error_msg = mp4_file_muxer_get_last_error(muxer);
///     fprintf(stderr, "エラー: %s\n", error_msg);
/// }
///
/// mp4_file_muxer_free(muxer);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_get_last_error(
    muxer: *const Mp4FileMuxer,
) -> *const c_char {
    if muxer.is_null() {
        return c"".as_ptr();
    }

    let muxer = unsafe { &*muxer.cast::<Mp4FileMuxerImpl>() };
    let Some(e) = &muxer.last_error_string else {
        return c"".as_ptr();
    };
    e.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_set_reserved_moov_box_size(
    muxer: *mut Mp4FileMuxer,
    size: u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    let muxer = unsafe { &mut *muxer.cast::<Mp4FileMuxerImpl>() };
    muxer.options.reserved_moov_box_size = size as usize;

    Mp4Error::MP4_ERROR_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_set_creation_timestamp(
    muxer: *mut Mp4FileMuxer,
    timestamp_micros: u64,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }

    let muxer = unsafe { &mut *muxer.cast::<Mp4FileMuxerImpl>() };
    let timestamp = Duration::from_micros(timestamp_micros);
    muxer.options.creation_timestamp = timestamp;

    Mp4Error::MP4_ERROR_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_initialize(muxer: *mut Mp4FileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer.cast::<Mp4FileMuxerImpl>() };

    if muxer.inner.is_some() {
        muxer.set_last_error("[mp4_file_muxer_initialize] Muxer has already been initialized");
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
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer.cast::<Mp4FileMuxerImpl>() };

    if out_output_offset.is_null() {
        muxer.set_last_error("[mp4_file_muxer_next_output] out_output_offset is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    if out_output_size.is_null() {
        muxer.set_last_error("[mp4_file_muxer_next_output] out_output_size is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    if out_output_data.is_null() {
        muxer.set_last_error("[mp4_file_muxer_next_output] out_output_data is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
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
            *out_output_data = std::ptr::null_mut();
        }
    }

    Mp4Error::MP4_ERROR_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_append_sample(
    muxer: *mut Mp4FileMuxer,
    sample: *const Mp4MuxSample,
) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer.cast::<Mp4FileMuxerImpl>() };

    if muxer.next_output_index < muxer.output_list.len() {
        muxer.set_last_error(
            "[mp4_file_muxer_append_sample] Output required before appending more samples",
        );
        return Mp4Error::MP4_ERROR_OUTPUT_REQUIRED;
    }

    if sample.is_null() {
        muxer.set_last_error("[mp4_file_muxer_append_sample] sample is null");
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let sample = unsafe { &*sample };

    let duration = Duration::from_micros(sample.duration_micros);
    let sample_entry = if sample.sample_entry.is_null() {
        None
    } else {
        unsafe {
            match (&*sample.sample_entry).to_sample_entry() {
                Ok(entry) => Some(entry),
                Err(e) => {
                    muxer.set_last_error("[mp4_file_muxer_append_sample] Invalid sample entry");
                    return e;
                }
            }
        }
    };

    let Some(inner) = &mut muxer.inner else {
        muxer.set_last_error("[mp4_file_muxer_append_sample] Muxer has not been initialized");
        return Mp4Error::MP4_ERROR_INVALID_STATE;
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
        Mp4Error::MP4_ERROR_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_file_muxer_finalize(muxer: *mut Mp4FileMuxer) -> Mp4Error {
    if muxer.is_null() {
        return Mp4Error::MP4_ERROR_NULL_POINTER;
    }
    let muxer = unsafe { &mut *muxer.cast::<Mp4FileMuxerImpl>() };

    if muxer.next_output_index < muxer.output_list.len() {
        muxer.set_last_error("[mp4_file_muxer_finalize] Output required before finalizing");
        return Mp4Error::MP4_ERROR_OUTPUT_REQUIRED;
    }

    let Some(inner) = &mut muxer.inner else {
        muxer.set_last_error("[mp4_file_muxer_finalize] Muxer has not been initialized");
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
            muxer.set_last_error(&format!(
                "[mp4_file_muxer_finalize] Failed to finalize muxer: {e}"
            ));
            e.into()
        }
    }
}
