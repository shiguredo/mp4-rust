#![expect(missing_docs, dead_code)]

use core::num::NonZeroU32;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::Error;

/// メディアトラックの種類を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackKind {
    /// 音声トラック
    Audio,

    /// 映像トラック
    Video,
}

/// メディアトラックの情報を表す構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackInfo {
    /// トラックの種類
    pub kind: TrackKind,

    /// トラックで使用されているタイムスケール
    pub timescale: NonZeroU32,
}

#[derive(Debug, Clone)]
pub struct Mp4FileMuxerOptions {
    pub audio_track: bool, // TODO: Option<TrackInfo>,
    pub video_track: bool, // TODO: Option<TrackInfo>,
    pub reserved_moov_box_size: usize,
}

impl Default for Mp4FileMuxerOptions {
    fn default() -> Self {
        Self {
            audio_track: true,
            video_track: true,
            reserved_moov_box_size: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub data_offset: u64,
    pub data_size: usize,
}

#[derive(Debug)]
pub enum MuxError {
    EncodeError(Error),
    PositionMismatch { expected: u64, actual: u64 },
}

#[derive(Debug)]
pub struct Mp4FileMuxer {
    options: Mp4FileMuxerOptions,
    header_bytes: Vec<u8>,
    next_position: u64,
}

impl Mp4FileMuxer {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_options(Mp4FileMuxerOptions::default())
    }

    pub fn with_options(options: Mp4FileMuxerOptions) -> Self {
        let mut this = Self {
            options,
            header_bytes: Vec::new(),
            next_position: 0,
        };
        this.build_header_bytes();
        this
    }

    fn build_header_bytes(&mut self) {
        // TODO: build ftyp / initial mdat box, then update header_bytes and next_position
    }

    pub fn header_bytes(&self) -> &[u8] {
        &self.header_bytes
    }

    pub fn finalized_bytes_list(&self) -> &[(u64, &[u8])] {
        todo!()
    }

    pub fn append_sample(&mut self, sample: &Sample) -> Result<(), MuxError> {
        if self.next_position != sample.data_offset {
            return Err(MuxError::PositionMismatch {
                expected: self.next_position,
                actual: sample.data_offset,
            });
        }
        self.next_position += sample.data_size as u64;
        todo!()
    }

    pub fn finalize(&mut self) {}
}
