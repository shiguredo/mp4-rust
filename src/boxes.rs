use std::io::{Read, Write};

use crate::{
    io::ExternalBytes, BaseBox, BoxHeader, BoxPath, BoxSize, BoxType, Decode, Encode, Error,
    FixedPointNumber, FullBox, FullBoxFlags, FullBoxHeader, IterUnknownBoxes, Mp4FileTime, Result,
    UnknownBox,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Brand([u8; 4]);

impl Brand {
    // TODO: Add constants for the predefined brands

    pub const fn new(brand: [u8; 4]) -> Self {
        Self(brand)
    }

    pub const fn get(self) -> [u8; 4] {
        self.0
    }
}

impl std::fmt::Debug for Brand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(s) = std::str::from_utf8(&self.0) {
            f.debug_tuple("Brand").field(&s).finish()
        } else {
            f.debug_tuple("Brand").field(&self.0).finish()
        }
    }
}

impl Encode for Brand {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.0)?;
        Ok(())
    }
}

impl Decode for Brand {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

/// [ISO/IEC 14496-12] FileTypeBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtypBox {
    pub major_brand: Brand,
    pub minor_version: u32,
    pub compatible_brands: Vec<Brand>,
}

impl FtypBox {
    pub const TYPE: BoxType = BoxType::Normal([b'f', b't', b'y', b'p']);

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.major_brand.encode(writer)?;
        self.minor_version.encode(writer)?;
        for brand in &self.compatible_brands {
            brand.encode(writer)?;
        }
        Ok(())
    }
}

impl Encode for FtypBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for FtypBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;

        header.with_box_payload_reader(reader, |reader| {
            let major_brand = Brand::decode(reader)?;
            let minor_version = u32::decode(reader)?;
            let mut compatible_brands = Vec::new();
            while reader.limit() > 0 {
                compatible_brands.push(Brand::decode(reader)?);
            }
            Ok(Self {
                major_brand,
                minor_version,
                compatible_brands,
            })
        })
    }
}

impl BaseBox for FtypBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }
}

impl IterUnknownBoxes for FtypBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        std::iter::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootBox {
    Free(FreeBox),
    Mdat(MdatBox),
    Moov(MoovBox),
    Unknown(UnknownBox),
}

impl Encode for RootBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            RootBox::Free(b) => b.encode(writer),
            RootBox::Mdat(b) => b.encode(writer),
            RootBox::Moov(b) => b.encode(writer),
            RootBox::Unknown(b) => b.encode(writer),
        }
    }
}

impl Decode for RootBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let (header, mut reader) = BoxHeader::peek(reader)?;
        match header.box_type {
            FreeBox::TYPE => Decode::decode(&mut reader).map(Self::Free),
            MdatBox::TYPE => Decode::decode(&mut reader).map(Self::Mdat),
            MoovBox::TYPE => Decode::decode(&mut reader).map(Self::Moov),
            _ => Decode::decode(&mut reader).map(Self::Unknown),
        }
    }
}

impl BaseBox for RootBox {
    fn box_type(&self) -> BoxType {
        match self {
            RootBox::Free(b) => b.box_type(),
            RootBox::Mdat(b) => b.box_type(),
            RootBox::Moov(b) => b.box_type(),
            RootBox::Unknown(b) => b.box_type(),
        }
    }

    fn box_payload_size(&self) -> u64 {
        match self {
            RootBox::Free(b) => b.box_payload_size(),
            RootBox::Mdat(b) => b.box_payload_size(),
            RootBox::Moov(b) => b.box_payload_size(),
            RootBox::Unknown(b) => b.box_payload_size(),
        }
    }
}

impl IterUnknownBoxes for RootBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        match self {
            RootBox::Free(b) => {
                Box::new(b.iter_unknown_boxes()) as Box<dyn '_ + Iterator<Item = _>>
            }
            RootBox::Mdat(b) => Box::new(b.iter_unknown_boxes()),
            RootBox::Moov(b) => Box::new(b.iter_unknown_boxes()),
            RootBox::Unknown(b) => Box::new(b.iter_unknown_boxes()),
        }
    }
}

/// [ISO/IEC 14496-12] FreeSpaceBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeBox {
    pub payload: Vec<u8>,
}

impl FreeBox {
    pub const TYPE: BoxType = BoxType::Normal([b'f', b'r', b'e', b'e']);
}

impl Encode for FreeBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for FreeBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;

        let mut payload = Vec::new();
        header.with_box_payload_reader(reader, |reader| Ok(reader.read_to_end(&mut payload)?))?;
        Ok(Self { payload })
    }
}

impl BaseBox for FreeBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        self.payload.len() as u64
    }
}

impl IterUnknownBoxes for FreeBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        std::iter::empty()
    }
}

/// [ISO/IEC 14496-12] MediaDataBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdatBox {
    pub is_variable_size: bool,
    pub payload: Vec<u8>,
}

impl MdatBox {
    pub const TYPE: BoxType = BoxType::Normal([b'm', b'd', b'a', b't']);
}

impl Encode for MdatBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for MdatBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;

        let mut payload = Vec::new();
        header.with_box_payload_reader(reader, |reader| Ok(reader.read_to_end(&mut payload)?))?;
        Ok(Self {
            is_variable_size: header.box_size == BoxSize::VARIABLE_SIZE,
            payload,
        })
    }
}

impl BaseBox for MdatBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_size(&self) -> BoxSize {
        if self.is_variable_size {
            BoxSize::VARIABLE_SIZE
        } else {
            BoxSize::with_payload_size(Self::TYPE, self.box_payload_size())
        }
    }

    fn box_payload_size(&self) -> u64 {
        self.payload.len() as u64
    }
}

impl IterUnknownBoxes for MdatBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        std::iter::empty()
    }
}

/// [ISO/IEC 14496-12] MovieBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoovBox {
    pub mvhd_box: MvhdBox,
    pub trak_boxes: Vec<TrakBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MoovBox {
    pub const TYPE: BoxType = BoxType::Normal([b'm', b'o', b'o', b'v']);

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.mvhd_box.encode(writer)?;
        for b in &self.trak_boxes {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut mvhd_box = None;
        let mut trak_boxes = Vec::new();
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                MvhdBox::TYPE => mvhd_box = Some(Decode::decode(&mut reader)?),
                TrakBox::TYPE => trak_boxes.push(Decode::decode(&mut reader)?),
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }

        let mvhd_box = mvhd_box
            .ok_or_else(|| Error::invalid_data("Missing mandary 'mvhd' box in 'moov' box"))?;
        Ok(Self {
            mvhd_box,
            trak_boxes,
            unknown_boxes,
        })
    }
}

impl Encode for MoovBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MoovBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for MoovBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }
}

impl IterUnknownBoxes for MoovBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        self.trak_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes())
            .chain(
                self.unknown_boxes
                    .iter()
                    .flat_map(|b| b.iter_unknown_boxes())
                    .map(|(path, b)| (path.join(Self::TYPE), b)),
            )
    }
}

/// [ISO/IEC 14496-12] MovieHeaderBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvhdBox {
    pub creation_time: Mp4FileTime,
    pub modification_time: Mp4FileTime,
    pub timescale: u32,
    pub duration: u64,
    pub rate: FixedPointNumber<i16, u16>,
    pub volume: FixedPointNumber<i8, u8>,
    pub matrix: [i32; 9],
    pub next_track_id: u32,
}

impl MvhdBox {
    pub const TYPE: BoxType = BoxType::Normal([b'm', b'v', b'h', b'd']);

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        if self.full_box_version() == 1 {
            self.creation_time.as_secs().encode(writer)?;
            self.modification_time.as_secs().encode(writer)?;
            self.timescale.encode(writer)?;
            self.duration.encode(writer)?;
        } else {
            (self.creation_time.as_secs() as u32).encode(writer)?;
            (self.modification_time.as_secs() as u32).encode(writer)?;
            self.timescale.encode(writer)?;
            (self.duration as u32).encode(writer)?;
        }
        self.rate.encode(writer)?;
        self.volume.encode(writer)?;
        [0; 2 + 4 * 2].encode(writer)?;
        self.matrix.encode(writer)?;
        [0; 4 * 6].encode(writer)?;
        self.next_track_id.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(reader)?;
        let mut this = Self::default();

        if full_header.version == 1 {
            this.creation_time = u64::decode(reader).map(Mp4FileTime::from_secs)?;
            this.modification_time = u64::decode(reader).map(Mp4FileTime::from_secs)?;
            this.timescale = u32::decode(reader)?;
            this.duration = u64::decode(reader)?;
        } else {
            this.creation_time = u32::decode(reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode(reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.timescale = u32::decode(reader)?;
            this.duration = u32::decode(reader)? as u64;
        }

        this.rate = FixedPointNumber::decode(reader)?;
        this.volume = FixedPointNumber::decode(reader)?;
        let _ = <[u8; 2 + 4 * 2]>::decode(reader)?;
        this.matrix = <[i32; 9]>::decode(reader)?;
        let _ = <[u8; 4 * 6]>::decode(reader)?;
        this.next_track_id = u32::decode(reader)?;

        Ok(this)
    }
}

impl Encode for MvhdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MvhdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Default for MvhdBox {
    fn default() -> Self {
        Self {
            creation_time: Mp4FileTime::from_secs(0),
            modification_time: Mp4FileTime::from_secs(0),
            timescale: 0,
            duration: 0,
            rate: FixedPointNumber::new(1, 0),   // 通常の再生速度
            volume: FixedPointNumber::new(1, 0), // 最大音量
            matrix: [0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000],
            next_track_id: 0,
        }
    }
}

impl BaseBox for MvhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }
}

impl FullBox for MvhdBox {
    fn full_box_version(&self) -> u8 {
        if self.creation_time.as_secs() > u32::MAX as u64
            || self.modification_time.as_secs() > u32::MAX as u64
            || self.duration > u32::MAX as u64
        {
            1
        } else {
            0
        }
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

impl IterUnknownBoxes for MvhdBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        std::iter::empty()
    }
}

/// [ISO/IEC 14496-12] TrackBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrakBox {
    pub unknown_boxes: Vec<UnknownBox>,
}

impl TrakBox {
    pub const TYPE: BoxType = BoxType::Normal([b't', b'r', b'a', b'k']);

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }

        Ok(Self { unknown_boxes })
    }
}

impl Encode for TrakBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for TrakBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for TrakBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }
}

impl IterUnknownBoxes for TrakBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        self.unknown_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes())
            .map(|(path, b)| (path.join(Self::TYPE), b))
    }
}
