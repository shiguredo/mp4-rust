#![allow(missing_docs)]

use core::time::Duration;

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
pub struct Sample {
    pub track_id: u32,
    pub sample_entry: Option<SampleEntry>,
    pub keyframe: bool,
    pub timestamp: Duration,
    pub duration: Duration,
    pub data_offset: u64,
    pub data_size: usize,
}

#[derive(Debug)]
pub struct Input<'a> {
    pub position: u64,
    pub data: &'a [u8],
}

// TODO: private
#[derive(Debug)]
pub struct TrackState {
    pub track_id: u32,
    pub table: SampleTableAccessor<StblBox>,
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

    pub fn handle_input(&mut self, input: &Input) -> Result<(), DemuxError> {
        match self.phase {
            Phase::ReadFtypBoxHeader => self.read_ftyp_box_header(input),
            Phase::ReadFtypBox { .. } => self.read_ftyp_box(input),
            Phase::ReadMoovBoxHeader { .. } => self.read_moov_box_header(input),
            Phase::ReadMoovBox { .. } => self.read_moov_box(input),
            Phase::Initialized => Ok(()),
        }
    }

    fn read_ftyp_box_header(&mut self, input: &Input) -> Result<(), DemuxError> {
        assert!(matches!(self.phase, Phase::ReadFtypBoxHeader));

        if input.position != 0 || input.data.len() < BoxHeader::MAX_SIZE {
            return Err(DemuxError::need_input(
                input.position,
                Some(BoxHeader::MAX_SIZE),
            ));
        }

        let (header, _header_size) = BoxHeader::decode(input.data)?;
        header.box_type.expect(FtypBox::TYPE)?;

        let box_size = Some(header.box_size.get() as usize).filter(|n| *n > 0);
        self.phase = Phase::ReadFtypBox { box_size };
        self.handle_input(input)
    }

    fn read_ftyp_box(&mut self, input: &Input) -> Result<(), DemuxError> {
        let Phase::ReadFtypBox { box_size } = self.phase else {
            panic!("bug");
        };
        if input.position != 0 || box_size.is_some_and(|n| input.data.len() < n) {
            return Err(DemuxError::need_input(input.position, box_size));
        }

        let (_ftyp_box, ftyp_box_size) = FtypBox::decode(input.data)?;
        self.phase = Phase::ReadMoovBoxHeader {
            offset: ftyp_box_size as u64,
        };
        self.handle_input(input)
    }

    fn read_moov_box_header(&mut self, input: &Input) -> Result<(), DemuxError> {
        let Phase::ReadMoovBoxHeader { offset } = self.phase else {
            panic!("bug");
        };

        if input.position != offset || input.data.len() < BoxHeader::MAX_SIZE {
            return Err(DemuxError::need_input(offset, Some(BoxHeader::MAX_SIZE)));
        }

        let (header, _header_size) = BoxHeader::decode(input.data)?;
        let box_size = Some(header.box_size.get()).filter(|n| *n > 0);

        if header.box_type != MoovBox::TYPE {
            let Some(box_size) = box_size else {
                return Err(DemuxError::DecodeError(Error::invalid_data(
                    "moov box not found",
                )));
            };
            let offset = offset + box_size;
            self.phase = Phase::ReadMoovBoxHeader { offset };
            return Err(DemuxError::need_input(offset, Some(BoxHeader::MAX_SIZE)));
        }

        let box_size = box_size.map(|n| n as usize);
        self.phase = Phase::ReadMoovBox { offset, box_size };
        self.handle_input(input)
    }

    fn read_moov_box(&mut self, input: &Input) -> Result<(), DemuxError> {
        let Phase::ReadMoovBox { offset, box_size } = self.phase else {
            panic!("bug");
        };

        if input.position != offset || box_size.is_some_and(|n| input.data.len() < n) {
            return Err(DemuxError::need_input(offset, box_size));
        }

        let (moov_box, _moov_box_size) = MoovBox::decode(input.data)?;

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
            self.tracks.push(TrackState { track_id, table })
        }

        self.phase = Phase::Initialized;
        Ok(())
    }

    pub fn tracks(&mut self) -> Result<&[TrackInfo], DemuxError> {
        self.initialize_if_need()?;
        Ok(&self.track_infos)
    }

    fn next_sample(&mut self) -> Result<Option<Sample>, DemuxError> {
        self.initialize_if_need()?;
        todo!()
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

impl Iterator for Mp4FileDemuxer {
    type Item = Result<Sample, DemuxError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_sample().transpose()
    }
}
