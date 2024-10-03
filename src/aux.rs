//! MP4 の仕様とは直接は関係がない、実装上便利な補助的なコンポーネントを集めたモジュール
use std::num::NonZeroU32;

use crate::{
    boxes::{SampleEntry, StblBox, StscBox, StscEntry, StszBox},
    BoxType, Either,
};

/// [`StblBox`] をラップして、その中の情報を簡単かつ効率的に取り出せるようにするための構造体
#[derive(Debug)]
pub struct SampleTableAccessor<'a> {
    stbl_box: &'a StblBox,
    chunk_count: u32,
    sample_count: u32,
    sample_durations: Vec<(u32, u32)>,     // (累計サンプル数、尺）
    sample_index_offsets: Vec<NonZeroU32>, // チャンク先頭のサンプルインデックス
}

impl<'a> SampleTableAccessor<'a> {
    /// 引数で渡された [`StblBox`] 用の [`SampleTableAccessor`] インスタンスを生成する
    pub fn new(stbl_box: &'a StblBox) -> Result<Self, SampleTableAccessorError> {
        let mut sample_count = 0;
        let mut sample_durations = Vec::new();
        for entry in &stbl_box.stts_box.entries {
            sample_durations.push((sample_count, entry.sample_delta));
            sample_count += entry.sample_count;
        }

        if let StszBox::Variable { entry_sizes } = &stbl_box.stsz_box {
            if entry_sizes.len() != sample_count as usize {
                // stts と stsz でサンプル数が異なる
                return Err(SampleTableAccessorError::InconsistentSampleCount {
                    stts_sample_count: sample_count,
                    other_box_type: StszBox::TYPE,
                    other_sample_count: entry_sizes.len() as u32,
                });
            }
        }

        let chunk_count = match &stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets.len() as u32,
            Either::B(b) => b.chunk_offsets.len() as u32,
        };

        if let Some(x) = stbl_box.stsc_box.entries.first() {
            if x.first_chunk.get() != 1 {
                // チャンクインデックスが 1 以外から始まっている
                return Err(SampleTableAccessorError::FirstChunkIndexIsNotOne {
                    actual_chunk_index: x.first_chunk,
                });
            }
        }
        if let Some(i) = stbl_box.stsc_box.entries.iter().position(|x| {
            stbl_box.stsd_box.entries.len() < x.sample_description_index.get() as usize
        }) {
            // 存在しないサンプルエントリーを参照しているチャンクがある
            return Err(SampleTableAccessorError::MissingSampleEntry {
                stsc_entry_index: i,
                sample_description_index: stbl_box.stsc_box.entries[i].sample_description_index,
                sample_entry_count: stbl_box.stsd_box.entries.len(),
            });
        }
        if stbl_box
            .stsc_box
            .entries
            .iter()
            .zip(stbl_box.stsc_box.entries.iter().skip(1))
            .any(|(prev, next)| prev.first_chunk >= next.first_chunk)
        {
            // stsc 内のチャンクインデックスが短調増加していない
            return Err(SampleTableAccessorError::ChunkIndicesNotMonotonicallyIncreasing);
        }

        let mut sample_index_offsets = Vec::new();
        let mut first_sample_index = NonZeroU32::MIN;
        for i in 0..chunk_count {
            let chunk_index = NonZeroU32::MIN.saturating_add(i);
            sample_index_offsets.push(first_sample_index);

            let j = stbl_box
                .stsc_box
                .entries
                .binary_search_by_key(&chunk_index, |x| x.first_chunk)
                .unwrap_or_else(|j| j - 1);
            first_sample_index =
                first_sample_index.saturating_add(stbl_box.stsc_box.entries[j].sample_per_chunk);
        }
        if first_sample_index.get() - 1 != sample_count {
            // stts と stsc でサンプル数が異なる
            return Err(SampleTableAccessorError::InconsistentSampleCount {
                stts_sample_count: sample_count,
                other_box_type: StscBox::TYPE,
                other_sample_count: first_sample_index.get() - 1,
            });
        }

        Ok(Self {
            stbl_box,
            chunk_count,
            sample_count,
            sample_durations,
            sample_index_offsets,
        })
    }

    /// トラック内のサンプルの数を取得する
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// トラック内のチャンクの数を取得する
    pub fn chunk_count(&self) -> u32 {
        self.chunk_count
    }

    /// 指定されたサンプルの情報を返す
    ///
    /// 存在しないサンプルが指定された場合には [`None`] が返される
    pub fn get_sample(&self, sample_index: NonZeroU32) -> Option<SampleAccessor> {
        (sample_index.get() <= self.sample_count).then_some(SampleAccessor {
            sample_table: self,
            index: sample_index,
        })
    }

    /// 指定されたチャンクの情報を返す
    ///
    /// 存在しないチャンクが指定された場合には [`None`] が返される
    pub fn get_chunk(&self, chunk_index: NonZeroU32) -> Option<ChunkAccessor> {
        (chunk_index.get() <= self.chunk_count()).then_some(ChunkAccessor {
            sample_table: self,
            index: chunk_index,
        })
    }

    /// トラック内のサンプル群の情報を走査するイテレーターを返す
    pub fn samples(&self) -> impl '_ + Iterator<Item = SampleAccessor> {
        (0..self.sample_count()).map(|i| SampleAccessor {
            sample_table: self,
            index: NonZeroU32::MIN.saturating_add(i),
        })
    }

    /// トラック内のチャンク群の情報を走査するイテレーターを返す
    pub fn chunks(&self) -> impl '_ + Iterator<Item = ChunkAccessor> {
        (0..self.chunk_count()).map(|i| ChunkAccessor {
            sample_table: self,
            index: NonZeroU32::MIN.saturating_add(i),
        })
    }
}

/// [`SampleTableAccessor::new()`] で発生する可能性があるエラー
#[derive(Debug)]
pub enum SampleTableAccessorError {
    /// [`SttsBox`] と他のボックスで、表現しているサンプル数が異なる
    InconsistentSampleCount {
        /// [`SttsBox`] 準拠のサンプル数
        stts_sample_count: u32,

        /// [`SttsBox`] とは異なるサンプル数を表しているボックスの種別
        other_box_type: BoxType,

        /// `other_box_type` 準拠のサンプル数
        other_sample_count: u32,
    },

    /// [`StscBox`] の最初のエントリのチャンクインデックスが 1 ではない
    FirstChunkIndexIsNotOne {
        /// 実際の最初のチャンクインデックスの値
        actual_chunk_index: NonZeroU32,
    },

    /// [`StscBox`] が存在しない [`SampleEntry`] を参照している
    MissingSampleEntry {
        /// [`StscEntry`] のインデックス
        stsc_entry_index: usize,

        /// 存在しないサンプルエントリーのインデックス
        sample_description_index: NonZeroU32,

        /// サンプルエントリーの総数
        sample_entry_count: usize,
    },

    /// [`StscBox`] のチャンクインデックスが短調増加していない
    ChunkIndicesNotMonotonicallyIncreasing,
}

impl std::fmt::Display for SampleTableAccessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleTableAccessorError::InconsistentSampleCount {
                stts_sample_count,
                other_box_type,
                other_sample_count,
            } => write!(f, "Sample count in `stts` box is {stts_sample_count}, but `{other_box_type}` has sample count {other_sample_count}"),
            SampleTableAccessorError::FirstChunkIndexIsNotOne { actual_chunk_index } => {
                write!(f,"First chunk index in `stsc` box is expected to 1, but got {actual_chunk_index}")
            }
            SampleTableAccessorError::MissingSampleEntry {
                stsc_entry_index,
                sample_description_index,
                sample_entry_count,
            } => {
                write!(f, "{stsc_entry_index}-th entry in `stsc` box refers to a missing sample entry {sample_description_index} (sample entry count is {sample_entry_count})")
            }
            SampleTableAccessorError::ChunkIndicesNotMonotonicallyIncreasing => {
                write!(f,"Chunk indices in `stsc` box is not monotonically increasing")
            }
        }
    }
}

impl std::error::Error for SampleTableAccessorError {}

/// [`StblBox`] 内の個々のサンプルの情報を取得するための構造体
#[derive(Debug)]
pub struct SampleAccessor<'a> {
    sample_table: &'a SampleTableAccessor<'a>,
    index: NonZeroU32,
}

impl<'a> SampleAccessor<'a> {
    /// このサンプルのインデックスを取得する
    pub fn index(&self) -> NonZeroU32 {
        self.index
    }

    /// サンプルの尺を取得する
    pub fn duration(&self) -> u32 {
        let i = self
            .sample_table
            .sample_durations
            .binary_search_by_key(&(self.index.get() - 1), |x| x.0)
            .unwrap_or_else(|i| i.checked_sub(1).expect("unreachable"));
        self.sample_table.sample_durations[i].1
    }

    /// サンプルのデータサイズ（バイト数）を取得する
    pub fn data_size(&self) -> u32 {
        let i = self.index.get() as usize - 1;
        match &self.sample_table.stbl_box.stsz_box {
            StszBox::Fixed { sample_size, .. } => sample_size.get(),
            StszBox::Variable { entry_sizes } => entry_sizes[i],
        }
    }

    /// サンプルが同期サンプルかどうかを判定する
    pub fn is_sync_sample(&self) -> bool {
        let Some(stss_box) = &self.sample_table.stbl_box.stss_box else {
            // stss ボックスが存在しない場合は全てが同期サンプル扱い
            return true;
        };

        stss_box.sample_numbers.binary_search(&self.index).is_ok()
    }

    /// サンプルが属するチャンクの情報を返す
    pub fn chunk(&self) -> ChunkAccessor {
        let i = self
            .sample_table
            .sample_index_offsets
            .binary_search(&self.index)
            .unwrap_or_else(|i| i - 1);
        let chunk_index = NonZeroU32::MIN.saturating_add(i as u32);
        self.sample_table
            .get_chunk(chunk_index)
            .expect("unreachable")
    }
}

/// [`StblBox`] 内の個々のチャンクの情報を取得するための構造体
#[derive(Debug)]
pub struct ChunkAccessor<'a> {
    sample_table: &'a SampleTableAccessor<'a>,
    index: NonZeroU32,
}

impl<'a> ChunkAccessor<'a> {
    /// このチャンクのインデックスを取得する
    pub fn index(&self) -> NonZeroU32 {
        self.index
    }

    /// チャンクのファイル内でのバイト位置を返す
    pub fn offset(&self) -> u64 {
        let i = self.index.get() as usize - 1;
        match &self.sample_table.stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets[i] as u64,
            Either::B(b) => b.chunk_offsets[i],
        }
    }

    /// チャンクが参照するサンプルエントリー返す
    pub fn sample_entry(&self) -> &SampleEntry {
        &self.sample_table.stbl_box.stsd_box.entries
            [self.stsc_entry().sample_description_index.get() as usize - 1]
    }

    /// チャンクに属するサンプルの数を返す
    pub fn sample_count(&self) -> u32 {
        self.stsc_entry().sample_per_chunk
    }

    /// チャンクに属するサンプル群を走査するイテレーターを返す
    pub fn samples(&self) -> impl '_ + Iterator<Item = SampleAccessor> {
        let count = self.sample_count();
        let sample_index_offset =
            self.sample_table.sample_index_offsets[self.index.get() as usize - 1];
        (0..count).map(move |i| {
            let sample_index = sample_index_offset.saturating_add(i);
            self.sample_table
                .get_sample(sample_index)
                .expect("unreachable")
        })
    }

    fn stsc_entry(&self) -> &StscEntry {
        let i = self
            .sample_table
            .stbl_box
            .stsc_box
            .entries
            .binary_search_by_key(&self.index, |x| x.first_chunk)
            .unwrap_or_else(|i| i - 1);
        &self.sample_table.stbl_box.stsc_box.entries[i]
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        boxes::{StcoBox, StscBox, StscEntry, StsdBox, StssBox, SttsBox, UnknownBox},
        BaseBox, BoxSize, BoxType,
    };

    use super::*;

    #[test]
    fn sample_table_accessor() {
        let sample_durations = [10, 5, 5, 20, 20, 20, 1, 1, 1, 1];
        let chunk_offsets = [100, 200, 300, 400];
        let stbl_box = StblBox {
            stsd_box: StsdBox {
                entries: vec![SampleEntry::Unknown(UnknownBox {
                    box_type: BoxType::Normal(*b"test"),
                    box_size: BoxSize::U32(8),
                    payload: Vec::new(),
                })],
            },
            stts_box: SttsBox::from_sample_deltas(sample_durations),
            stsc_box: StscBox {
                entries: [(index(1), 2, index(1)), (index(3), 3, index(1))]
                    .into_iter()
                    .map(
                        |(first_chunk, sample_per_chunk, sample_description_index)| StscEntry {
                            first_chunk,
                            sample_per_chunk,
                            sample_description_index,
                        },
                    )
                    .collect(),
            },
            stsz_box: StszBox::Variable {
                entry_sizes: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            },
            stco_or_co64_box: Either::A(StcoBox {
                chunk_offsets: chunk_offsets.to_vec(),
            }),
            stss_box: Some(StssBox {
                sample_numbers: vec![index(1), index(3), index(5), index(7), index(9)],
            }),
            unknown_boxes: Vec::new(),
        };

        let sample_table = SampleTableAccessor::new(&stbl_box).expect("bug");
        assert_eq!(sample_table.sample_count(), 10);
        assert_eq!(sample_table.chunk_count(), 4);

        let sample_chunks = [1, 1, 2, 2, 3, 3, 3, 4, 4, 4];
        for i in 0..10 {
            let sample = sample_table.get_sample(index(i as u32 + 1)).expect("bug");
            assert_eq!(sample.duration(), sample_durations[i]);
            assert_eq!(sample.data_size(), i as u32);
            assert_eq!(sample.is_sync_sample(), (i + 1) % 2 == 1);
            assert_eq!(sample.chunk().index().get(), sample_chunks[i]);
        }
        assert!(sample_table.get_sample(index(11)).is_none());

        let sample_counts = [2, 2, 3, 3];
        for i in 0..4 {
            let chunk = sample_table.get_chunk(index(i as u32 + 1)).expect("bug");
            assert_eq!(chunk.offset(), chunk_offsets[i] as u64);
            assert_eq!(chunk.sample_entry().box_type().as_bytes(), b"test");
            assert_eq!(chunk.sample_count(), sample_counts[i]);
            assert_eq!(chunk.samples().count(), sample_counts[i] as usize);
        }
        assert!(sample_table.get_chunk(index(5)).is_none());
    }

    fn index(i: u32) -> NonZeroU32 {
        NonZeroU32::new(i).expect("invalid index")
    }
}