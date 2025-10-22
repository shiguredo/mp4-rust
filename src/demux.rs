#![allow(missing_docs)]

use core::time::Duration;

use crate::{
    BoxHeader, Decode, Error,
    aux::SampleTableAccessor,
    boxes::{FtypBox, MoovBox, SampleEntry, StblBox},
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

#[derive(Debug)]
pub struct TrackState {
    table: SampleTableAccessor<StblBox>,
}

#[derive(Debug)]
pub enum DemuxError {
    DecodeError(Error),
    NeedInput { position: u64, size: usize },
}

impl DemuxError {
    fn need_input(position: u64, size: usize) -> Self {
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
    ReadFtypBox { box_size: usize },
    ReadMoovBoxHeader { offset: u64 },
    Initialized,
}

#[derive(Debug)]
pub struct Mp4FileDemuxer {
    phase: Phase,
    tracks: Vec<TrackState>,
}

impl Mp4FileDemuxer {
    pub fn new() -> Self {
        Self {
            phase: Phase::ReadFtypBoxHeader,
            tracks: Vec::new(),
        }
    }

    pub fn handle_input(&mut self, input: &Input) -> Result<(), DemuxError> {
        match self.phase {
            Phase::ReadFtypBoxHeader => self.read_ftyp_box_header(input),
            Phase::ReadFtypBox { .. } => self.read_ftyp_box(input),
            Phase::ReadMoovBoxHeader { .. } => self.read_moov_box_header(input),
            Phase::Initialized => Ok(()),
        }
    }

    fn read_ftyp_box_header(&mut self, input: &Input) -> Result<(), DemuxError> {
        assert!(matches!(self.phase, Phase::ReadFtypBoxHeader));

        if input.position != 0 || input.data.len() < BoxHeader::MAX_SIZE {
            return Err(DemuxError::need_input(input.position, BoxHeader::MAX_SIZE));
        }

        let (header, _header_size) = BoxHeader::decode(input.data)?;
        header.box_type.expect(FtypBox::TYPE)?;

        let box_size = header.box_size.get() as usize;
        if box_size == 0 {
            return Err(DemuxError::DecodeError(Error::invalid_data(
                "ftype box must have a fixed size and cannot be variable size",
            )));
        }

        self.phase = Phase::ReadFtypBox { box_size };
        self.handle_input(input)
    }

    fn read_ftyp_box(&mut self, input: &Input) -> Result<(), DemuxError> {
        let Phase::ReadFtypBox { box_size } = self.phase else {
            panic!("bug");
        };
        if input.position != 0 || input.data.len() < box_size {
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
            return Err(DemuxError::need_input(offset, BoxHeader::MAX_SIZE));
        }

        let (header, _header_size) = BoxHeader::decode(input.data)?;
        header.box_type.expect(MoovBox::TYPE)?;

        let box_size = header.box_size.get() as usize;
        if box_size == 0 {
            return Err(DemuxError::DecodeError(Error::invalid_data(
                "moov box must have a fixed size and cannot be variable size",
            )));
        }

        // TODO: Transition to reading the moov box content
        // For now, mark as initialized
        self.phase = Phase::Initialized;
        Ok(())
    }

    pub fn tracks(&self) -> Result<&[TrackInfo], DemuxError> {
        todo!()
    }

    pub fn seek(&mut self, _timestamp: Duration) -> Result<(), DemuxError> {
        todo!()
    }

    /// 指定のタイムスタンプ以下で、一番タイムスタンプが大きいキーフレームにシークする
    pub fn seek_to_keyframe(&mut self, _timestamp: Duration) -> Result<(), DemuxError> {
        todo!()
    }

    fn next_sample(&mut self) -> Result<Option<Sample>, DemuxError> {
        self.initialize_if_need()?;
        todo!()
    }

    fn initialize_if_need(&mut self) -> Result<(), DemuxError> {
        match self.phase {
            Phase::ReadFtypBoxHeader => Err(DemuxError::need_input(0, BoxHeader::MAX_SIZE)),
            Phase::ReadFtypBox { box_size } => Err(DemuxError::need_input(0, box_size)),
            Phase::ReadMoovBoxHeader { offset } => {
                Err(DemuxError::need_input(offset, BoxHeader::MAX_SIZE))
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
