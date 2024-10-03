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
    chunk_count: u32,
    sample_count: u32,
    sample_durations: Vec<(u32, u32)>,     // (累計サンプル数、尺）
    sample_index_offsets: Vec<NonZeroU32>, // チャンク先頭のサンプルインデックス
}

impl<'a> SampleTableAccessor<'a> {
    /// 引数で渡された [`StblBox`] 用の [`SampleTableAccessor`] インスタンスを生成する
    ///
    /// [`StblBox`] の子ボックス群に不整合がある場合には [`None`] が返される
    pub fn new(stbl_box: &'a StblBox) -> Option<Self> {
        let mut sample_count = 0;
        let mut sample_durations = Vec::new();
        for entry in &stbl_box.stts_box.entries {
            sample_durations.push((sample_count, entry.sample_delta));
            sample_count += entry.sample_count;
        }

        if let StszBox::Variable { entry_sizes } = &stbl_box.stsz_box {
            if entry_sizes.len() != sample_count as usize {
                // stts と stsz でサンプル数が異なる
                return None;
            }
        }

        let chunk_count = match &stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets.len() as u32,
            Either::B(b) => b.chunk_offsets.len() as u32,
        };

        if stbl_box
            .stsc_box
            .entries
            .first()
            .map_or(false, |x| x.first_chunk.get() != 1)
        {
            // チャンクインデックスが 1 以外から始まっている
            return None;
        }
        if stbl_box
            .stsc_box
            .entries
            .iter()
            .any(|x| stbl_box.stsd_box.entries.len() < x.sample_description_index.get() as usize)
        {
            // 存在しないサンプルエントリーを参照しているチャンクがある
            return None;
        }
        if stbl_box
            .stsc_box
            .entries
            .iter()
            .zip(stbl_box.stsc_box.entries.iter().skip(1))
            .any(|(prev, next)| prev.first_chunk >= next.first_chunk)
        {
            // stsc 内のチャンクインデックスが短調増加していない
            return None;
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
        if first_sample_index.get() != sample_count {
            // stts と stsc でサンプル数が異なる
            return None;
        }

        Some(Self {
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
        BoxSize, BoxType,
    };

    use super::*;

    #[test]
    fn sample_table_accessor() {
        let sample_durations = [10, 5, 5, 20, 20, 20, 1, 1, 1, 1];
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
                entries: [(index(1), 2, index(1)), (index(7), 4, index(1))]
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
                chunk_offsets: vec![100, 200, 300],
            }),
            stss_box: Some(StssBox {
                sample_numbers: vec![index(1), index(3), index(5), index(7), index(9)],
            }),
            unknown_boxes: Vec::new(),
        };

        let sample_table = SampleTableAccessor::new(&stbl_box).expect("bug");
        assert_eq!(sample_table.sample_count(), 10);
        assert_eq!(sample_table.chunk_count(), 3);

        for i in 0..10 {
            let sample = sample_table.get_sample(index(i as u32 + 1)).expect("bug");
            assert_eq!(sample.duration(), sample_durations[i]);
            assert_eq!(sample.data_size(), i as u32);
            assert_eq!(sample.is_sync_sample(), (i + 1) % 2 == 1);
        }
        assert!(sample_table.get_sample(index(11)).is_none());

        // // Chunk offset.
        // assert_eq!(sample_table.get_chunk_offset(index(1)), Some(100));
        // assert_eq!(sample_table.get_chunk_offset(index(2)), Some(200));
        // assert_eq!(sample_table.get_chunk_offset(index(3)), Some(300));
        // assert_eq!(sample_table.get_chunk_offset(index(4)), None);

        // // Sample entry.
        // assert!(sample_table.get_sample_entry(index(1)).is_some());
        // assert!(sample_table.get_sample_entry(index(2)).is_none());

        // Chunks.
        // assert_eq!(sample_table.chunks(), vec![]);
    }

    fn index(i: u32) -> NonZeroU32 {
        NonZeroU32::new(i).expect("invalid index")
    }
}
