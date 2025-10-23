//! MP4 ファイルのデマルチプレックス（分離）機能を提供するモジュール
//!
//! このモジュールは、MP4ファイルに含まれる複数のメディアトラック（音声・映像）から
//! 時系列順にサンプルを抽出するための機能を提供する。
use core::{num::NonZeroU32, time::Duration};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::{
    BoxHeader, Decode, Error,
    aux::SampleTableAccessor,
    boxes::{FtypBox, HdlrBox, MoovBox, SampleEntry, StblBox},
};

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
    /// トラックID
    pub track_id: u32,

    /// トラックの種類
    pub kind: TrackKind,

    /// トラックの尺
    pub duration: Duration,
}

/// MP4 ファイルから抽出されたメディアサンプルを表す構造体
///
/// この構造体は MP4 ファイル内の各サンプル（フレーム単位の音声または映像データ）の
/// メタデータとデータ位置情報を保持する
///
/// # NOTE
///
/// サンプルのタイムスタンプと尺は、MP4 ファイル内ではトラックのタイムスケールに基づいた整数単位で格納されているが、
/// このフィールドでは [`Duration`] で表現されている。
///
/// [`Duration`] の時間単位は固定的なものであるため、タイムスケールの値によっては
/// 変換時に若干の誤差が生じる可能性がある。
/// しかし実用上、通常はこの誤差は無視できる程度のものであるため、API の利便性を優先してこの設計としている。
///
/// もし誤差が許容できないユースケースの場合は、[`Mp4FileDemuxer`] は使用せずに、
/// 直接ボックスを操作して、生のサンプルデータを取得することを推奨する。
#[derive(Debug, Clone)]
pub struct Sample<'a> {
    /// サンプルが属するトラックの ID
    pub track_id: u32,

    /// サンプルの詳細情報
    pub sample_entry: &'a SampleEntry,

    /// キーフレームであるかの判定
    pub keyframe: bool,

    /// サンプルのタイムスタンプ
    pub timestamp: Duration,

    /// サンプルの尺
    pub duration: Duration,

    /// ファイル内におけるサンプルデータの開始位置（バイト単位）
    pub data_offset: u64,

    /// サンプルデータのサイズ（バイト単位）
    pub data_size: usize,
}

/// [`Mp4FileDemuxer::handle_input()`] に渡す入力データを表す構造体
///
/// [`Mp4FileDemuxer`] 自体は I/O 操作を行わず、各メソッドで I/O 操作が必要になった場合には、
/// [`DemuxError::NeedInput`] を通して呼び出し元への要求が発行される。
/// この構造体は、その要求をうけた呼び出し元が I/O 操作の結果を [`Mp4FileDemuxer`] に伝えるために使用される。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input<'a> {
    /// バッファ内のデータがファイル内で始まる位置（バイト単位）
    pub position: u64,

    /// ファイルデータのバッファ
    pub data: &'a [u8],
}

impl<'a> Input<'a> {
    fn slice_range(self, position: u64, size: Option<usize>) -> Option<&'a [u8]> {
        let offset = position.checked_sub(self.position)? as usize;
        if offset > self.data.len() {
            return None;
        }

        if let Some(size) = size {
            self.data.get(offset..offset + size)
        } else {
            Some(&self.data[offset..])
        }
    }
}

#[derive(Debug)]
struct TrackState {
    track_id: u32,
    table: SampleTableAccessor<StblBox>,
    next_sample_index: NonZeroU32,
    timescale: NonZeroU32,
}

/// MP4 デマルチプレックス処理中に発生するエラーを表す列挙型
pub enum DemuxError {
    /// MP4 ボックスのデコード処理中に発生したエラー
    DecodeError(Error),

    /// ファイルデータの読み込みが必要なことを示すエラー
    ///
    /// このエラーが返された場合、呼び出し元は指定された位置とサイズのファイルデータを
    /// 読み込み、[`Mp4FileDemuxer::handle_input()`] に渡す必要がある。
    NeedInput {
        /// 必要なデータの開始位置（バイト単位）
        position: u64,

        /// 必要なデータのサイズ（バイト単位）
        ///
        /// `None` はファイルの末尾までを意味する
        ///
        /// なお、ここで指定されたサイズよりも大きいデータを
        /// [`Mp4FileDemuxer::handle_input()`] に渡しても問題はない
        size: Option<usize>,
    },
}

impl DemuxError {
    fn need_input(position: u64, size: Option<usize>) -> Self {
        Self::NeedInput { position, size }
    }
}

impl From<Error> for DemuxError {
    fn from(error: Error) -> Self {
        DemuxError::DecodeError(error)
    }
}

impl core::fmt::Debug for DemuxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
    }
}

impl core::fmt::Display for DemuxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DemuxError::DecodeError(error) => {
                write!(f, "Failed to decode MP4 box: {error}")
            }
            DemuxError::NeedInput { position, size } => match size {
                Some(s) => write!(f, "Need input data: {s} bytes at position {position}"),
                None => write!(
                    f,
                    "Need input data: from position {position} to end of file",
                ),
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DemuxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let DemuxError::DecodeError(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    ReadFtypBoxHeader,
    ReadFtypBox {
        box_size: Option<usize>,
    },
    ReadMoovBoxHeader {
        offset: u64,
    },
    ReadMoovBox {
        offset: u64,
        box_size: Option<usize>,
    },
    Initialized,
}

/// MP4 ファイルをデマルチプレックスして、メディアサンプルを取得するための構造体
///
/// この構造体は段階的にファイルデータを処理し、複数のメディアトラックから
/// 時系列順にサンプルを抽出する機能を提供する。
///
/// なお、この構造体自体は I/O 操作は行わないため、
/// ファイル読み込みなどを必要に応じて行うのは利用側の責務となっている。
#[derive(Debug)]
pub struct Mp4FileDemuxer {
    phase: Phase,
    track_infos: Vec<TrackInfo>,
    tracks: Vec<TrackState>,
}

impl Mp4FileDemuxer {
    /// 新しい [`Mp4FileDemuxer`] インスタンスを生成する
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            phase: Phase::ReadFtypBoxHeader,
            track_infos: Vec::new(),
            tracks: Vec::new(),
        }
    }

    /// ファイルデータを入力として受け取り、デマルチプレックス処理を進める
    ///
    /// さらなるデータの読み込みが必要な場合は [`DemuxError::NeedInput`] が返される。
    /// その場合、呼び出し元は指定された位置とサイズのファイルデータを読み込み、
    /// 再度このメソッドを呼び出す必要がある。
    pub fn handle_input(&mut self, input: Input) -> Result<(), DemuxError> {
        match self.phase {
            Phase::ReadFtypBoxHeader => self.read_ftyp_box_header(input),
            Phase::ReadFtypBox { .. } => self.read_ftyp_box(input),
            Phase::ReadMoovBoxHeader { .. } => self.read_moov_box_header(input),
            Phase::ReadMoovBox { .. } => self.read_moov_box(input),
            Phase::Initialized => Ok(()),
        }
    }

    fn read_ftyp_box_header(&mut self, input: Input) -> Result<(), DemuxError> {
        assert!(matches!(self.phase, Phase::ReadFtypBoxHeader));

        let data_size = Some(BoxHeader::MAX_SIZE);
        let Some(data) = input.slice_range(0, data_size) else {
            return Err(DemuxError::need_input(0, data_size));
        };
        let (header, _header_size) = BoxHeader::decode(data)?;
        header.box_type.expect(FtypBox::TYPE)?;

        let box_size = Some(header.box_size.get() as usize).filter(|n| *n > 0);
        self.phase = Phase::ReadFtypBox { box_size };
        self.handle_input(input)
    }

    fn read_ftyp_box(&mut self, input: Input) -> Result<(), DemuxError> {
        let Phase::ReadFtypBox { box_size } = self.phase else {
            panic!("bug");
        };
        let Some(data) = input.slice_range(0, box_size) else {
            return Err(DemuxError::need_input(0, box_size));
        };
        let (_ftyp_box, ftyp_box_size) = FtypBox::decode(data)?;
        self.phase = Phase::ReadMoovBoxHeader {
            offset: ftyp_box_size as u64,
        };
        self.handle_input(input)
    }

    fn read_moov_box_header(&mut self, input: Input) -> Result<(), DemuxError> {
        let Phase::ReadMoovBoxHeader { offset } = self.phase else {
            panic!("bug");
        };

        let data_size = Some(BoxHeader::MAX_SIZE);
        let Some(data) = input.slice_range(offset, data_size) else {
            return Err(DemuxError::need_input(offset, data_size));
        };

        let (header, _header_size) = BoxHeader::decode(data)?;
        let box_size = Some(header.box_size.get()).filter(|n| *n > 0);

        if header.box_type != MoovBox::TYPE {
            let Some(box_size) = box_size else {
                return Err(DemuxError::DecodeError(Error::invalid_data(
                    "moov box not found",
                )));
            };
            let offset = offset + box_size;
            self.phase = Phase::ReadMoovBoxHeader { offset };
        } else {
            let box_size = box_size.map(|n| n as usize);
            self.phase = Phase::ReadMoovBox { offset, box_size };
        }
        self.handle_input(input)
    }

    fn read_moov_box(&mut self, input: Input) -> Result<(), DemuxError> {
        let Phase::ReadMoovBox { offset, box_size } = self.phase else {
            panic!("bug");
        };

        let Some(data) = input.slice_range(offset, box_size) else {
            return Err(DemuxError::need_input(offset, box_size));
        };
        let (moov_box, _moov_box_size) = MoovBox::decode(data)?;

        for trak_box in moov_box.trak_boxes {
            let track_id = trak_box.tkhd_box.track_id;
            let kind = match trak_box.mdia_box.hdlr_box.handler_type {
                HdlrBox::HANDLER_TYPE_VIDE => TrackKind::Video,
                HdlrBox::HANDLER_TYPE_SOUN => TrackKind::Audio,
                _ => continue,
            };
            let timescale = trak_box.mdia_box.mdhd_box.timescale.get();
            let duration = Duration::from_secs(trak_box.mdia_box.mdhd_box.duration) / timescale;
            let Ok(table) = SampleTableAccessor::new(trak_box.mdia_box.minf_box.stbl_box) else {
                continue;
            };
            self.track_infos.push(TrackInfo {
                track_id,
                kind,
                duration,
            });
            self.tracks.push(TrackState {
                track_id,
                table,
                next_sample_index: NonZeroU32::MIN,
                timescale: trak_box.mdia_box.mdhd_box.timescale,
            })
        }

        self.phase = Phase::Initialized;
        Ok(())
    }

    /// MP4 ファイル内のすべてのメディアトラック情報を取得する
    ///
    /// なお、トラック情報を取得するために I/O 操作が必要な場合は [`DemuxError::NeedInput`] が返される。
    /// その場合、呼び出し元は指定された位置とサイズのファイルデータを読み込み、
    /// [`handle_input()`] に渡した後、再度このメソッドを呼び出す必要がある。
    pub fn tracks(&mut self) -> Result<&[TrackInfo], DemuxError> {
        self.initialize_if_need()?;
        Ok(&self.track_infos)
    }

    /// 時系列順に次のサンプルを取得する
    ///
    /// すべてのトラックから、まだ取得していないものの中で、
    /// 最も早いタイムスタンプを持つサンプルを返す。
    /// サンプルが存在しない場合は `None` が返される。
    ///
    /// なお、次のサンプルの情報を取得するために I/O 操作が必要な場合は [`DemuxError::NeedInput`] が返される。
    /// その場合、呼び出し元は指定された位置とサイズのファイルデータを読み込み、
    /// [`handle_input()`] に渡した後、再度このメソッドを呼び出す必要がある。
    pub fn next_sample(&mut self) -> Result<Option<Sample<'_>>, DemuxError> {
        self.initialize_if_need()?;

        let mut earliest_sample: Option<(Duration, usize)> = None;

        // 全トラックの中で最も早いタイムスタンプを持つサンプルを探す
        for (track_index, track) in self.tracks.iter().enumerate() {
            let Some(sample_accessor) = track.table.get_sample(track.next_sample_index) else {
                continue;
            };
            let timestamp =
                Duration::from_secs(sample_accessor.timestamp()) / track.timescale.get();
            if earliest_sample.as_ref().is_some_and(|s| timestamp >= s.0) {
                continue;
            }

            earliest_sample = Some((timestamp, track_index));
        }

        // 最も早いサンプルを提供したトラックを進める
        if let Some((timestamp, track_index)) = earliest_sample {
            let track_id = self.tracks[track_index].track_id;
            let sample_index = self.tracks[track_index].next_sample_index;
            let timescale = self.tracks[track_index].timescale;
            self.tracks[track_index].next_sample_index =
                sample_index.checked_add(1).ok_or_else(|| {
                    DemuxError::DecodeError(Error::invalid_data("sample index overflow"))
                })?;

            let sample_accessor = self.tracks[track_index]
                .table
                .get_sample(sample_index)
                .expect("bug");
            let duration = Duration::from_secs(sample_accessor.duration() as u64) / timescale.get();
            let sample = Sample {
                track_id,
                sample_entry: sample_accessor.chunk().sample_entry(),
                keyframe: sample_accessor.is_sync_sample(),
                timestamp,
                duration,
                data_offset: sample_accessor.data_offset(),
                data_size: sample_accessor.data_size() as usize,
            };
            Ok(Some(sample))
        } else {
            Ok(None)
        }
    }

    fn initialize_if_need(&mut self) -> Result<(), DemuxError> {
        match self.phase {
            Phase::ReadFtypBoxHeader => Err(DemuxError::need_input(0, Some(BoxHeader::MAX_SIZE))),
            Phase::ReadFtypBox { box_size } => Err(DemuxError::need_input(0, box_size)),
            Phase::ReadMoovBoxHeader { offset } => {
                Err(DemuxError::need_input(offset, Some(BoxHeader::MAX_SIZE)))
            }
            Phase::ReadMoovBox { offset, box_size } => {
                Err(DemuxError::need_input(offset, box_size))
            }
            Phase::Initialized => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_tracks_from_file_data(file_data: &[u8]) -> Vec<TrackInfo> {
        let input = Input {
            position: 0,
            data: &file_data,
        };
        let mut demuxer = Mp4FileDemuxer::new();
        demuxer.handle_input(input).expect("failed to handle input");

        let tracks = demuxer.tracks().expect("failed to get tracks").to_vec();

        let mut sample_count = 0;
        let mut keyframe_count = 0;
        while let Some(sample) = demuxer.next_sample().expect("failed to read samples") {
            assert!(sample.data_size > 0);
            assert!(sample.duration > Duration::ZERO);
            sample_count += 1;
            if sample.keyframe {
                keyframe_count += 1;
            }
        }
        assert_ne!(sample_count, 0);
        assert_ne!(keyframe_count, 0);

        tracks
    }

    #[test]
    fn test_read_aac_audio() {
        let tracks =
            read_tracks_from_file_data(include_bytes!("../tests/testdata/beep-aac-audio.mp4"));

        assert_eq!(tracks.len(), 1);
        assert!(matches!(tracks[0].kind, TrackKind::Audio));
    }

    #[test]
    fn test_read_opus_audio() {
        let tracks =
            read_tracks_from_file_data(include_bytes!("../tests/testdata/beep-opus-audio.mp4"));

        assert_eq!(tracks.len(), 1);
        assert!(matches!(tracks[0].kind, TrackKind::Audio));
    }

    #[test]
    fn test_read_h264_video() {
        let tracks =
            read_tracks_from_file_data(include_bytes!("../tests/testdata/black-h264-video.mp4"));

        assert_eq!(tracks.len(), 1);
        assert!(matches!(tracks[0].kind, TrackKind::Video));
    }

    #[test]
    fn test_read_h265_video() {
        let tracks =
            read_tracks_from_file_data(include_bytes!("../tests/testdata/black-h265-video.mp4"));

        assert_eq!(tracks.len(), 1);
        assert!(matches!(tracks[0].kind, TrackKind::Video));
    }

    #[test]
    fn test_read_vp9_video() {
        let tracks =
            read_tracks_from_file_data(include_bytes!("../tests/testdata/black-vp9-video.mp4"));

        assert_eq!(tracks.len(), 1);
        assert!(matches!(tracks[0].kind, TrackKind::Video));
    }

    #[test]
    fn test_read_av1_video() {
        let tracks =
            read_tracks_from_file_data(include_bytes!("../tests/testdata/black-av1-video.mp4"));

        assert_eq!(tracks.len(), 1);
        assert!(matches!(tracks[0].kind, TrackKind::Video));
    }
}
