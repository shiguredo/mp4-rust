#![expect(missing_docs)]

use core::{num::NonZeroU32, time::Duration};

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::{
    BoxHeader, BoxSize, Either, Encode, Error, FixedPointNumber, Mp4FileTime, TrackKind,
    Utf8String,
    boxes::{
        Brand, Co64Box, DinfBox, FreeBox, FtypBox, HdlrBox, MdatBox, MdhdBox, MdiaBox, MinfBox,
        MoovBox, MvhdBox, SampleEntry, SmhdBox, StblBox, StcoBox, StscBox, StscEntry, StsdBox,
        StssBox, StszBox, SttsBox, TkhdBox, TrakBox, VmhdBox,
    },
};

#[derive(Debug, Clone)]
pub struct Mp4FileMuxerOptions {
    pub reserved_moov_box_size: usize,
    pub creation_timestamp: Duration,
}

impl Default for Mp4FileMuxerOptions {
    fn default() -> Self {
        #[cfg(feature = "std")]
        let creation_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);

        #[cfg(not(feature = "std"))]
        let creation_timestamp = Duration::ZERO;

        Self {
            reserved_moov_box_size: 0,
            creation_timestamp,
        }
    }
}

#[derive(Debug)]
pub struct FinalizedBoxes {
    moov_box_offset: u64,
    moov_box_bytes: Vec<u8>,
    mdat_box_offset: u64,
    mdat_box_header_bytes: Vec<u8>,
}

impl FinalizedBoxes {
    pub fn is_faststart_enabled(&self) -> bool {
        self.moov_box_offset < self.mdat_box_offset
    }

    pub fn offset_and_bytes_pairs(&self) -> impl Iterator<Item = (u64, &[u8])> {
        [
            (self.moov_box_offset, self.moov_box_bytes.as_slice()),
            (self.mdat_box_offset, self.mdat_box_header_bytes.as_slice()),
        ]
        .into_iter()
    }
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

pub enum MuxError {
    EncodeError(Error),
    PositionMismatch { expected: u64, actual: u64 },
    MissingSampleEntry { track_kind: TrackKind },
    AlreadyFinalized,
}

impl From<Error> for MuxError {
    fn from(error: Error) -> Self {
        MuxError::EncodeError(error)
    }
}

impl core::fmt::Debug for MuxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
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
            MuxError::MissingSampleEntry { track_kind } => {
                write!(
                    f,
                    "Missing sample entry for first sample of {track_kind:?} track",
                )
            }
            MuxError::AlreadyFinalized => {
                write!(f, "Muxer has already been finalized")
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

#[derive(Debug, Clone)]
struct SampleMetadata {
    duration: u32,
    keyframe: bool,
    size: u32,
}

#[derive(Debug, Clone)]
struct Chunk {
    offset: u64,
    sample_entry: SampleEntry,
    samples: Vec<SampleMetadata>,
}

#[derive(Debug)]
pub struct Mp4FileMuxer {
    options: Mp4FileMuxerOptions,
    initial_boxes_bytes: Vec<u8>,
    free_box_offset: u64,
    mdat_box_offset: u64,
    next_position: u64,
    last_sample_kind: Option<TrackKind>,
    finalized_boxes: Option<FinalizedBoxes>,
    audio_chunks: Vec<Chunk>,
    video_chunks: Vec<Chunk>,
}

impl Mp4FileMuxer {
    pub const TIMESCALE: NonZeroU32 = NonZeroU32::MIN.saturating_add(1_000_000 - 1);

    pub fn new() -> Result<Self, MuxError> {
        Self::with_options(Mp4FileMuxerOptions::default())
    }

    pub fn with_options(options: Mp4FileMuxerOptions) -> Result<Self, MuxError> {
        let mut this = Self {
            options,
            initial_boxes_bytes: Vec::new(),
            free_box_offset: 0,
            mdat_box_offset: 0,
            next_position: 0,
            last_sample_kind: None,
            finalized_boxes: None,
            audio_chunks: Vec::new(),
            video_chunks: Vec::new(),
        };
        this.build_initial_boxes()?;
        Ok(this)
    }

    fn build_initial_boxes(&mut self) -> Result<(), MuxError> {
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
        self.initial_boxes_bytes = ftyp_box.encode_to_vec()?;
        self.free_box_offset = self.initial_boxes_bytes.len() as u64;

        // faststart 用の moov ボックス用の領域を free ボックスで事前に確保する
        // （先頭付近に moov ボックスを配置することで、動画プレイヤーの再生開始までに掛かる時間を短縮できる）
        if self.options.reserved_moov_box_size > 0 {
            let free_box = FreeBox {
                payload: vec![0; self.options.reserved_moov_box_size],
            };
            self.initial_boxes_bytes
                .extend_from_slice(&free_box.encode_to_vec()?);
        }
        self.mdat_box_offset = self.initial_boxes_bytes.len() as u64;

        // 可変長の mdat ボックスのヘッダーを書きこむ
        let mdat_box_header = BoxHeader::new(MdatBox::TYPE, BoxSize::LARGE_VARIABLE_SIZE);
        self.initial_boxes_bytes
            .extend_from_slice(&mdat_box_header.encode_to_vec()?);

        // サンプルのデータが mdat ボックスに追記されていくように、ポジションを更新
        self.next_position = self.initial_boxes_bytes.len() as u64;

        Ok(())
    }

    pub fn initial_boxes_bytes(&self) -> &[u8] {
        &self.initial_boxes_bytes
    }

    pub fn finalized_boxes(&self) -> Option<&FinalizedBoxes> {
        self.finalized_boxes.as_ref()
    }

    pub fn append_sample(&mut self, sample: &Sample) -> Result<(), MuxError> {
        if self.finalized_boxes.is_some() {
            return Err(MuxError::AlreadyFinalized);
        }
        if self.next_position != sample.data_offset {
            return Err(MuxError::PositionMismatch {
                expected: self.next_position,
                actual: sample.data_offset,
            });
        }

        let metadata = SampleMetadata {
            duration: sample.duration.as_micros() as u32,
            keyframe: sample.keyfframe,
            size: sample.data_size as u32,
        };

        let is_new_chunk_needed = self.is_new_chunk_needed(sample);

        let chunks = match sample.track_kind {
            TrackKind::Audio => &mut self.audio_chunks,
            TrackKind::Video => &mut self.video_chunks,
        };

        if is_new_chunk_needed {
            let sample_entry = sample
                .sample_entry
                .clone()
                .or_else(|| chunks.last().map(|c| c.sample_entry.clone()))
                .ok_or(MuxError::MissingSampleEntry {
                    track_kind: sample.track_kind,
                })?;

            chunks.push(Chunk {
                offset: sample.data_offset,
                sample_entry,
                samples: Vec::new(),
            });
        }

        chunks.last_mut().expect("bug").samples.push(metadata);

        self.next_position += sample.data_size as u64;
        self.last_sample_kind = Some(sample.track_kind);
        Ok(())
    }

    fn is_new_chunk_needed(&self, sample: &Sample) -> bool {
        if self.last_sample_kind != Some(sample.track_kind) {
            return true;
        }

        let chunks = match sample.track_kind {
            TrackKind::Audio => &self.audio_chunks,
            TrackKind::Video => &self.video_chunks,
        };

        let Some(sample_entry) = &sample.sample_entry else {
            return false;
        };

        chunks
            .last()
            .is_none_or(|c| c.sample_entry != *sample_entry)
    }

    pub fn finalize(&mut self) -> Result<(), MuxError> {
        if self.finalized_boxes.is_some() {
            return Err(MuxError::AlreadyFinalized);
        }

        // moov ボックスを構築
        let moov_box = self.build_moov_box()?;
        let mut moov_box_bytes = moov_box.encode_to_vec()?;

        // moov ボックスの書き込み位置を決定
        let moov_box_offset = if let Some(free_box_payload_size) = self
            .options
            .reserved_moov_box_size
            .checked_sub(moov_box_bytes.len())
        {
            // 事前に確保した free ボックスのペイロード領域に収まる場合は、そこに moov ボックスを書き込む
            // （free ボックスのヘッダーも更新して moov ボックスの末尾に追加する）
            let free_box = FreeBox {
                payload: vec![0; free_box_payload_size],
            };
            moov_box_bytes.extend_from_slice(&free_box.encode_to_vec()?);

            self.free_box_offset
        } else {
            // 収まらない場合はファイル末尾に moov ボックスを追記
            self.next_position
        };

        // mdat ボックスヘッダーのサイズ部分を確定する
        let mdat_box_size = self.next_position - self.mdat_box_offset;
        let mdat_box_header = BoxHeader::new(MdatBox::TYPE, BoxSize::U64(mdat_box_size));
        let mdat_box_header_bytes = mdat_box_header.encode_to_vec()?;

        self.finalized_boxes = Some(FinalizedBoxes {
            moov_box_offset,
            moov_box_bytes,
            mdat_box_offset: self.mdat_box_offset,
            mdat_box_header_bytes,
        });
        Ok(())
    }

    fn build_moov_box(&self) -> Result<MoovBox, MuxError> {
        let mut trak_boxes = Vec::new();

        if !self.audio_chunks.is_empty() {
            let track_id = trak_boxes.len() as u32 + 1;
            trak_boxes.push(self.build_audio_trak_box(track_id)?);
        }

        if !self.video_chunks.is_empty() {
            let track_id = trak_boxes.len() as u32 + 1;
            trak_boxes.push(self.build_video_trak_box(track_id)?);
        }

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let mvhd_box = MvhdBox {
            creation_time,
            modification_time: creation_time,
            timescale: Self::TIMESCALE,
            duration: self.calculate_total_duration(),
            rate: MvhdBox::DEFAULT_RATE,
            volume: MvhdBox::DEFAULT_VOLUME,
            matrix: MvhdBox::DEFAULT_MATRIX,
            next_track_id: trak_boxes.len() as u32 + 1,
        };

        Ok(MoovBox {
            mvhd_box,
            trak_boxes,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_audio_trak_box(&self, track_id: u32) -> Result<TrakBox, MuxError> {
        let total_duration = self
            .audio_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let tkhd_box = TkhdBox {
            flag_track_enabled: true,
            flag_track_in_movie: true,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,
            creation_time,
            modification_time: creation_time,
            track_id,
            duration: total_duration,
            layer: TkhdBox::DEFAULT_LAYER,
            alternate_group: TkhdBox::DEFAULT_ALTERNATE_GROUP,
            volume: TkhdBox::DEFAULT_AUDIO_VOLUME,
            matrix: TkhdBox::DEFAULT_MATRIX,
            width: FixedPointNumber::default(),
            height: FixedPointNumber::default(),
        };

        Ok(TrakBox {
            tkhd_box,
            edts_box: None,
            mdia_box: self.build_audio_mdia_box()?,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_video_trak_box(&self, track_id: u32) -> Result<TrakBox, MuxError> {
        let total_duration = self
            .video_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let (max_width, max_height) = self
            .video_chunks
            .iter()
            .filter_map(|c| c.sample_entry.video_resolution())
            .fold((0u32, 0u32), |(max_w, max_h), (w, h)| {
                (max_w.max(w), max_h.max(h))
            });

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let tkhd_box = TkhdBox {
            flag_track_enabled: true,
            flag_track_in_movie: true,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,
            creation_time,
            modification_time: creation_time,
            track_id,
            duration: total_duration,
            layer: TkhdBox::DEFAULT_LAYER,
            alternate_group: TkhdBox::DEFAULT_ALTERNATE_GROUP,
            volume: TkhdBox::DEFAULT_VIDEO_VOLUME,
            matrix: TkhdBox::DEFAULT_MATRIX,
            width: FixedPointNumber::new(max_width as i16, 0),
            height: FixedPointNumber::new(max_height as i16, 0),
        };

        Ok(TrakBox {
            tkhd_box,
            edts_box: None,
            mdia_box: self.build_video_mdia_box()?,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_audio_mdia_box(&self) -> Result<MdiaBox, MuxError> {
        let total_duration = self
            .audio_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let sample_entry = self.audio_chunks.first().map(|c| &c.sample_entry).ok_or(
            MuxError::MissingSampleEntry {
                track_kind: TrackKind::Audio,
            },
        )?;

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let mdhd_box = MdhdBox {
            creation_time,
            modification_time: creation_time,
            timescale: Self::TIMESCALE,
            duration: total_duration,
            language: MdhdBox::LANGUAGE_UNDEFINED,
        };

        let hdlr_box = HdlrBox {
            handler_type: HdlrBox::HANDLER_TYPE_SOUN,
            name: Utf8String::EMPTY.into_null_terminated_bytes(),
        };

        let minf_box = MinfBox {
            smhd_or_vmhd_box: Either::A(SmhdBox::default()),
            dinf_box: DinfBox::LOCAL_FILE,
            stbl_box: self.build_stbl_box(sample_entry, &self.audio_chunks),
            unknown_boxes: Vec::new(),
        };

        Ok(MdiaBox {
            mdhd_box,
            hdlr_box,
            minf_box,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_video_mdia_box(&self) -> Result<MdiaBox, MuxError> {
        let total_duration = self
            .video_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let sample_entry = self.video_chunks.first().map(|c| &c.sample_entry).ok_or(
            MuxError::MissingSampleEntry {
                track_kind: TrackKind::Video,
            },
        )?;

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let mdhd_box = MdhdBox {
            creation_time,
            modification_time: creation_time,
            timescale: Self::TIMESCALE,
            duration: total_duration,
            language: MdhdBox::LANGUAGE_UNDEFINED,
        };

        let hdlr_box = HdlrBox {
            handler_type: HdlrBox::HANDLER_TYPE_VIDE,
            name: Utf8String::EMPTY.into_null_terminated_bytes(),
        };

        let minf_box = MinfBox {
            smhd_or_vmhd_box: Either::B(VmhdBox::default()),
            dinf_box: DinfBox::LOCAL_FILE,
            stbl_box: self.build_stbl_box(sample_entry, &self.video_chunks),
            unknown_boxes: Vec::new(),
        };

        Ok(MdiaBox {
            mdhd_box,
            hdlr_box,
            minf_box,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_stbl_box(&self, sample_entry: &SampleEntry, chunks: &[Chunk]) -> StblBox {
        let stsd_box = StsdBox {
            entries: vec![sample_entry.clone()],
        };

        let stts_box = SttsBox::from_sample_deltas(
            chunks
                .iter()
                .flat_map(|c| c.samples.iter().map(|s| s.duration)),
        );

        let stsc_box = StscBox {
            entries: chunks
                .iter()
                .enumerate()
                .map(|(i, c)| StscEntry {
                    first_chunk: NonZeroU32::MIN.saturating_add(i as u32),
                    sample_per_chunk: c.samples.len() as u32,
                    sample_description_index: NonZeroU32::MIN,
                })
                .collect(),
        };

        let stsz_box = StszBox::Variable {
            entry_sizes: chunks
                .iter()
                .flat_map(|c| c.samples.iter().map(|s| s.size))
                .collect(),
        };

        let stco_or_co64_box = if self.next_position > u32::MAX as u64 {
            Either::B(Co64Box {
                chunk_offsets: chunks.iter().map(|c| c.offset).collect(),
            })
        } else {
            Either::A(StcoBox {
                chunk_offsets: chunks.iter().map(|c| c.offset as u32).collect(),
            })
        };

        let is_all_keyframe = chunks.iter().all(|c| c.samples.iter().all(|s| s.keyframe));
        let stss_box = if is_all_keyframe {
            None
        } else {
            Some(StssBox {
                sample_numbers: chunks
                    .iter()
                    .flat_map(|c| c.samples.iter())
                    .enumerate()
                    .filter_map(|(i, s)| {
                        s.keyframe
                            .then_some(NonZeroU32::MIN.saturating_add(i as u32))
                    })
                    .collect(),
            })
        };

        StblBox {
            stsd_box,
            stts_box,
            stsc_box,
            stsz_box,
            stco_or_co64_box,
            stss_box,
            unknown_boxes: Vec::new(),
        }
    }

    fn calculate_total_duration(&self) -> u64 {
        let audio_duration = self
            .audio_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let video_duration = self
            .video_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        audio_duration.max(video_duration)
    }
}
