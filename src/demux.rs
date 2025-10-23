#![allow(missing_docs)]

use core::{num::NonZeroU32, time::Duration};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::{
    BoxHeader, Decode, Error,
    aux::SampleTableAccessor,
    boxes::{FtypBox, HdlrBox, MoovBox, SampleEntry, StblBox},
};

#[derive(Debug, Clone)]
pub enum TrackKind {
    Audio,
    Video,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_id: u32,
    pub kind: TrackKind,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct Sample<'a> {
    pub track_id: u32,
    pub sample_entry: &'a SampleEntry,
    pub keyframe: bool,
    // NOTE:
    // `Duration` で表現するとタイムスケールの値によっては実際値と微妙にズレる可能性はあるが、
    // 実用上はまず問題がないはずなので、利便性を考慮して今の実装にしている
    pub timestamp: Duration,
    pub duration: Duration,
    pub data_offset: u64,
    pub data_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Input<'a> {
    pub position: u64,
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

// TODO: private
#[derive(Debug)]
pub struct TrackState {
    pub track_id: u32,
    pub table: SampleTableAccessor<StblBox>,
    next_sample_index: NonZeroU32,
    timescale: NonZeroU32,
}

#[derive(Debug)]
pub enum DemuxError {
    DecodeError(Error),
    NeedInput {
        position: u64,
        // None はファイルの末尾までを意味する
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

#[derive(Debug)]
pub struct Mp4FileDemuxer {
    phase: Phase,
    track_infos: Vec<TrackInfo>,
    tracks: Vec<TrackState>,
}

impl Mp4FileDemuxer {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            phase: Phase::ReadFtypBoxHeader,
            track_infos: Vec::new(),
            tracks: Vec::new(),
        }
    }

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

    pub fn tracks(&mut self) -> Result<&[TrackInfo], DemuxError> {
        self.initialize_if_need()?;
        Ok(&self.track_infos)
    }

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
