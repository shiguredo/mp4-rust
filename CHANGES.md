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
  -  @sile
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
