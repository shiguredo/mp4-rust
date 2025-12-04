# 変更履歴

- UPDATE
  - 後方互換がある変更
- ADD
  - 後方互換がある追加
- CHANGE
  - 後方互換のない変更
- FIX
  - バグ修正

## develop

- [ADD] FLAC 対応を追加する
  - FLAC を扱うのに必要な以下のボックスへの対応を追加する:
    - fLaC
    - dfLa
  - @sile
- [CHANGE] MinfBox 構造体の smhd_or_vmhd_box フィールドを Option 型に変更する
  - メディアトラック以外を含む MP4 ファイルの読み込みに対応するための変更
  - @sile
- [FIX] メディアトラック以外を含む MP4 ファイルの読み込みに失敗する問題を修正する
  - minf ボックスの中には smhd ボックス（音声）ないし vmhd ボックス（映像）が必ず存在する想定の実装となっていたため、そうではない場合にエラーになっていた
  - メディアトラック以外の場合には、minf ボックスがこれらを含まないため、それを許容するようにする
  - @sile
- [FIX] vmhd ボックスのデコード時にフラグの値チェックを緩和する
  - ISO/IEC 14496-12 の仕様では vmhd ボックスのフラグの値は 1 になると記載されているが、実装によっては 0 となるファイルも存在するため、このチェックは行わないようにする
  - @sile
- [FIX] AAC の MP4 読み込み時に DecoderConfigDescriptor が存在しないとエラーになる問題を修正する
  - ISO/IEC 14496-1 の仕様としては DecoderConfigDescriptor はオプショナルだが、実装が必須扱いになっていたのでエラーとなっていた
  - 仕様に合わせてオプショナル扱いとするように実装を修正する
  - @sile
- [UPDATE] avcC ボックスのデコード時にペイロード境界チェックを追加する
  - SPS/PPS/SPS EXT データ読み込み時にオフセットがペイロード範囲内にあるかをチェックし、範囲外ならエラーにする
  - @sile
- [FIX] H.264 の MP4 ファイルの読み込み時に、仕様に準拠しない avcC ボックスのデコードでエラーになる問題を修正する
  - H.264 のプロファイルが 66 | 77 | 88 以外の場合、ISO/IEC 14496-15 の仕様では avcC ボックスの末尾に追加フィールドが存在することになっている
  - しかし MP4 ファイル作成ライブラリやツールによっては、これを省略する実装が存在するため、ボックスのペイロード終端に達した場合は追加フィールドの処理をスキップするようにした
  - @sile
- [FIX] sans-I/O 対応の際にエラーメッセージに載る情報が不十分になっていたのを修正する
  - デコードエラー時に「どのボックスでエラーが発生したか」の情報を載せるようにする
  - エラー発生行番号が、新規に追加した共通関数（`Decode::decode_at()`）の中の位置ではなく、その呼び出し元の位置になるようにする
  - @sile

### misc

- [ADD] examples/demux.rs を追加する
  - @sile
- [ADD] Windows と macOS を CI 対象に追加する
  - @sile

## 2025.3.0

- [ADD] C 言語バインディングを追加する
  - MP4 ファイルのマルチプレックス・デマルチプレックス機能を C 言語から利用するための API を提供する `crates/c-api/` を追加した
    - このクレートは別の Rust ライブラリ から利用されることを想定していないため、crates-io には登録しない
  - `mp4_file_demuxer_*` 関数群により、MP4 ファイルの読み込みと時系列順のサンプル抽出が可能になった
  - `mp4_file_muxer_*` 関数群により、複数のメディアトラックからサンプルを統合して MP4 ファイルを構築できるようになった
  - サンプルプログラム（`examples/demux.c`, `examples/remux.c`）とテストプログラム（`tests/simple_mux_demux.c`）を追加した
  - @sile
- [ADD] MP4 ファイルのマルチプレックス機能を追加する
  - 複数のメディアトラック（音声・映像）からのサンプルを時系列順に統合して、MP4 ファイルを構築するための `mux` モジュールを追加した
  - 新しく追加された `Mp4FileMuxer` 構造体により、段階的にサンプルを追加して MP4 ファイルを構築できる
  - I/O 操作に依存しない設計で、ファイル書き込みは利用側で実施する
  - @sile
- [ADD] MP4 ファイルのデマルチプレックス機能を追加する
  - MP4 ファイルから複数のメディアトラック（音声・映像）内のサンプル群を時系列順に分離して抽出するための `demux` モジュールを追加した
  - 新しく追加された `Mp4FileDemuxer` 構造体により、段階的にファイルデータを処理し、サンプルを順序付けて取得できる
  - I/O 操作に依存しない設計で、ファイル読み込みは利用側で実施する
  - @sile
- [ADD] no_std 環境のサポートを追加する
  - `default-features = false` を指定することで no_std 環境でも利用可能になった
  - std 環境がデフォルトなので、既存のコードへの影響はない
  - @voluntas
- [CHANGE] `MdatBox::is_variable_size` フィールドを削除する
  -  4 GB までのペイロードしか扱えず中途半端だったので、`MdatBox` 構造体から `is_variable_size` フィールドを削除した
  - 今後は可変長ペイロードを表現する場合は、`MdatBox` ではなく [`BoxHeader`] を直接使用する必要がある
  - @sile
- [CHANGE] IgnoredBox 構造体を削除する
  - この構造体は Decode トレイトの古い設計前提であったので、設計変更に伴い不要となった
  - @sile
- [CHANGE] Error 構造体の std::io::Error への依存をなくす（sans-I/O 対応）
  - std::io モジュールへの依存をなくしたのに伴い、独自の ErrorKind enum を定義し、使用するようにした
  - @sile
- [CHANGE] Encode および Decode トレイトを I/O に依存しない設計に変更する（sans-I/O 対応）
  - モチベーション: no_std / wasm / C API に対応する際に、I/O と密結合になっていると取り回しが難しいので、mp4-rust レイヤーでは I/O に依存しないようにする
  - std::io::{Read, Write} に対してではなく、バッファ（&[u8]）に対して操作を行うように変更した
  - @sile

## 2025.2.0

- [FIX] Windows でリポジトリの clone に失敗する問題を修正する
  - Windows での予約ファイル名に衝突する `aux.rs` がリポジトリに含まれていたのが原因だった
  - ファイル名を `auxiliary.rs` に変更した上で、その中身を `lib.rs` の中でインラインで定義された `aux` モジュールに再エクスポートすることで対応した
    - 外部インターフェースへの変更は発生しないので、以前のバージョンとの互換性は維持されている
  - @sile
- [CHANGE] 最小サポート Rust バージョンを 1.88 に設定する
  - `let-else` 構文を使い始めたため
  - @sile

### misc

- [UPDATE] GitHub Actions の ci.yml で使用する Ubuntu のバージョンを 24.04 に固定する
  - @voluntas
- [UPDATE] clippy 0.1.89 に対応する
  - @sile
- [UPDATE] clippy 0.1.88 に対応する
  - @sile
- [UPDATE] actions/checkout を v5 に上げる
  - @miosakuma
- [ADD] GitHub Actions の ci.yml を平日 10:00 (JST) に実行するようにする
  - @voluntas

## 2025.1.0

- [ADD] AAC 関連の定数を追加する
  - MP4 に AAC ストリームを格納する際に、典型的に使用される値を以下の定数として定義した:
    - `EsDescriptor::MIN_ES_ID`
    - `EsDescriptor::LOWEST_STREAM_PRIORITY`
    - `DecoderConfigDescriptor::OBJECT_TYPE_INDICATION_AUDIO_ISO_IEC_14496_3`
    - `DecoderConfigDescriptor::STREAM_TYPE_AUDIO`
    - `DecoderConfigDescriptor::UP_STREAM_FALSE`
  - @sile
- [ADD] `AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX` 定数を追加する
  - 通常の用途では `data_reference_index` には常にこの定数値が設定されることになる
  - @sile
- [CHANGE] `AudioSampleEntryFields.data_reference_index` の型を `u16` から `NonZeroU16` に変更する
  - 値が 0 になることはないため non zero 版に変更した
    - `VisualSampleEntryFields.data_reference_index` は元々 `NonZeroU16` だったので、両者の齟齬の解消も兼ねている
  - @sile
- [CHANGE] Rust のエディションを 2021 から 2024 に上げる
  - @sile
- [FIX] ディスクリプターのサイズがリトルエンディアンでエンコードされていたのを修正する
  - @sile

## 2024.4.0

- [ADD] AAC 用のボックスを追加する
  - @sile
- [UPDATE] `ChunkAccessor` と `SampleAccessor` の一部のメソッドのライフタイム制約が必要以上に厳しかったのを緩くする
  - @sile

## 2024.3.0

- [CHANGE] `Encode::encode()` が `writer: &mut W` ではなく `writer: W` を引数に取るように変更する
  - @sile
- [CHANGE] `Decode::decode()` が `reader: &mut R` ではなく `reader: R` を引数に取るように変更する
  - @sile
- [ADD] デコード時にペイロードデータを保持しない `IgnoredBox` を追加する
  - @sile
- [ADD] `SampleTableAccessor::get_sample_by_timestamp()` を追加する
  - @sile
- [ADD] `SampleAccessor::timestamp()` を追加する
  - @sile
- [ADD] `SampleAccessor::sync_sample()` を追加する
  - @sile
- [CHANGE] `SampleTableAccessor::new()` で stco ボックスと stsc ボックスの不整合をチェックするようにする
  - @sile
- [UPDATE] `SampleTableAccessor` が borrowed / owned の両方に対応できるようにする
  - @sile
- [UPDATE] 共通関数でエラーが発生した場合のファイル名・行番号表示を改善する
  - 今までは共通関数のエラー位置が `Error` に含まれていたが、それでは情報量が少ないので、その一つ上の呼び出し元の位置を使うように変更した
  - @sile

## 2024.2.0

- [ADD] WebCodecs を使ってローカルで MP4 ファイルを変換するサンプルを追加する
  - @sile
- [ADD] `StblBox` の情報へのアクセスを簡単かつ安全にするための `SampleTableAccessor` 構造体を追加する
  - @sile
- [ADD] `SttsBox::from_sample_deltas()` 関数を追加する
  - @sile
- [ADD] `Utf8String::into_null_terminated_bytes()` メソッドを追加する
  - @sile
- [CHANGE] 仕様上 0 を取らないフィールドの型は `NonZeroXXX` にする
  - @sile
- [UPDATE] ボックスに `Hash` を実装する
  - @sile
- [CHANGE] `BoxHeader` 書き込み時に large size にするかどうかの自動判定は行わないようにする
  - `BoxSize` 自体はどちらを使うべきかの情報を有しているので、それをそのまま反映するようにした
  - @sile
- [UPDATE] `Error` 構造体にエラー発生箇所特定用のフィールドを追加する
  - エラー発生時のボック種別、および、エラー発生ファイルと行番号、の情報を取得できるようにした
  - @sile
- [FIX] hdlr ボックスの name フィールドは単なるバイト列として扱うようにする
  - ISO の仕様上は、このフィールドは null 終端の UTF-8 文字列であるべきだが、それに準拠しない MP4 ファイルを生成する実装が普通に存在するため、中身を厳密にチェックしないようにした
  - @sile
- [FIX] 64 bit のボックスサイズが使われていた場合にペイロードのデコードに失敗する問題を修正する
  - @sile

## 2024.1.0

**公開**
