#ifndef SHIGUREDO_MP4_H
#define SHIGUREDO_MP4_H

/* Generated with cbindgen:0.29.2 */

#include <stdbool.h>
#include <stdint.h>

/**
 * 発生する可能性のあるエラーの種類を表現する列挙型
 */
typedef enum Mp4Error {
  /**
   * エラーが発生しなかったことを示す
   */
  MP4_ERROR_OK = 0,
  /**
   * 入力引数ないしパラメーターが無効である
   */
  MP4_ERROR_INVALID_INPUT,
  /**
   * 入力データが破損しているか無効な形式である
   */
  MP4_ERROR_INVALID_DATA,
  /**
   * 操作に対する内部状態が無効である
   */
  MP4_ERROR_INVALID_STATE,
  /**
   * 入力データの読み込みが必要である
   */
  MP4_ERROR_INPUT_REQUIRED,
  /**
   * 出力データの書き込みが必要である
   */
  MP4_ERROR_OUTPUT_REQUIRED,
  /**
   * NULL ポインタが渡された
   */
  MP4_ERROR_NULL_POINTER,
  /**
   * これ以上読み込むサンプルが存在しない
   */
  MP4_ERROR_NO_MORE_SAMPLES,
  /**
   * 操作またはデータ形式がサポートされていない
   */
  MP4_ERROR_UNSUPPORTED,
  /**
   * 上記以外のエラーが発生した
   */
  MP4_ERROR_OTHER,
} Mp4Error;

/**
 * MP4 ファイル内のトラックの種類を表す列挙型
 */
typedef enum Mp4TrackKind {
  /**
   * 音声トラック
   */
  MP4_TRACK_KIND_AUDIO = 0,
  /**
   * 映像トラック
   */
  MP4_TRACK_KIND_VIDEO = 1,
} Mp4TrackKind;

/**
 * サンプルエントリーの種類を表す列挙型
 *
 * MP4 ファイル内で使用されるコーデックの種類を識別するために使用される
 */
typedef enum Mp4SampleEntryKind {
  /**
   * AVC1 (H.264)
   */
  MP4_SAMPLE_ENTRY_KIND_AVC1,
  /**
   * HEV1 (H.265/HEVC)
   */
  MP4_SAMPLE_ENTRY_KIND_HEV1,
  /**
   * VP08 (VP8)
   */
  MP4_SAMPLE_ENTRY_KIND_VP08,
  /**
   * VP09 (VP9)
   */
  MP4_SAMPLE_ENTRY_KIND_VP09,
  /**
   * AV01 (AV1)
   */
  MP4_SAMPLE_ENTRY_KIND_AV01,
  /**
   * Opus
   */
  MP4_SAMPLE_ENTRY_KIND_OPUS,
  /**
   * MP4A (AAC)
   */
  MP4_SAMPLE_ENTRY_KIND_MP4A,
} Mp4SampleEntryKind;

/**
 * MP4 ファイルをデマルチプレックスして、メディアサンプルを時系列順に取得するための構造体
 *
 * # 関連関数
 *
 * この構造体は、直接ではなく、以下の関数を通して操作する必要がある:
 * - `mp4_file_demuxer_new()`: `Mp4FileDemuxer` インスタンスを生成する
 * - `mp4_file_demuxer_free()`: リソースを解放する
 * - `mp4_file_demuxer_get_required_input()`: 次の処理に必要な入力データの位置とサイズを取得する
 * - `mp4_file_demuxer_handle_input()`: ファイルデータを入力として受け取る
 * - `mp4_file_demuxer_get_tracks()`: MP4 ファイル内のすべてのメディアトラック情報を取得する
 * - `mp4_file_demuxer_next_sample()`: 時系列順に次のサンプルを取得する
 * - `mp4_file_demuxer_get_last_error()`: 最後に発生したエラーのメッセージを取得する
 *
 * # Examples
 *
 * ```c
 * // デマルチプレックスの初期化
 * Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
 *
 * // 入力ファイルデータを供給
 * while (true) {
 *     uint64_t required_pos;
 *     int32_t required_size;
 *     mp4_file_demuxer_get_required_input(demuxer, &required_pos, &required_size);
 *     if (required_size == 0) break;
 *
 *     uint8_t buffer[4096]; // NOTE: 実際には required_size に合わせて動的に確保するべき
 *     size_t bytes_read = read_file_data(required_pos, buffer, sizeof(buffer));
 *     mp4_file_demuxer_handle_input(demuxer, required_pos, buffer, bytes_read);
 * }
 *
 * // トラック情報を取得
 * const Mp4DemuxTrackInfo *tracks;
 * uint32_t track_count;
 * Mp4Error ret = mp4_file_demuxer_get_tracks(demuxer, &tracks, &track_count);
 * if (ret == MP4_ERROR_OK) {
 *     // トラック情報を処理
 *     // ...
 * }
 *
 * // サンプルを取得
 * Mp4DemuxSample sample;
 * while (mp4_file_demuxer_next_sample(demuxer, &sample) == MP4_ERROR_OK) {
 *     // サンプルを処理
 *     // ...
 * }
 *
 * // リソース解放
 * mp4_file_demuxer_free(demuxer);
 * ```
 */
typedef struct Mp4FileDemuxer {
  uint8_t _private[0];
} Mp4FileDemuxer;

/**
 * MP4 デマルチプレックス処理中に抽出されたメディアトラックの情報を表す構造体
 */
typedef struct Mp4DemuxTrackInfo {
  /**
   * このトラックを識別するための ID
   */
  uint32_t track_id;
  /**
   * トラックの種類（音声または映像）
   */
  enum Mp4TrackKind kind;
  /**
   * トラックの尺（タイムスケール単位で表現）
   *
   * 実際の時間（秒単位）を得るには、この値を `timescale` で除算すること
   */
  uint64_t duration;
  /**
   * このトラック内で使用されているタイムスケール
   *
   * タイムスタンプと尺の単位を定義する値で、1 秒間の単位数を表す
   * 例えば `timescale` が 1000 の場合、タイムスタンプは 1 ms 単位で表現される
   */
  uint32_t timescale;
} Mp4DemuxTrackInfo;

/**
 * AVC1（H.264）コーデック用のサンプルエントリー
 *
 * H.264 ビデオコーデックの詳細情報を保持する構造体で、
 * 解像度、プロファイル、レベル、SPS/PPS パラメータセットなどの情報が含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * SPS / PPS リストへのアクセス例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_AVC1) {
 *     Mp4SampleEntryAvc1 *avc1 = &entry.data.avc1;
 *
 *     // すべての SPS パラメータセットを処理
 *     for (uint32_t i = 0; i < avc1->sps_count; i++) {
 *         const uint8_t *sps_data = avc1->sps_data[i];
 *         uint32_t sps_size = avc1->sps_sizes[i];
 *         // SPS データを処理...
 *     }
 *
 *     // すべての PPS パラメータセットを処理
 *     for (uint32_t i = 0; i < avc1->pps_count; i++) {
 *         const uint8_t *pps_data = avc1->pps_data[i];
 *         uint32_t pps_size = avc1->pps_sizes[i];
 *         // PPS データを処理...
 *     }
 * }
 * ```
 */
typedef struct Mp4SampleEntryAvc1 {
  uint16_t width;
  uint16_t height;
  uint8_t avc_profile_indication;
  uint8_t profile_compatibility;
  uint8_t avc_level_indication;
  uint8_t length_size_minus_one;
  const uint8_t *const *sps_data;
  const uint32_t *sps_sizes;
  uint32_t sps_count;
  const uint8_t *const *pps_data;
  const uint32_t *pps_sizes;
  uint32_t pps_count;
  bool is_chroma_format_present;
  uint8_t chroma_format;
  bool is_bit_depth_luma_minus8_present;
  uint8_t bit_depth_luma_minus8;
  bool is_bit_depth_chroma_minus8_present;
  uint8_t bit_depth_chroma_minus8;
} Mp4SampleEntryAvc1;

/**
 * HEV1（H.265/HEVC）コーデック用のサンプルエントリー
 *
 * H.265 ビデオコーデックの詳細情報を保持する構造体で、
 * 解像度、プロファイル、レベル、NALU パラメータセットなどの情報が含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * NALU リストへのアクセス例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_HEV1) {
 *     Mp4SampleEntryHev1 *hev1 = &entry.data.hev1;
 *
 *     // すべての NALU 配列を処理
 *     uint32_t nalu_index = 0;
 *     for (uint32_t i = 0; i < hev1->nalu_array_count; i++) {
 *         uint8_t nalu_type = hev1->nalu_types[i];
 *         uint32_t nalu_count = hev1->nalu_counts[i];
 *
 *         // この NALU タイプのすべてのユニットを処理
 *         for (uint32_t j = 0; j < nalu_count; j++) {
 *             const uint8_t *nalu_data = hev1->nalu_data[nalu_index];
 *             uint32_t nalu_size = hev1->nalu_sizes[nalu_index];
 *             // NALU データを処理...
 *             nalu_index++;
 *         }
 *     }
 * }
 * ```
 */
typedef struct Mp4SampleEntryHev1 {
  uint16_t width;
  uint16_t height;
  uint8_t general_profile_space;
  uint8_t general_tier_flag;
  uint8_t general_profile_idc;
  uint32_t general_profile_compatibility_flags;
  uint64_t general_constraint_indicator_flags;
  uint8_t general_level_idc;
  uint8_t chroma_format_idc;
  uint8_t bit_depth_luma_minus8;
  uint8_t bit_depth_chroma_minus8;
  uint16_t min_spatial_segmentation_idc;
  uint8_t parallelism_type;
  uint16_t avg_frame_rate;
  uint8_t constant_frame_rate;
  uint8_t num_temporal_layers;
  uint8_t temporal_id_nested;
  uint8_t length_size_minus_one;
  uint32_t nalu_array_count;
  const uint8_t *nalu_types;
  const uint32_t *nalu_counts;
  const uint8_t *const *nalu_data;
  const uint32_t *nalu_sizes;
} Mp4SampleEntryHev1;

/**
 * VP08（VP8）コーデック用のサンプルエントリー
 *
 * VP8 ビデオコーデックの詳細情報を保持する構造体で、
 * 解像度、ビット深度、色彩空間情報などが含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * 基本的な使用例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_VP08) {
 *     Mp4SampleEntryVp08 *vp08 = &entry.data.vp08;
 *     printf("解像度: %dx%d\n", vp08->width, vp08->height);
 *     printf("ビット深度: %d\n", vp08->bit_depth);
 *     printf("フルレンジ: %s\n", vp08->video_full_range_flag ? "有効" : "無効");
 * }
 * ```
 */
typedef struct Mp4SampleEntryVp08 {
  uint16_t width;
  uint16_t height;
  uint8_t bit_depth;
  uint8_t chroma_subsampling;
  bool video_full_range_flag;
  uint8_t colour_primaries;
  uint8_t transfer_characteristics;
  uint8_t matrix_coefficients;
} Mp4SampleEntryVp08;

/**
 * VP09（VP9）コーデック用のサンプルエントリー
 *
 * VP9 ビデオコーデックの詳細情報を保持する構造体で、
 * 解像度、プロファイル、レベル、ビット深度、色彩空間情報、
 * およびコーデック初期化データなどが含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * 基本的な使用例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_VP09) {
 *     Mp4SampleEntryVp09 *vp09 = &entry.data.vp09;
 *     printf("解像度: %dx%d\n", vp09->width, vp09->height);
 *     printf("プロファイル: %d\n", vp09->profile);
 *     printf("レベル: %d\n", vp09->level);
 *     printf("ビット深度: %d\n", vp09->bit_depth);
 *
 *     // コーデック初期化データにアクセス
 *     if (vp09->codec_initialization_data_size > 0) {
 *         const uint8_t *init_data = vp09->codec_initialization_data;
 *         uint32_t init_size = vp09->codec_initialization_data_size;
 *         // 初期化データを処理...
 *     }
 * }
 * ```
 */
typedef struct Mp4SampleEntryVp09 {
  uint16_t width;
  uint16_t height;
  uint8_t profile;
  uint8_t level;
  uint8_t bit_depth;
  uint8_t chroma_subsampling;
  bool video_full_range_flag;
  uint8_t colour_primaries;
  uint8_t transfer_characteristics;
  uint8_t matrix_coefficients;
  const uint8_t *codec_initialization_data;
  uint32_t codec_initialization_data_size;
} Mp4SampleEntryVp09;

/**
 * AV01（AV1）コーデック用のサンプルエントリー
 *
 * AV1 ビデオコーデックの詳細情報を保持する構造体で、
 * 解像度、プロファイル、レベル、ビット深度、色彩空間情報、
 * およびコーデック設定 OBU（Open Bitstream Unit）などが含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * 基本的な使用例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_AV01) {
 *     Mp4SampleEntryAv01 *av01 = &entry.data.av01;
 *     printf("解像度: %dx%d\n", av01->width, av01->height);
 *     printf("プロファイル: %d\n", av01->seq_profile);
 *     printf("レベル: %d\n", av01->seq_level_idx_0);
 *     printf("ビット深度: %s\n", av01->high_bitdepth ? "10-12bit" : "8bit");
 *
 *     // コーデック設定 OBU にアクセス
 *     if (av01->config_obus_size > 0) {
 *         const uint8_t *config_data = av01->config_obus;
 *         uint32_t config_size = av01->config_obus_size;
 *         // 設定 OBU を処理...
 *     }
 * }
 * ```
 */
typedef struct Mp4SampleEntryAv01 {
  uint16_t width;
  uint16_t height;
  uint8_t seq_profile;
  uint8_t seq_level_idx_0;
  uint8_t seq_tier_0;
  uint8_t high_bitdepth;
  uint8_t twelve_bit;
  uint8_t monochrome;
  uint8_t chroma_subsampling_x;
  uint8_t chroma_subsampling_y;
  uint8_t chroma_sample_position;
  bool initial_presentation_delay_present;
  uint8_t initial_presentation_delay_minus_one;
  const uint8_t *config_obus;
  uint32_t config_obus_size;
} Mp4SampleEntryAv01;

/**
 * Opus 音声コーデック用のサンプルエントリー
 *
 * Opus 音声コーデックの詳細情報を保持する構造体で、
 * チャンネル数、サンプルレート、サンプルサイズ、
 * およびOpus固有のパラメータなどが含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * 基本的な使用例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_OPUS) {
 *     Mp4SampleEntryOpus *opus = &entry.data.opus;
 *     printf("チャンネル数: %d\n", opus->channel_count);
 *     printf("サンプルレート: %d Hz\n", opus->sample_rate);
 *     printf("プリスキップ: %d サンプル\n", opus->pre_skip);
 *     printf("入力サンプルレート: %d Hz\n", opus->input_sample_rate);
 *     printf("出力ゲイン: %d dB\n", opus->output_gain);
 * }
 * ```
 */
typedef struct Mp4SampleEntryOpus {
  uint8_t channel_count;
  uint16_t sample_rate;
  uint16_t sample_size;
  uint16_t pre_skip;
  uint32_t input_sample_rate;
  int16_t output_gain;
} Mp4SampleEntryOpus;

/**
 * MP4A（AAC）音声コーデック用のサンプルエントリー
 *
 * AAC 音声コーデックの詳細情報を保持する構造体で、
 * チャンネル数、サンプルレート、サンプルサイズ、バッファサイズ、ビットレート情報、
 * およびデコーダ固有情報などが含まれる
 *
 * 各フィールドの詳細については MP4 やコーデックの仕様を参照のこと
 *
 * # 使用例
 *
 * 基本的な使用例:
 * ```c
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_MP4A) {
 *     Mp4SampleEntryMp4a *mp4a = &entry.data.mp4a;
 *     printf("チャンネル数: %d\n", mp4a->channel_count);
 *     printf("サンプルレート: %d Hz\n", mp4a->sample_rate);
 *     printf("サンプルサイズ: %d bits\n", mp4a->sample_size);
 *     printf("最大ビットレート: %d bps\n", mp4a->max_bitrate);
 *     printf("平均ビットレート: %d bps\n", mp4a->avg_bitrate);
 *
 *     // デコーダ固有情報にアクセス
 *     if (mp4a->dec_specific_info_size > 0) {
 *         const uint8_t *dec_info = mp4a->dec_specific_info;
 *         uint32_t dec_info_size = mp4a->dec_specific_info_size;
 *         // デコーダ固有情報を処理...
 *     }
 * }
 * ```
 */
typedef struct Mp4SampleEntryMp4a {
  uint8_t channel_count;
  uint16_t sample_rate;
  uint16_t sample_size;
  uint32_t buffer_size_db;
  uint32_t max_bitrate;
  uint32_t avg_bitrate;
  const uint8_t *dec_specific_info;
  uint32_t dec_specific_info_size;
} Mp4SampleEntryMp4a;

/**
 * MP4 サンプルエントリーの詳細データを格納するユニオン型
 *
 * このユニオン型は、`Mp4SampleEntry` の `kind` フィールドで指定されたコーデック種別に応じて、
 * 対応する構造体へのアクセスを提供する
 */
typedef union Mp4SampleEntryData {
  /**
   * AVC1（H.264）コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryAvc1 avc1;
  /**
   * HEV1（H.265/HEVC）コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryHev1 hev1;
  /**
   * VP08（VP8）コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryVp08 vp08;
  /**
   * VP09（VP9）コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryVp09 vp09;
  /**
   * AV01（AV1）コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryAv01 av01;
  /**
   * Opus 音声コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryOpus opus;
  /**
   * MP4A（AAC）音声コーデック用のサンプルエントリー
   */
  struct Mp4SampleEntryMp4a mp4a;
} Mp4SampleEntryData;

/**
 * MP4 サンプルエントリー
 *
 * MP4 ファイル内で使用されるメディアサンプル（フレーム単位の音声または映像データ）の
 * 詳細情報を保持する構造体
 *
 * 各サンプルはコーデック種別ごとに異なる詳細情報を持つため、
 * この構造体は `kind` フィールドでコーデック種別を識別し、
 * `data` ユニオンフィールドで対応するコーデック固有の詳細情報にアクセスする設計となっている
 *
 * # サンプルエントリーとは
 *
 * サンプルエントリー（Sample Entry）は、MP4 ファイル形式において、
 * メディアサンプル（動画フレームや音声フレーム）の属性情報を定義するメタデータである
 *
 * MP4 ファイルの各トラック内には、使用されるすべての異なるコーデック設定に対応する
 * サンプルエントリーが格納される
 *
 * サンプルデータ自体はこのサンプルエントリーを参照することで、
 * どのコーデックを使用し、どのような属性を持つかが定義される
 *
 * # 使用例
 *
 * ```c
 * // AVC1（H.264）コーデック用のサンプルエントリーを作成し、
 * // その詳細情報にアクセスする例
 * Mp4SampleEntry entry = // ...;
 *
 * if (entry.kind == MP4_SAMPLE_ENTRY_KIND_AVC1) {
 *     Mp4SampleEntryAvc1 *avc1 = &entry.data.avc1;
 *     printf("解像度: %dx%d\n", avc1->width, avc1->height);
 *     printf("プロファイル: %d\n", avc1->avc_profile_indication);
 * }
 * ```
 */
typedef struct Mp4SampleEntry {
  /**
   * このサンプルエントリーで使用されているコーデックの種別
   *
   * この値によって、`data` ユニオンフィールド内のどのメンバーが有効であるかが決まる
   *
   * 例えば、`kind` が `MP4_SAMPLE_ENTRY_KIND_AVC1` である場合、
   * `data.avc1` メンバーにアクセス可能であり、その他のメンバーはアクセス不可となる
   */
  enum Mp4SampleEntryKind kind;
  /**
   * コーデック種別に応じた詳細情報を保持するユニオン
   *
   * `kind` で指定されたメンバー以外にアクセスすると未定義動作となるため、
   * 必ず事前に `kind` フィールドを確認してからアクセスすること
   */
  union Mp4SampleEntryData data;
} Mp4SampleEntry;

/**
 * MP4 デマルチプレックス処理によって抽出されたメディアサンプルを表す構造体
 *
 * MP4 ファイル内の各サンプル（フレーム単位の音声または映像データ）のメタデータと
 * ファイル内の位置情報を保持する
 *
 * この構造体が参照しているポインタのメモリ管理が `Mp4FileDemuxer` が行っており、
 * `Mp4FileDemuxer` インスタンスが破棄されるまでは安全に参照可能である
 */
typedef struct Mp4DemuxSample {
  /**
   * サンプルが属するトラックの情報へのポインタ
   *
   * このポインタの参照先には `Mp4FileDemuxer` インスタンスが有効な間のみアクセス可能である
   */
  const struct Mp4DemuxTrackInfo *track;
  /**
   * サンプルの詳細情報（コーデック設定など）へのポインタ
   *
   * このポインタの参照先には `Mp4FileDemuxer` インスタンスが有効な間のみアクセス可能である
   */
  const struct Mp4SampleEntry *sample_entry;
  /**
   * トラック内でユニークなサンプルエントリーのインデックス番号
   *
   * この値を使用して、複数のサンプルが同じコーデック設定を使用しているかどうかを
   * 簡単に判定できる
   */
  uint32_t sample_entry_index;
  /**
   * このサンプルがキーフレームであるかの判定
   *
   * `true` の場合、このサンプルはキーフレームであり、このポイントから復号を開始できる
   *
   * 音声の場合には、通常はすべてのサンプルがキーフレーム扱いとなる
   */
  bool keyframe;
  /**
   * サンプルのタイムスタンプ（タイムスケール単位）
   *
   * 実際の時間（秒単位）を得るには、この値を対応する `Mp4DemuxTrackInfo` の
   * `timescale` で除算すること
   */
  uint64_t timestamp;
  /**
   * サンプルの尺（タイムスケール単位）
   *
   * 実際の時間（秒単位）を得るには、この値を対応する `Mp4DemuxTrackInfo` の
   * `timescale` で除算すること
   */
  uint32_t duration;
  /**
   * ファイル内におけるサンプルデータの開始位置（バイト単位）
   *
   * 実際のサンプルデータへアクセスするには、この位置から `data_size` 分のバイト列を
   * 入力ファイルから読み込む必要がある
   */
  uint64_t data_offset;
  /**
   * サンプルデータのサイズ（バイト単位）
   *
   * `data_offset` から `data_offset + data_size` までの範囲がサンプルデータとなる
   */
  uintptr_t data_size;
} Mp4DemuxSample;

typedef struct Mp4MuxSample {
  enum Mp4TrackKind track_kind;
  const struct Mp4SampleEntry *sample_entry;
  bool keyframe;
  uint64_t duration_micros;
  uint64_t data_offset;
  uint32_t data_size;
} Mp4MuxSample;

/**
 * 新しい `Mp4FileDemuxer` インスタンスを作成して、それへのポインタを返す
 *
 * この関数が返したポインタは、使用後に `mp4_file_demuxer_free()` で破棄する必要がある
 *
 * # 使用例
 *
 * ```c
 * // Mp4FileDemuxer インスタンスを生成
 * Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
 * if (demuxer == NULL) {
 *     fprintf(stderr, "Failed to create demuxer\n");
 *     return;
 * }
 *
 * // 処理を実行...
 *
 * // リソース解放
 * mp4_file_demuxer_free(demuxer);
 * ```
 */
struct Mp4FileDemuxer *mp4_file_demuxer_new(void);

/**
 * `Mp4FileDemuxer` インスタンスを破棄して、割り当てられたリソースを解放する
 *
 * この関数は、`mp4_file_demuxer_new()` で作成された `Mp4FileDemuxer` インスタンスを破棄し、
 * その内部で割り当てられたすべてのメモリを解放する。
 *
 * # 引数
 *
 * - `demuxer`: 破棄する `Mp4FileDemuxer` インスタンスへのポインタ
 *   - NULL ポインタが渡された場合、この関数は何もしない
 */
void mp4_file_demuxer_free(struct Mp4FileDemuxer *demuxer);

/**
 * `Mp4FileDemuxer` で最後に発生したエラーのメッセージを取得する
 *
 * この関数は、デマルチプレックス処理中に発生した最後のエラーのメッセージ（NULL 終端）を返す
 *
 * エラーが発生していない場合は NULL ポインタを返す
 *
 * # 引数
 *
 * - `demuxer`: `Mp4FileDemuxer` インスタンスへのポインタ
 *
 * # 戻り値
 *
 *
 * - メッセージが存在する場合: NULL 終端のエラーメッセージへのポインタ
 * - メッセージが存在しない場合: NULL ポインタ
 * - `demuxer` 引数が NULL の場合: NULL ポインタ
 *
 * # 使用例
 *
 * ```c
 * Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
 *
 * Mp4Error ret = // なんらかの処理;
 *
 * // エラーが発生した場合、メッセージを取得
 * if (ret != MP4_ERROR_OK) {
 *     const char *error_msg = mp4_file_demuxer_get_last_error(demuxer);
 *     if (error_msg != NULL) {
 *         fprintf(stderr, "エラー: %s\n", error_msg);
 *     }
 * }
 * ```
 */
const char *mp4_file_demuxer_get_last_error(const struct Mp4FileDemuxer *demuxer);

/**
 * `Mp4FileDemuxer` で次の処理を進めるために必要な I/O の位置とサイズを取得する
 *
 * この関数は、処理を進めるために必要な I/O がない場合には `out_required_input_size` に 0 を設定して返し、
 * それ以外の場合は、ファイルから読み込む必要があるデータの位置とサイズを出力引数に設定して返す
 *
 * この関数から取得した位置とサイズの情報をもとに、呼び出し元がファイルなどからデータを読み込み、
 * `mp4_file_demuxer_handle_input()` に渡す必要がある
 *
 * なお、現在の `Mp4FileDemuxer` の実装は fragmented MP4 には対応していないため、
 * サンプルの取得に必要なメタデータ（moovボックス）の読み込み（初期化）が終わったら、
 * 以後はこの関数が追加の入力データを要求することはない
 *
 * # 引数
 *
 * - `demuxer`: `Mp4FileDemuxer` インスタンスへのポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *
 * - `out_required_input_position`: 必要なデータの開始位置（バイト単位）を受け取るポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *
 * - `out_required_input_size`: 必要なデータのサイズ（バイト単位）を受け取るポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *   - なお、ここに設定されるサイズはあくまでもヒントであり、厳密に一致したサイズのデータを提供する必要はない
 *     - 通常は、より大きな範囲のデータを一度に渡した方が効率がいい
 *   - 0 が設定された場合は、これ以上の入力データが不要であることを意味する
 *   - -1 が設定された場合は、ファイルの末尾までのデータが必要であることを意味する
 *
 * # 戻り値
 *
 * - `MP4_ERROR_OK`: 正常に処理された
 * - `MP4_ERROR_NULL_POINTER`: 引数として NULL ポインタが渡された
 *
 * # 使用例
 *
 * ```c
 * Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
 * FILE *fp = fopen("sample.mp4", "rb");
 * uint8_t buffer[4096];  // NOTE: 実際には必要なサイズを動的に確保すべき
 *
 * // 初期化が完了するまでループ
 * while (true) {
 *     uint64_t required_pos;
 *     int32_t required_size;
 *     mp4_file_demuxer_get_required_input(demuxer, &required_pos, &required_size);
 *     if (required_size == 0) break; // 初期化完了
 *
 *     // ファイルから必要なデータを読み込む
 *     //
 *     // NOTE: `required_size == -1` の場合には、実際にはファイル末尾までを読み込む必要がある
 *     size_t read_size = (required_size > 0) ? required_size : sizeof(buffer);
 *     fseek(fp, required_pos, SEEK_SET);
 *     size_t bytes_read = fread(buffer, 1, read_size, fp);
 *
 *     // demuxer にデータを供給
 *     mp4_file_demuxer_handle_input(demuxer, required_pos, buffer, bytes_read);
 * }
 * ```
 */
enum Mp4Error mp4_file_demuxer_get_required_input(struct Mp4FileDemuxer *demuxer,
                                                  uint64_t *out_required_input_position,
                                                  int32_t *out_required_input_size);

/**
 * `Mp4FileDemuxer` にファイルデータを入力として供給し、デマルチプレックス処理を進める
 *
 * この関数は、`mp4_file_demuxer_get_required_input()` で取得した位置に対応するファイルデータを
 * 受け取り、デマルチプレックス処理を進める
 *
 * なお、この関数はデータの部分的な消費を行わないため、呼び出し元が必要なデータを一度に全て渡す必要がある
 * （固定長のバッファを使って複数回に分けてデータを供給することはできない）
 *
 * # 引数
 *
 * - `demuxer`: `Mp4FileDemuxer` インスタンスへのポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *
 * - `input_position`: 入力データがファイル内で始まる位置（バイト単位）
 *   - `mp4_file_demuxer_get_required_input()` で取得した位置と一致していることが期待される
 *
 * - `input_data`: ファイルデータのバッファへのポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *
 * - `input_data_size`: 入力データのサイズ（バイト単位）
 *   - 0 以上の値を指定する必要がある
 *   - `mp4_file_demuxer_get_required_input()` で取得したサイズより大きいサイズを指定することは問題ない
 *
 * # 戻り値
 *
 * - `MP4_ERROR_OK`: 正常に入力データが受け取られた
 *   - この場合でも `mp4_file_demuxer_get_required_input()` を使って、追加の入力が必要かどうかを確認する必要がある
 * - `MP4_ERROR_NULL_POINTER`: 引数として NULL ポインタが渡された
 */
enum Mp4Error mp4_file_demuxer_handle_input(struct Mp4FileDemuxer *demuxer,
                                            uint64_t input_position,
                                            const uint8_t *input_data,
                                            uint32_t input_data_size);

/**
 * MP4 ファイル内に含まれるすべてのメディアトラック（音声および映像）の情報を取得する
 *
 * # 引数
 *
 * - `demuxer`: `Mp4FileDemuxer` インスタンスへのポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *
 * - `out_tracks`: 取得したトラック情報の配列へのポインタを受け取るポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *   - このポインタの参照先には `Mp4FileDemuxer` インスタンスが有効な間のみアクセス可能である
 *
 * - `out_track_count`: トラック情報の個数を受け取るポインタ
 *   - NULL ポインタが渡された場合、`MP4_ERROR_NULL_POINTER` が返される
 *   - MP4 ファイルにトラックが含まれていない場合は 0 が設定される
 *
 * # 戻り値
 *
 * - `MP4_ERROR_OK`: 正常にトラック情報が取得された
 * - `MP4_ERROR_NULL_POINTER`: 引数として NULL ポインタが渡された
 * - `MP4_ERROR_INPUT_REQUIRED`: 初期化に必要な入力データが不足している
 *   - `mp4_file_demuxer_get_required_input()` および `mp4_file_demuxer_handle_input()` のハンドリングが必要
 * - `MP4_ERROR_INVALID_DATA`: MP4 ファイルが破損しているか無効な形式である
 *
 * # 使用例
 *
 * ```c
 * Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
 *
 * // ファイルデータを供給（省略）
 * // ...
 *
 * // トラック情報を取得
 * const Mp4DemuxTrackInfo *tracks;
 * uint32_t track_count;
 * Mp4Error ret = mp4_file_demuxer_get_tracks(demuxer, &tracks, &track_count);
 *
 * if (ret == MP4_ERROR_OK) {
 *     printf("Found %u tracks\n", track_count);
 *     for (uint32_t i = 0; i < track_count; i++) {
 *         printf("Track %u: ID=%u, Kind=%d, Duration=%lu, Timescale=%u\n",
 *                i, tracks[i].track_id, tracks[i].kind,
 *                tracks[i].duration, tracks[i].timescale);
 *     }
 * } else {
 *     fprintf(stderr, "Error: %d\n", ret);
 * }
 * ```
 */
enum Mp4Error mp4_file_demuxer_get_tracks(struct Mp4FileDemuxer *demuxer,
                                          const struct Mp4DemuxTrackInfo **out_tracks,
                                          uint32_t *out_track_count);

enum Mp4Error mp4_file_demuxer_next_sample(struct Mp4FileDemuxer *demuxer,
                                           struct Mp4DemuxSample *out_sample);

uint32_t mp4_estimate_maximum_moov_box_size(uint32_t audio_sample_count,
                                            uint32_t video_sample_count);

struct Mp4FileMuxer *mp4_file_muxer_new(void);

void mp4_file_muxer_free(struct Mp4FileMuxer *muxer);

const char *mp4_file_muxer_get_last_error(const struct Mp4FileMuxer *muxer);

enum Mp4Error mp4_file_muxer_set_reserved_moov_box_size(struct Mp4FileMuxer *muxer, uint64_t size);

enum Mp4Error mp4_file_muxer_set_creation_timestamp(struct Mp4FileMuxer *muxer,
                                                    uint64_t timestamp_micros);

enum Mp4Error mp4_file_muxer_initialize(struct Mp4FileMuxer *muxer);

enum Mp4Error mp4_file_muxer_next_output(struct Mp4FileMuxer *muxer,
                                         uint64_t *out_output_offset,
                                         uint32_t *out_output_size,
                                         const uint8_t **out_output_data);

enum Mp4Error mp4_file_muxer_append_sample(struct Mp4FileMuxer *muxer,
                                           const struct Mp4MuxSample *sample);

enum Mp4Error mp4_file_muxer_finalize(struct Mp4FileMuxer *muxer);

#endif  /* SHIGUREDO_MP4_H */
