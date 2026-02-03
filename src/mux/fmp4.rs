//! Fragmented MP4 (fMP4) 向けの file muxer。
use alloc::{vec, vec::Vec};
use core::{num::NonZeroU32, time::Duration};

use crate::{
    BoxHeader, BoxSize, Either, Encode, Error, FixedPointNumber, Mp4FileTime, SampleFlags,
    TrackKind, Utf8String,
    boxes::{
        Brand, DinfBox, FtypBox, HdlrBox, MdatBox, MdhdBox, MdiaBox, MinfBox, MoofBox, MoovBox,
        MvexBox, MvhdBox, SampleEntry, SmhdBox, StblBox, StcoBox, StscBox, StsdBox, StszBox,
        SttsBox, TfdtBox, TfhdBox, TkhdBox, TrafBox, TrakBox, TrexBox, TrunBox, TrunSample,
        VmhdBox,
    },
};

/// Fragmented MP4 の file muxer 用のオプション。
#[derive(Debug, Clone)]
pub struct Mp4FragmentedFileMuxerOptions {
    /// ftyp ボックスの major brand。
    pub major_brand: Brand,
    /// ftyp ボックスの minor version。
    pub minor_version: u32,
    /// ftyp ボックスの compatible brands。
    pub compatible_brands: Vec<Brand>,
    /// ファイル作成時刻。
    pub creation_timestamp: Duration,
}

impl Default for Mp4FragmentedFileMuxerOptions {
    fn default() -> Self {
        Self {
            major_brand: Brand::ISOM,
            minor_version: 0,
            compatible_brands: vec![
                Brand::ISOM,
                Brand::ISO6,
                Brand::MP41,
                Brand::AVC1,
                Brand::AV01,
            ],
            creation_timestamp: Duration::ZERO,
        }
    }
}

/// fMP4 の初期化セグメントに含めるトラック情報。
#[derive(Debug, Clone)]
pub struct TrackConfig {
    /// トラック ID。
    pub track_id: u32,
    /// トラック種別。
    pub kind: TrackKind,
    /// タイムスケール。
    pub timescale: NonZeroU32,
    /// サンプルエントリー。
    pub sample_entry: SampleEntry,
}

/// フラグメントに含めるサンプル情報。
#[derive(Debug, Clone)]
pub struct FragmentSample {
    /// トラック ID。
    pub track_id: u32,
    /// サンプルの尺（タイムスケール単位）。
    pub duration: u32,
    /// サンプルデータのサイズ（バイト）。
    pub data_size: u32,
    /// キーフレームであるかどうか。
    pub keyframe: bool,
    /// composition time offset。
    pub composition_time_offset: Option<i32>,
    /// サンプルフラグ（指定がない場合は keyframe から推定）。
    pub sample_flags: Option<SampleFlags>,
}

/// フラグメント生成結果。
#[derive(Debug, Clone)]
pub struct FragmentOutput {
    moof_box: MoofBox,
    moof_bytes: Vec<u8>,
    mdat_header_bytes: Vec<u8>,
    media_data_size: u64,
}

impl FragmentOutput {
    /// moof ボックスのバイト列を返す。
    pub fn moof_bytes(&self) -> &[u8] {
        &self.moof_bytes
    }

    /// mdat ボックスのヘッダーバイト列を返す。
    pub fn mdat_header_bytes(&self) -> &[u8] {
        &self.mdat_header_bytes
    }

    /// mdat ボックスのペイロードサイズを返す。
    pub fn media_data_size(&self) -> u64 {
        self.media_data_size
    }

    /// 構築された moof ボックスを返す。
    pub fn moof_box(&self) -> &MoofBox {
        &self.moof_box
    }
}

/// fMP4 の muxer で発生するエラー。
#[non_exhaustive]
pub enum Fmp4MuxError {
    /// MP4 ボックスのエンコード処理中に発生したエラー。
    EncodeError(Error),
    /// トラックが 1 つも設定されていない。
    EmptyTracks,
    /// 無効なトラック ID。
    InvalidTrackId {
        /// トラック ID。
        track_id: u32,
    },
    /// トラック ID が重複している。
    DuplicateTrackId {
        /// トラック ID。
        track_id: u32,
    },
    /// 未登録のトラック ID が指定された。
    UnknownTrackId {
        /// トラック ID。
        track_id: u32,
    },
    /// フラグメント内のサンプルが空。
    EmptyFragment,
    /// トラックごとのサンプルが連続していない。
    InterleavedSamples {
        /// トラック ID。
        track_id: u32,
    },
    /// メディアデータのサイズがオーバーフローした。
    MediaDataSizeOverflow,
    /// data_offset が i32 の範囲を超えた。
    DataOffsetTooLarge {
        /// data_offset。
        data_offset: u64,
    },
    /// フラグメントのシーケンス番号がオーバーフローした。
    SequenceNumberOverflow,
    /// トラックの decode time がオーバーフローした。
    DecodeTimeOverflow {
        /// トラック ID。
        track_id: u32,
    },
    /// next_track_id がオーバーフローした。
    NextTrackIdOverflow,
}

impl From<Error> for Fmp4MuxError {
    fn from(error: Error) -> Self {
        Self::EncodeError(error)
    }
}

impl core::fmt::Debug for Fmp4MuxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
    }
}

impl core::fmt::Display for Fmp4MuxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EncodeError(error) => write!(f, "Failed to encode MP4 box: {error}"),
            Self::EmptyTracks => write!(f, "No tracks configured"),
            Self::InvalidTrackId { track_id } => write!(f, "Invalid track_id: {track_id}"),
            Self::DuplicateTrackId { track_id } => {
                write!(f, "Duplicate track_id: {track_id}")
            }
            Self::UnknownTrackId { track_id } => write!(f, "Unknown track_id: {track_id}"),
            Self::EmptyFragment => write!(f, "No samples provided for fragment"),
            Self::InterleavedSamples { track_id } => {
                write!(
                    f,
                    "Interleaved samples are not supported: track_id {track_id}"
                )
            }
            Self::MediaDataSizeOverflow => write!(f, "Media data size overflow"),
            Self::DataOffsetTooLarge { data_offset } => {
                write!(f, "data_offset exceeds i32 range: {data_offset}")
            }
            Self::SequenceNumberOverflow => write!(f, "Fragment sequence number overflow"),
            Self::DecodeTimeOverflow { track_id } => {
                write!(f, "Decode time overflow for track_id {track_id}")
            }
            Self::NextTrackIdOverflow => write!(f, "next_track_id overflow"),
        }
    }
}

impl core::error::Error for Fmp4MuxError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        if let Self::EncodeError(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct TrackState {
    next_decode_time: u64,
}

#[derive(Debug, Clone)]
struct TrackFragment<'a> {
    track_index: usize,
    track_id: u32,
    samples: Vec<&'a FragmentSample>,
    base_decode_time: u64,
    duration_sum: u64,
    data_size_sum: u64,
    use_composition_time_offset: bool,
}

/// Fragmented MP4 の file muxer。
#[derive(Debug, Clone)]
pub struct Mp4FragmentedFileMuxer {
    tracks: Vec<TrackConfig>,
    track_states: Vec<TrackState>,
    init_segment_bytes: Vec<u8>,
    next_sequence_number: u32,
}

impl Mp4FragmentedFileMuxer {
    /// デフォルトオプションで muxer を生成する。
    pub fn new(tracks: Vec<TrackConfig>) -> Result<Self, Fmp4MuxError> {
        Self::with_options(tracks, Mp4FragmentedFileMuxerOptions::default())
    }

    /// 指定したオプションで muxer を生成する。
    pub fn with_options(
        tracks: Vec<TrackConfig>,
        options: Mp4FragmentedFileMuxerOptions,
    ) -> Result<Self, Fmp4MuxError> {
        validate_tracks(&tracks)?;
        let init_segment_bytes = build_init_segment(&tracks, &options)?;
        let track_states = tracks
            .iter()
            .map(|_track| TrackState {
                next_decode_time: 0,
            })
            .collect();

        Ok(Self {
            tracks,
            track_states,
            init_segment_bytes,
            next_sequence_number: 1,
        })
    }

    /// 初期化セグメントのバイト列を返す。
    pub fn init_segment_bytes(&self) -> &[u8] {
        &self.init_segment_bytes
    }

    /// フラグメントを構築する。
    ///
    /// サンプルはトラックごとに連続して並んでいる必要がある。
    /// 渡された順に mdat に格納される前提で data_offset を計算する。
    pub fn build_fragment(
        &mut self,
        samples: &[FragmentSample],
    ) -> Result<FragmentOutput, Fmp4MuxError> {
        if samples.is_empty() {
            return Err(Fmp4MuxError::EmptyFragment);
        }

        let sequence_number = self.next_sequence_number;
        let next_sequence_number = self
            .next_sequence_number
            .checked_add(1)
            .ok_or(Fmp4MuxError::SequenceNumberOverflow)?;

        let fragments = self.group_samples(samples)?;
        let total_data_size = fragments.iter().try_fold(0u64, |acc, fragment| {
            acc.checked_add(fragment.data_size_sum)
                .ok_or(Fmp4MuxError::MediaDataSizeOverflow)
        })?;

        let placeholder_moof = MoofBox {
            mfhd_box: crate::boxes::MfhdBox { sequence_number },
            traf_boxes: self.build_traf_boxes(&fragments, None)?,
            unknown_boxes: Vec::new(),
        };
        let placeholder_moof_bytes = placeholder_moof.encode_to_vec()?;
        let moof_size = placeholder_moof_bytes.len() as u64;

        let mdat_header_bytes = build_mdat_header_bytes(total_data_size)?;
        let mdat_header_size = mdat_header_bytes.len() as u64;

        let data_offsets = compute_data_offsets(&fragments, moof_size, mdat_header_size)?;

        let moof_box = MoofBox {
            mfhd_box: crate::boxes::MfhdBox { sequence_number },
            traf_boxes: self.build_traf_boxes(&fragments, Some(&data_offsets))?,
            unknown_boxes: Vec::new(),
        };
        let moof_bytes = moof_box.encode_to_vec()?;

        self.apply_fragment_updates(&fragments)?;
        self.next_sequence_number = next_sequence_number;

        Ok(FragmentOutput {
            moof_box,
            moof_bytes,
            mdat_header_bytes,
            media_data_size: total_data_size,
        })
    }

    fn track_index(&self, track_id: u32) -> Result<usize, Fmp4MuxError> {
        self.tracks
            .iter()
            .position(|track| track.track_id == track_id)
            .ok_or(Fmp4MuxError::UnknownTrackId { track_id })
    }

    fn group_samples<'a>(
        &self,
        samples: &'a [FragmentSample],
    ) -> Result<Vec<TrackFragment<'a>>, Fmp4MuxError> {
        let mut fragments: Vec<TrackFragment<'a>> = Vec::new();
        let mut seen_tracks: Vec<u32> = Vec::new();

        for sample in samples {
            let track_index = self.track_index(sample.track_id)?;
            let base_decode_time = self
                .track_states
                .get(track_index)
                .expect("track state should exist")
                .next_decode_time;

            if let Some(last) = fragments.last_mut()
                && last.track_id == sample.track_id
            {
                last.duration_sum = last
                    .duration_sum
                    .checked_add(sample.duration as u64)
                    .ok_or(Fmp4MuxError::DecodeTimeOverflow {
                        track_id: sample.track_id,
                    })?;
                last.data_size_sum = last
                    .data_size_sum
                    .checked_add(sample.data_size as u64)
                    .ok_or(Fmp4MuxError::MediaDataSizeOverflow)?;
                last.use_composition_time_offset |= sample.composition_time_offset.is_some();
                last.samples.push(sample);
                continue;
            }

            if seen_tracks.contains(&sample.track_id) {
                return Err(Fmp4MuxError::InterleavedSamples {
                    track_id: sample.track_id,
                });
            }
            seen_tracks.push(sample.track_id);

            let mut fragment = TrackFragment {
                track_index,
                track_id: sample.track_id,
                samples: Vec::new(),
                base_decode_time,
                duration_sum: 0,
                data_size_sum: 0,
                use_composition_time_offset: false,
            };
            fragment.duration_sum = fragment
                .duration_sum
                .checked_add(sample.duration as u64)
                .ok_or(Fmp4MuxError::DecodeTimeOverflow {
                    track_id: sample.track_id,
                })?;
            fragment.data_size_sum = fragment
                .data_size_sum
                .checked_add(sample.data_size as u64)
                .ok_or(Fmp4MuxError::MediaDataSizeOverflow)?;
            fragment.use_composition_time_offset = sample.composition_time_offset.is_some();
            fragment.samples.push(sample);
            fragments.push(fragment);
        }

        Ok(fragments)
    }

    fn build_traf_boxes(
        &self,
        fragments: &[TrackFragment<'_>],
        data_offsets: Option<&[i32]>,
    ) -> Result<Vec<TrafBox>, Fmp4MuxError> {
        let mut traf_boxes = Vec::new();
        for (index, fragment) in fragments.iter().enumerate() {
            let data_offset = data_offsets
                .and_then(|offsets| offsets.get(index).copied())
                .unwrap_or(0);
            let trun_box = TrunBox {
                data_offset: Some(data_offset),
                first_sample_flags: None,
                samples: build_trun_samples(fragment),
            };

            let tfhd_box = TfhdBox {
                track_id: fragment.track_id,
                base_data_offset: None,
                sample_description_index: None,
                default_sample_duration: None,
                default_sample_size: None,
                default_sample_flags: None,
                duration_is_empty: false,
                default_base_is_moof: true,
            };
            let tfdt_box = TfdtBox {
                version: 0,
                base_media_decode_time: fragment.base_decode_time,
            };

            traf_boxes.push(TrafBox {
                tfhd_box,
                tfdt_box: Some(tfdt_box),
                trun_boxes: vec![trun_box],
                unknown_boxes: Vec::new(),
            });
        }
        Ok(traf_boxes)
    }

    fn apply_fragment_updates(
        &mut self,
        fragments: &[TrackFragment<'_>],
    ) -> Result<(), Fmp4MuxError> {
        let mut updates = Vec::new();
        for fragment in fragments {
            let next_decode_time = fragment
                .base_decode_time
                .checked_add(fragment.duration_sum)
                .ok_or(Fmp4MuxError::DecodeTimeOverflow {
                    track_id: fragment.track_id,
                })?;
            updates.push((fragment.track_index, next_decode_time));
        }

        for (track_index, next_decode_time) in updates {
            let state = self
                .track_states
                .get_mut(track_index)
                .expect("track state should exist");
            state.next_decode_time = next_decode_time;
        }
        Ok(())
    }
}

fn build_trun_samples(fragment: &TrackFragment<'_>) -> Vec<TrunSample> {
    let mut trun_samples = Vec::new();
    for sample in &fragment.samples {
        let flags = sample
            .sample_flags
            .unwrap_or_else(|| sample_flags_from_keyframe(sample.keyframe));
        let composition_time_offset = if fragment.use_composition_time_offset {
            Some(sample.composition_time_offset.unwrap_or(0))
        } else {
            None
        };
        trun_samples.push(TrunSample {
            duration: Some(sample.duration),
            size: Some(sample.data_size),
            flags: Some(flags),
            composition_time_offset,
        });
    }
    trun_samples
}

fn sample_flags_from_keyframe(keyframe: bool) -> SampleFlags {
    let sample_depends_on = if keyframe { 2 } else { 1 };
    SampleFlags::from_fields(0, sample_depends_on, 0, 0, 0, !keyframe, 0)
}

fn build_mdat_header_bytes(total_data_size: u64) -> Result<Vec<u8>, Fmp4MuxError> {
    let box_size = BoxSize::with_payload_size(MdatBox::TYPE, total_data_size);
    let header = BoxHeader::new(MdatBox::TYPE, box_size);
    Ok(header.encode_to_vec()?)
}

fn compute_data_offsets(
    fragments: &[TrackFragment<'_>],
    moof_size: u64,
    mdat_header_size: u64,
) -> Result<Vec<i32>, Fmp4MuxError> {
    let mut offsets = Vec::new();
    let mut running_offset = 0u64;
    let base_offset = moof_size
        .checked_add(mdat_header_size)
        .ok_or(Fmp4MuxError::MediaDataSizeOverflow)?;

    for fragment in fragments {
        let data_offset = base_offset
            .checked_add(running_offset)
            .ok_or(Fmp4MuxError::MediaDataSizeOverflow)?;
        if data_offset > i32::MAX as u64 {
            return Err(Fmp4MuxError::DataOffsetTooLarge { data_offset });
        }
        offsets.push(data_offset as i32);
        running_offset = running_offset
            .checked_add(fragment.data_size_sum)
            .ok_or(Fmp4MuxError::MediaDataSizeOverflow)?;
    }

    Ok(offsets)
}

fn validate_tracks(tracks: &[TrackConfig]) -> Result<(), Fmp4MuxError> {
    if tracks.is_empty() {
        return Err(Fmp4MuxError::EmptyTracks);
    }

    let mut seen_tracks = Vec::new();
    for track in tracks {
        if track.track_id == 0 {
            return Err(Fmp4MuxError::InvalidTrackId {
                track_id: track.track_id,
            });
        }
        if seen_tracks.contains(&track.track_id) {
            return Err(Fmp4MuxError::DuplicateTrackId {
                track_id: track.track_id,
            });
        }
        seen_tracks.push(track.track_id);
    }

    Ok(())
}

fn build_init_segment(
    tracks: &[TrackConfig],
    options: &Mp4FragmentedFileMuxerOptions,
) -> Result<Vec<u8>, Fmp4MuxError> {
    let ftyp_box = FtypBox {
        major_brand: options.major_brand,
        minor_version: options.minor_version,
        compatible_brands: options.compatible_brands.clone(),
    };

    let creation_time = Mp4FileTime::from_unix_time(options.creation_timestamp);
    let mvhd_timescale = tracks.first().expect("tracks must not be empty").timescale;
    let max_track_id = tracks
        .iter()
        .map(|track| track.track_id)
        .max()
        .expect("tracks must not be empty");
    let next_track_id = max_track_id
        .checked_add(1)
        .ok_or(Fmp4MuxError::NextTrackIdOverflow)?;

    let mvhd_box = MvhdBox {
        creation_time,
        modification_time: creation_time,
        timescale: mvhd_timescale,
        duration: 0,
        rate: MvhdBox::DEFAULT_RATE,
        volume: MvhdBox::DEFAULT_VOLUME,
        matrix: MvhdBox::DEFAULT_MATRIX,
        next_track_id,
    };

    let mut trak_boxes = Vec::new();
    for track in tracks {
        trak_boxes.push(build_trak_box(track, creation_time)?);
    }

    let trex_boxes = tracks
        .iter()
        .map(|track| TrexBox {
            track_id: track.track_id,
            default_sample_description_index: 1,
            default_sample_duration: 0,
            default_sample_size: 0,
            default_sample_flags: SampleFlags::empty(),
        })
        .collect();

    let mvex_box = MvexBox {
        mehd_box: None,
        trex_boxes,
        unknown_boxes: Vec::new(),
    };

    let moov_box = MoovBox {
        mvhd_box,
        trak_boxes,
        mvex_box: Some(mvex_box),
        unknown_boxes: Vec::new(),
    };

    let mut bytes = ftyp_box.encode_to_vec()?;
    bytes.extend_from_slice(&moov_box.encode_to_vec()?);
    Ok(bytes)
}

fn build_trak_box(
    track: &TrackConfig,
    creation_time: Mp4FileTime,
) -> Result<TrakBox, Fmp4MuxError> {
    let (width, height) = if let TrackKind::Video = track.kind {
        track.sample_entry.video_resolution().unwrap_or((0, 0))
    } else {
        (0, 0)
    };

    let tkhd_box = TkhdBox {
        flag_track_enabled: true,
        flag_track_in_movie: true,
        flag_track_in_preview: false,
        flag_track_size_is_aspect_ratio: false,
        creation_time,
        modification_time: creation_time,
        track_id: track.track_id,
        duration: 0,
        layer: TkhdBox::DEFAULT_LAYER,
        alternate_group: TkhdBox::DEFAULT_ALTERNATE_GROUP,
        volume: match track.kind {
            TrackKind::Audio => TkhdBox::DEFAULT_AUDIO_VOLUME,
            TrackKind::Video => TkhdBox::DEFAULT_VIDEO_VOLUME,
        },
        matrix: TkhdBox::DEFAULT_MATRIX,
        width: FixedPointNumber::new(width as i16, 0),
        height: FixedPointNumber::new(height as i16, 0),
    };

    Ok(TrakBox {
        tkhd_box,
        edts_box: None,
        mdia_box: build_mdia_box(track, creation_time)?,
        unknown_boxes: Vec::new(),
    })
}

fn build_mdia_box(
    track: &TrackConfig,
    creation_time: Mp4FileTime,
) -> Result<MdiaBox, Fmp4MuxError> {
    let mdhd_box = MdhdBox {
        creation_time,
        modification_time: creation_time,
        timescale: track.timescale,
        duration: 0,
        language: MdhdBox::LANGUAGE_UNDEFINED,
    };

    let (handler_type, minf_box) = match track.kind {
        TrackKind::Audio => (
            HdlrBox::HANDLER_TYPE_SOUN,
            MinfBox {
                smhd_or_vmhd_box: Some(Either::A(SmhdBox::default())),
                dinf_box: DinfBox::LOCAL_FILE,
                stbl_box: build_empty_stbl_box(track)?,
                unknown_boxes: Vec::new(),
            },
        ),
        TrackKind::Video => (
            HdlrBox::HANDLER_TYPE_VIDE,
            MinfBox {
                smhd_or_vmhd_box: Some(Either::B(VmhdBox::default())),
                dinf_box: DinfBox::LOCAL_FILE,
                stbl_box: build_empty_stbl_box(track)?,
                unknown_boxes: Vec::new(),
            },
        ),
    };

    let hdlr_box = HdlrBox {
        handler_type,
        name: Utf8String::EMPTY.into_null_terminated_bytes(),
    };

    Ok(MdiaBox {
        mdhd_box,
        hdlr_box,
        minf_box,
        unknown_boxes: Vec::new(),
    })
}

fn build_empty_stbl_box(track: &TrackConfig) -> Result<StblBox, Fmp4MuxError> {
    let stsd_box = StsdBox {
        entries: vec![track.sample_entry.clone()],
    };

    let stts_box = SttsBox {
        entries: Vec::new(),
    };
    let stsc_box = StscBox {
        entries: Vec::new(),
    };
    let stsz_box = StszBox::Variable {
        entry_sizes: Vec::new(),
    };
    let stco_box = StcoBox {
        chunk_offsets: Vec::new(),
    };

    Ok(StblBox {
        stsd_box,
        stts_box,
        stsc_box,
        stsz_box,
        stco_or_co64_box: Either::A(stco_box),
        stss_box: None,
        unknown_boxes: Vec::new(),
    })
}
