//! MP4 ファイルのデマルチプレックス（分離）機能を提供するモジュール
//!
//! このモジュールは、MP4ファイルに含まれる複数のメディアトラック（音声・映像）から
//! 時系列順にサンプルを抽出するための機能を提供する。
//!
//! # Examples
//!
//! MP4 ファイル全体をメモリに読み込んでデマルチプレックスする例：
//!
//! ```no_run
//! use shiguredo_mp4::demux::{Mp4FileDemuxer, Input};
//!
//! // MP4 ファイル全体をメモリに読み込む
//! let file_data = std::fs::read("sample.mp4").expect("ファイル読み込み失敗");
//!
//! // デマルチプレックス処理を初期化し、ファイルデータ全体を提供する
//! let mut demuxer = Mp4FileDemuxer::new();
//! let input = Input {
//!     position: 0,
//!     data: &file_data,
//! };
//! demuxer.handle_input(input);
//!
//! // トラック情報を取得する
//! let tracks = demuxer.tracks().expect("トラック取得失敗");
//! println!("{}個のトラックが見つかりました", tracks.len());
//! for track in tracks {
//!     println!("トラックID: {}, 種類: {:?}, 尺: {}, タイムスケール: {}",
//!              track.track_id, track.kind, track.duration, track.timescale);
//! }
//!
//! // 時系列順にサンプルを抽出する
//! while let Some(sample) = demuxer.next_sample().expect("サンプル読み込み失敗") {
//!     println!("サンプル - トラックID: {}, タイムスタンプ: {}, サイズ: {} バイト",
//!              sample.track.track_id, sample.timestamp, sample.data_size);
//!     // sample.data_offset の位置から sample.data_size バイトのサンプルデータにアクセス
//!     let sample_data = &file_data[sample.data_offset as usize..
//!                                  sample.data_offset as usize + sample.data_size];
//!     // sample_data を処理...
//! }
//! ```
use core::{num::NonZeroU32, time::Duration};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::{
    BoxHeader, Decode, Error, TrackKind,
    aux::{SampleTableAccessor, SampleTableAccessorError},
    boxes::{FtypBox, HdlrBox, MoovBox, SampleEntry, StblBox},
};

/// メディアトラックの情報を表す構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackInfo {
    /// トラックID
    pub track_id: u32,

    /// トラックの種類
    pub kind: TrackKind,

    /// トラックの尺（タイムスケール単位）
    ///
    /// 秒単位の尺は、この値を `timescale` で割ることで求められる
    pub duration: u64,

    /// トラックで使用されているタイムスケール
    pub timescale: NonZeroU32,
}

/// MP4 ファイルから抽出されたメディアサンプルを表す構造体
///
/// この構造体は MP4 ファイル内の各サンプル（フレーム単位の音声または映像データ）の
/// メタデータとデータ位置情報を保持する
#[derive(Debug, Clone)]
pub struct Sample<'a> {
    /// サンプルが属するトラックの情報
    pub track: &'a TrackInfo,

    /// サンプルの詳細情報
    ///
    /// 前のサンプルから変更がない場合には None になる（最初のサンプルは常に Some となる）
    pub sample_entry: Option<&'a SampleEntry>,

    /// キーフレームであるかの判定
    pub keyframe: bool,

    /// サンプルのタイムスタンプ（トラックのタイムスケール単位）
    ///
    /// 秒単位のタイムスタンプは、この値を `track.timescale` で割ることで求められる
    ///
    /// なお、この値は「同じトラック内の前方に位置するサンプルの尺の累積値」を計算することで求めらている
    pub timestamp: u64,

    /// サンプルの尺（トラックのタイムスケール単位）
    ///
    /// 秒単位の尺は、この値を `track.timescale` で割ることで求められる
    pub duration: u32,

    /// ファイル内におけるサンプルデータの開始位置（バイト単位）
    pub data_offset: u64,

    /// サンプルデータのサイズ（バイト単位）
    pub data_size: usize,
}

/// デマルチプレックスに必要な入力データの情報を表す構造体
///
/// この構造体は、デマルチプレックス処理を進めるために読み込む必要があるデータの
/// 位置とサイズを示す。この情報をもとに呼び出し元がファイルなどからデータを読み込み、
/// [`Mp4FileDemuxer::handle_input()`] に渡す
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequiredInput {
    /// 必要なデータの開始位置（バイト単位）
    pub position: u64,

    /// 必要なデータのサイズ（バイト単位）
    ///
    ///
    /// ここで指定されたサイズはあくまでもヒントであり、厳密に一致したサイズのデータを提供する必要はない
    /// （特に指定サイズよりも大きいデータを呼び出し元がすでに保持している場合には、それをそのまま渡した方が効率がいい）
    ///
    /// `None` はファイルなどの末尾までを意味する
    pub size: Option<usize>,
}

impl RequiredInput {
    const fn new(position: u64, size: Option<usize>) -> Self {
        Self { position, size }
    }

    /// この [`RequiredInput`] インスタンスを、提供されたデータを使用して [`Input`] に変換する
    pub const fn to_input<'a>(self, data: &'a [u8]) -> Input<'a> {
        Input {
            position: self.position,
            data,
        }
    }
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
    table: SampleTableAccessor<StblBox>,
    timescale: NonZeroU32,
    next_sample_index: NonZeroU32,
}

/// MP4 デマルチプレックス処理中に発生するエラーを表す列挙型
#[non_exhaustive]
pub enum DemuxError {
    /// MP4 ボックスのデコード処理中に発生したエラー
    DecodeError(Error),

    /// サンプルテーブル処理中に発生したエラー
    SampleTableError(SampleTableAccessorError),

    /// 入力データの読み込みが必要なことを示すエラー
    ///
    /// このエラーが返された場合、呼び出し元は指定された位置とサイズのファイルデータを
    /// 読み込み、[`Mp4FileDemuxer::handle_input()`] に渡す必要がある
    ///
    /// なお I/O が必要になる可能性がある各メソッドを使用する前に [`Mp4FileDemuxer::required_input()`] を呼び出すことで、
    /// 事前に必要なデータを [`Mp4FileDemuxer`] に供給するが可能となる
    InputRequired(RequiredInput),
}

impl DemuxError {
    fn input_required(position: u64, size: Option<usize>) -> Self {
        Self::InputRequired(RequiredInput::new(position, size))
    }
}

impl From<Error> for DemuxError {
    fn from(error: Error) -> Self {
        DemuxError::DecodeError(error)
    }
}

impl From<SampleTableAccessorError> for DemuxError {
    fn from(error: SampleTableAccessorError) -> Self {
        DemuxError::SampleTableError(error)
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
            DemuxError::SampleTableError(error) => {
                write!(f, "Sample table error: {error}")
            }
            DemuxError::InputRequired(required) => match required.size {
                Some(s) => write!(
                    f,
                    "Need input data: {s} bytes at position {}",
                    required.position
                ),
                None => write!(
                    f,
                    "Need input data: from position {} to end of file",
                    required.position
                ),
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DemuxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DemuxError::DecodeError(error) => Some(error),
            DemuxError::SampleTableError(error) => Some(error),
            _ => None,
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
///
/// また、この構造体はストリーミング用途での使用は想定していない。
#[derive(Debug)]
pub struct Mp4FileDemuxer {
    phase: Phase,
    track_infos: Vec<TrackInfo>,
    tracks: Vec<TrackState>,
    handle_input_error: Option<DemuxError>,
    /// 前回の `handle_input()` 呼び出し時の状態を記録する
    ///
    /// (返された RequiredInput, 提供された Input の position, 提供された Input の data.len())
    /// を記録し、同じ RequiredInput が返り、かつ前回のデータが要求を満たしていた場合に
    /// 無限ループとしてエラーを返す
    last_input_state: Option<(RequiredInput, u64, usize)>,
}

impl Mp4FileDemuxer {
    /// 新しい [`Mp4FileDemuxer`] インスタンスを生成する
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            phase: Phase::ReadFtypBoxHeader,
            track_infos: Vec::new(),
            tracks: Vec::new(),
            handle_input_error: None,
            last_input_state: None,
        }
    }

    /// 次の処理を進めるために必要な I/O の位置とサイズを返す
    ///
    /// デマルチプレックス処理が初期化済みの場合は `None` を返す。
    /// それ以外の場合は、必要なファイルデータの情報を返す。
    pub fn required_input(&self) -> Option<RequiredInput> {
        if self.handle_input_error.is_some() {
            return None;
        }

        match self.phase {
            Phase::ReadFtypBoxHeader => Some(RequiredInput::new(0, Some(BoxHeader::MAX_SIZE))),
            Phase::ReadFtypBox { box_size } => Some(RequiredInput::new(0, box_size)),
            Phase::ReadMoovBoxHeader { offset } => {
                Some(RequiredInput::new(offset, Some(BoxHeader::MAX_SIZE)))
            }
            Phase::ReadMoovBox { offset, box_size } => Some(RequiredInput::new(offset, box_size)),
            Phase::Initialized => None,
        }
    }

    /// ファイルデータを入力として受け取り、デマルチプレックス処理を進める
    pub fn handle_input(&mut self, input: Input) {
        match self.handle_input_inner(input) {
            Ok(()) => {
                // 入力処理が成功したので、前回の状態をクリアする
                self.last_input_state = None;
            }
            Err(DemuxError::InputRequired(required)) => {
                // 前回の状態と比較して無限ループを検出する
                let is_infinite_loop = self.last_input_state.as_ref().is_some_and(
                    |(last_required, last_pos, last_len)| {
                        // 同じ RequiredInput が返された
                        *last_required == required
                            // かつ、前回提供されたデータの位置が要求と一致
                            && *last_pos == required.position
                            // かつ、前回提供されたデータが要求サイズを満たしていた
                            && required
                                .size
                                .is_none_or(|required_size| *last_len >= required_size)
                    },
                );

                if is_infinite_loop {
                    self.handle_input_error = Some(DemuxError::DecodeError(Error::invalid_data(
                        "no progress despite sufficient input (possibly corrupted data)",
                    )));
                } else {
                    // 今回の状態を記録
                    self.last_input_state = Some((required, input.position, input.data.len()));
                }
            }
            Err(e) => {
                // 入力処理中に（入力不足以外の）エラーが出た場合は required_input() との
                // 相互呼び出しで無限ループするのを避けるためにエラー情報を覚えておく
                // （このメソッド自体が Result を返すと、呼び出し元のハンドリングが複雑になるのでそれは避ける）
                self.handle_input_error = Some(e);
            }
        }
    }

    fn handle_input_inner(&mut self, input: Input) -> Result<(), DemuxError> {
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
            return Err(DemuxError::input_required(0, data_size));
        };
        let (header, _header_size) = BoxHeader::decode(data)?;
        header.box_type.expect(FtypBox::TYPE)?;

        let box_size = Some(header.box_size.get() as usize).filter(|n| *n > 0);
        self.phase = Phase::ReadFtypBox { box_size };
        self.handle_input_inner(input)
    }

    fn read_ftyp_box(&mut self, input: Input) -> Result<(), DemuxError> {
        let Phase::ReadFtypBox { box_size } = self.phase else {
            panic!("bug");
        };
        let Some(data) = input.slice_range(0, box_size) else {
            return Err(DemuxError::input_required(0, box_size));
        };
        let (_ftyp_box, ftyp_box_size) = FtypBox::decode(data)?;
        self.phase = Phase::ReadMoovBoxHeader {
            offset: ftyp_box_size as u64,
        };
        self.handle_input_inner(input)
    }

    fn read_moov_box_header(&mut self, input: Input) -> Result<(), DemuxError> {
        let Phase::ReadMoovBoxHeader { offset } = self.phase else {
            panic!("bug");
        };

        let data_size = Some(BoxHeader::MAX_SIZE);
        let Some(data) = input.slice_range(offset, data_size) else {
            return Err(DemuxError::input_required(offset, data_size));
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
        self.handle_input_inner(input)
    }

    fn read_moov_box(&mut self, input: Input) -> Result<(), DemuxError> {
        let Phase::ReadMoovBox { offset, box_size } = self.phase else {
            panic!("bug");
        };

        let Some(data) = input.slice_range(offset, box_size) else {
            return Err(DemuxError::input_required(offset, box_size));
        };
        let (moov_box, _moov_box_size) = MoovBox::decode(data)?;

        for trak_box in moov_box.trak_boxes {
            let track_id = trak_box.tkhd_box.track_id;
            let kind = match trak_box.mdia_box.hdlr_box.handler_type {
                HdlrBox::HANDLER_TYPE_VIDE => TrackKind::Video,
                HdlrBox::HANDLER_TYPE_SOUN => TrackKind::Audio,
                _ => continue,
            };
            let timescale = trak_box.mdia_box.mdhd_box.timescale;
            let table = SampleTableAccessor::new(trak_box.mdia_box.minf_box.stbl_box)?;
            self.track_infos.push(TrackInfo {
                track_id,
                kind,
                duration: trak_box.mdia_box.mdhd_box.duration,
                timescale,
            });
            self.tracks.push(TrackState {
                table,
                timescale,
                next_sample_index: NonZeroU32::MIN,
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
        self.ensure_initialized()?;
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
        self.ensure_initialized()?;

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
        if let Some((_timestamp, track_index)) = earliest_sample {
            let sample_index = self.tracks[track_index].next_sample_index;
            self.tracks[track_index].next_sample_index =
                sample_index.checked_add(1).ok_or_else(|| {
                    DemuxError::DecodeError(Error::invalid_data("sample index overflow"))
                })?;

            let track = &mut self.tracks[track_index];
            let sample_accessor = track.table.get_sample(sample_index).expect("bug");
            let sample_entry = sample_accessor.chunk().sample_entry();
            let sample_entry_index = sample_accessor.chunk().sample_entry_index();

            // サンプルエントリーに変更があるかどうかをチェックする
            // NOTE: 将来的にシークに対応する場合には、シーク直後は常に新規扱いにする必要がある
            let is_new_sample_entry = if let Some(prev_sample_index) =
                NonZeroU32::new(sample_index.get() - 1)
            {
                let prev_sample_accessor = track.table.get_sample(prev_sample_index).expect("bug");
                if prev_sample_accessor.chunk().sample_entry_index() == sample_entry_index {
                    // サンプルエントリーのインデックスが等しい場合には、内容も常に等しい
                    false
                } else {
                    prev_sample_accessor.chunk().sample_entry() != sample_entry
                }
            } else {
                // 最初のサンプル
                true
            };

            let sample = Sample {
                track: &self.track_infos[track_index],
                sample_entry: is_new_sample_entry.then_some(sample_entry),
                keyframe: sample_accessor.is_sync_sample(),
                timestamp: sample_accessor.timestamp(),
                duration: sample_accessor.duration(),
                data_offset: sample_accessor.data_offset(),
                data_size: sample_accessor.data_size() as usize,
            };
            Ok(Some(sample))
        } else {
            Ok(None)
        }
    }

    fn ensure_initialized(&mut self) -> Result<(), DemuxError> {
        if let Some(e) = self.handle_input_error.take() {
            Err(e)
        } else if let Some(required) = self.required_input() {
            Err(DemuxError::InputRequired(required))
        } else {
            Ok(())
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
        demuxer.handle_input(input);

        let tracks = demuxer.tracks().expect("failed to get tracks").to_vec();

        let mut sample_count = 0;
        let mut keyframe_count = 0;
        while let Some(sample) = demuxer.next_sample().expect("failed to read samples") {
            assert!(sample.data_size > 0);
            assert!(sample.duration > 0);
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
