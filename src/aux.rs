//! MP4 の仕様とは直接は関係がない、実装上便利な補助的なコンポーネントを集めたモジュール

use std::num::NonZeroU32;

use crate::{
    boxes::{SampleEntry, StblBox, StscEntry, StszBox},
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

    /// 指定されたサンプルディスクリプション（サンプルエントリー）を返す
    ///
    /// 存在しないサンプルディスクリプションが指定された場合には [`None`] が返される
    pub fn sample_description(&self, sample_description_index: NonZeroU32) -> Option<&SampleEntry> {
        self.stbl_box
            .stsd_box
            .entries
            .get(sample_description_index.get() as usize - 1)
    }

    /// このトラック内のチャンク一覧を返す
    ///
    /// [`StscBox`] の中身に不整合がある場合には [`None`] が返される
    pub fn chunks(&self) -> Option<Vec<ExpandedStscEntry>> {
        let mut chunks = Vec::new();
        let mut chunk_end = self.chunk_count();
        let mut sample_end = self.sample_count;
        for StscEntry {
            first_chunk,
            sample_per_chunk,
            sample_description_index,
        } in self.stbl_box.stsc_box.entries.iter().cloned().rev()
        {
            let chunk_start = first_chunk.get() - 1;
            for chunk in (chunk_start..chunk_end).rev() {
                let sample_start = sample_end.checked_sub(sample_per_chunk)?;
                chunks.push(ExpandedStscEntry {
                    chunk_index: NonZeroU32::MIN.saturating_add(chunk),
                    sample_description_index,
                    sample_index_offset: NonZeroU32::MIN.saturating_add(sample_start),
                    sample_count: sample_per_chunk,
                });
                sample_end = sample_start;
            }
            chunk_end = chunk_start;
        }
        chunks.reverse();
        Some(chunks)
    }
}

/// [`StscEntry`] を展開して、特定のチャンクに対応する情報を保持するようにした構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpandedStscEntry {
    /// チャンクインデックス
    pub chunk_index: NonZeroU32,

    /// このチャンクが参照するサンプルエントリーのインデックス
    pub sample_description_index: NonZeroU32,

    /// このチャンクが属する最初のサンプルのインデックス
    pub sample_index_offset: NonZeroU32,

    /// このチャンクに属するサンプルの数
    pub sample_count: u32,
}

impl ExpandedStscEntry {
    /// このチャンクに属するサンプル群のインデックスを走査するイテレーターを返す
    pub fn sample_indices(&self) -> impl '_ + Iterator<Item = NonZeroU32> {
        (0..self.sample_count).map(|i| self.sample_index_offset.saturating_add(i))
    }
}
