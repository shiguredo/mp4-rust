#![expect(missing_docs)]

use core::{num::NonZeroU32, time::Duration};

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::{
    BoxHeader, Encode, Error,
    boxes::{Brand, FreeBox, FtypBox, MdatBox, SampleEntry},
};

pub const TIMESCALE: NonZeroU32 = NonZeroU32::MIN.saturating_add(1_000_000 - 1);

/// メディアトラックの種類を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackKind {
    /// 音声トラック
    Audio,

    /// 映像トラック
    Video,
}

#[derive(Debug, Default, Clone)]
pub struct Mp4FileMuxerOptions {
    pub reserved_moov_box_size: usize,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub track_kind: TrackKind,
    pub sample_entry: Option<SampleEntry>,
    pub keyfframe: bool,
    pub duration: Duration,
    pub data_offset: u64,
    pub data_size: usize,
}

#[derive(Debug)]
pub enum MuxError {
    EncodeError(Error),
    PositionMismatch { expected: u64, actual: u64 },
}

impl From<Error> for MuxError {
    fn from(error: Error) -> Self {
        MuxError::EncodeError(error)
    }
}

impl core::fmt::Display for MuxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MuxError::EncodeError(error) => {
                write!(f, "Failed to encode MP4 box: {error}")
            }
            MuxError::PositionMismatch { expected, actual } => {
                write!(
                    f,
                    "Position mismatch: expected {expected}, but got {actual}"
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MuxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let MuxError::EncodeError(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Mp4FileMuxer {
    options: Mp4FileMuxerOptions,
    header_bytes: Vec<u8>,
    next_position: u64,
}

impl Mp4FileMuxer {
    pub fn new() -> Result<Self, MuxError> {
        Self::with_options(Mp4FileMuxerOptions::default())
    }

    pub fn with_options(options: Mp4FileMuxerOptions) -> Result<Self, MuxError> {
        let mut this = Self {
            options,
            header_bytes: Vec::new(),
            next_position: 0,
        };
        this.build_header()?;
        Ok(this)
    }

    fn build_header(&mut self) -> Result<(), MuxError> {
        // ftyp ボックスを構築
        let ftyp_box = FtypBox {
            major_brand: Brand::ISOM,
            minor_version: 0,
            compatible_brands: vec![
                Brand::ISOM,
                Brand::ISO2,
                Brand::MP41,
                Brand::AVC1,
                Brand::AV01,
            ],
        };

        // ftyp ボックスをヘッダーバイト列に追加
        self.header_bytes = ftyp_box.encode_to_vec()?;

        // faststart 用の moov ボックス用の領域を free ボックスで事前に確保する
        // （先頭付近にmoovボックスを配置することで、動画プレイヤーの再生開始までに掛かる時間を短縮できる）
        if let Some(payload_size) = self
            .options
            .reserved_moov_box_size
            .checked_sub(BoxHeader::MIN_SIZE)
        {
            let free_box = FreeBox {
                payload: vec![0; payload_size],
            };
            self.header_bytes
                .extend_from_slice(&free_box.encode_to_vec()?);
        }

        // 可変長の mdat ボックスのヘッダーを書きこむ
        let mdat_box = MdatBox {
            is_variable_size: true,
            payload: Vec::new(),
        };
        self.header_bytes
            .extend_from_slice(&mdat_box.encode_to_vec()?);

        // サンプルのデータが mdat ボックスに追記されていくように、ポジションを更新
        self.next_position = self.header_bytes.len() as u64;

        Ok(())
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
