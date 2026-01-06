# MP4 ライブラリ WebAssembly API

MP4 ファイルの読み込み（デマルチプレックス）と書き込み（マルチプレックス）を行うための WebAssembly API です。

FFI 方式で実装されており、JavaScript/TypeScript から直接呼び出すことができます。

## ビルド方法

```bash
# WebAssembly ターゲットをインストール（初回のみ）
rustup target add wasm32-unknown-unknown

# ビルド
cargo build -p wasm --target wasm32-unknown-unknown --release

# 出力ファイル: target/wasm32-unknown-unknown/release/wasm.wasm
```

### サイズ最適化

[wasm-opt](https://github.com/WebAssembly/binaryen) を使用してサイズを最適化できます。

```bash
wasm-opt -Oz -o optimized.wasm target/wasm32-unknown-unknown/release/wasm.wasm
```

## 提供する機能

### メモリ管理

- `mp4_wasm_alloc`: メモリ確保
- `mp4_wasm_free`: メモリ解放
- `mp4_wasm_vec_ptr`: Vec のポインタ取得
- `mp4_wasm_vec_len`: Vec の長さ取得
- `mp4_wasm_vec_free`: Vec の解放
- `mp4_wasm_version`: ライブラリバージョン取得

### デマルチプレックス

- `mp4_wasm_demuxer_new`: デマルチプレクサ作成
- `mp4_wasm_demuxer_free`: デマルチプレクサ解放
- `mp4_wasm_demuxer_get_last_error`: エラーメッセージ取得
- `mp4_wasm_demuxer_get_required_input_position`: 必要な入力位置取得
- `mp4_wasm_demuxer_get_required_input_size`: 必要な入力サイズ取得
- `mp4_wasm_demuxer_handle_input`: 入力データ供給
- `mp4_wasm_demuxer_get_track_count`: トラック数取得
- `mp4_wasm_demuxer_get_track`: トラック情報取得
- `mp4_wasm_demuxer_next_sample`: 次のサンプル取得
- `mp4_wasm_sample_entry_to_json`: サンプルエントリを JSON で取得

### マルチプレックス

- `mp4_wasm_estimate_maximum_moov_box_size`: moov ボックスサイズ見積もり
- `mp4_wasm_muxer_new`: マルチプレクサ作成
- `mp4_wasm_muxer_free`: マルチプレクサ解放
- `mp4_wasm_muxer_get_last_error`: エラーメッセージ取得
- `mp4_wasm_muxer_set_reserved_moov_box_size`: faststart 用 moov サイズ設定
- `mp4_wasm_muxer_initialize`: 初期化
- `mp4_wasm_muxer_has_output`: 出力データ有無確認
- `mp4_wasm_muxer_get_output_offset`: 出力オフセット取得
- `mp4_wasm_muxer_get_output_size`: 出力サイズ取得
- `mp4_wasm_muxer_get_output_ptr`: 出力ポインタ取得
- `mp4_wasm_muxer_advance_output`: 次の出力へ進む
- `mp4_wasm_muxer_append_sample`: サンプル追加
- `mp4_wasm_muxer_finalize`: 完了処理

## JSON 形式のサンプルエントリ

`mp4_wasm_sample_entry_to_json` を使用すると、複雑なポインタ操作なしでサンプルエントリ情報を JSON 文字列として取得できます。

バイナリデータ（SPS/PPS/NALU 等）は Base64 エンコードされます。

### AVC1 (H.264) の例

```json
{
  "kind": "avc1",
  "width": 1920,
  "height": 1080,
  "avcProfileIndication": 100,
  "profileCompatibility": 0,
  "avcLevelIndication": 40,
  "lengthSizeMinusOne": 3,
  "sps": ["Z2QAH62..."],
  "pps": ["aOvjYA=="]
}
```

### HEV1/HVC1 (H.265/HEVC) の例

```json
{
  "kind": "hev1",
  "width": 1920,
  "height": 1080,
  "generalProfileSpace": 0,
  "generalTierFlag": 0,
  "generalProfileIdc": 1,
  "generalProfileCompatibilityFlags": 1610612736,
  "generalConstraintIndicatorFlags": 144115188075855872,
  "generalLevelIdc": 120,
  "chromaFormatIdc": 1,
  "bitDepthLumaMinus8": 0,
  "bitDepthChromaMinus8": 0,
  "minSpatialSegmentationIdc": 0,
  "parallelismType": 0,
  "avgFrameRate": 0,
  "constantFrameRate": 0,
  "numTemporalLayers": 1,
  "temporalIdNested": 1,
  "lengthSizeMinusOne": 3,
  "naluArrays": [
    {"type": 32, "nalus": ["QAE..."]},
    {"type": 33, "nalus": ["QgE..."]},
    {"type": 34, "nalus": ["RAE..."]}
  ]
}
```

### Opus の例

```json
{
  "kind": "opus",
  "channelCount": 2,
  "sampleRate": 48000,
  "sampleSize": 16,
  "preSkip": 312,
  "inputSampleRate": 48000,
  "outputGain": 0
}
```

### MP4A (AAC) の例

```json
{
  "kind": "mp4a",
  "channelCount": 2,
  "sampleRate": 48000,
  "sampleSize": 16,
  "bufferSizeDb": 0,
  "maxBitrate": 128000,
  "avgBitrate": 128000,
  "decSpecificInfo": "EZBt..."
}
```
