//! MP4 の仕様とは直接は関係がない、実装上便利な補助的なコンポーネントを集めたモジュール

use std::num::NonZeroU32;

use crate::{
    boxes::{StblBox, StszBox},
    Either,
};

/// [`StblBox`] をラップして、その中の情報を簡単かつ効率的に取り出せるようにするための構造体
#[derive(Debug)]
pub struct SampleTableAccessor<'a> {
    stbl_box: &'a StblBox,
    sample_count: u32,
    stts_table: Vec<(u32, u32)>, // (累計サンプル数、尺）
}

impl<'a> SampleTableAccessor<'a> {
    /// 引数で渡された [`StblBox`] 用の [`SampleTableAccessor`] インスタンスを生成する
    pub fn new(stbl_box: &'a StblBox) -> Self {
        let mut stts_table = Vec::new();
        let mut sample_count = 0;
        for entry in &stbl_box.stts_box.entries {
            stts_table.push((sample_count, entry.sample_delta));
            sample_count += entry.sample_count;
        }

        Self {
            stbl_box,
            sample_count,
            stts_table,
        }
    }

    /// トラック内のサンプルの数を取得する
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// トラック内のチャンクの数を取得する
    pub fn chunk_count(&self) -> u32 {
        match &self.stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets.len() as u32,
            Either::B(b) => b.chunk_offsets.len() as u32,
        }
    }

    /// 指定されたサンプルの尺を取得する
    ///
    /// 存在しないサンプルが指定された場合には [`None`] が返される
    pub fn sample_duration(&self, sample_index: NonZeroU32) -> Option<u32> {
        if self.sample_count < sample_index.get() {
            return None;
        }

        let i = self
            .stts_table
            .binary_search_by_key(&(sample_index.get() - 1), |x| x.0)
            .map(|i| i)
            .unwrap_or_else(|i| i);
        self.stts_table.get(i).map(|x| x.1)
    }

    /// 指定されたサンプルのデータサイズ（バイト数）を取得する
    ///
    /// 存在しないサンプルが指定された場合には [`None`] が返される
    pub fn sample_size(&self, sample_index: NonZeroU32) -> Option<u32> {
        if self.sample_count < sample_index.get() {
            return None;
        }

        let i = sample_index.get() as usize - 1;
        match &self.stbl_box.stsz_box {
            StszBox::Fixed { sample_size, .. } => Some(sample_size.get()),
            StszBox::Variable { entry_sizes } => entry_sizes.get(i).copied(),
        }
    }

    /// 指定されたサンプルが同期サンプルかどうかを判定する
    ///
    /// 存在しないサンプルが指定された場合には [`None`] が返される
    pub fn is_sync_sample(&self, sample_index: NonZeroU32) -> Option<bool> {
        if self.sample_count < sample_index.get() {
            return None;
        }

        let Some(stss_box) = &self.stbl_box.stss_box else {
            // stss ボックスが存在しない場合は全てが同期サンプル扱い
            return Some(true);
        };

        Some(stss_box.sample_numbers.binary_search(&sample_index).is_ok())
    }

    /// 指定されたチャンクのファイル内でのバイト位置を返す
    ///
    /// 存在しないチャンクが指定された場合には [`None`] が返される
    pub fn chunk_offset(&self, chunk_index: NonZeroU32) -> Option<u64> {
        let i = chunk_index.get() as usize - 1;
        match &self.stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets.get(i).copied().map(|v| v as u64),
            Either::B(b) => b.chunk_offsets.get(i).copied(),
        }
    }
}
