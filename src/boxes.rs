use std::{
    io::{Read, Write},
    num::NonZeroU32,
};

use crate::{
    io::ExternalBytes, BaseBox, BoxHeader, BoxSize, BoxType, Decode, Either, Encode, Error,
    FixedPointNumber, FullBox, FullBoxFlags, FullBoxHeader, Mp4FileTime, Result, Uint, Utf8String,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownBox {
    pub box_type: BoxType,
    pub box_size: BoxSize,
    pub payload: Vec<u8>,
}

impl Encode for UnknownBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for UnknownBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        let mut payload = Vec::new();
        header.with_box_payload_reader(reader, |reader| Ok(reader.read_to_end(&mut payload)?))?;
        Ok(Self {
            box_type: header.box_type,
            box_size: header.box_size,
            payload,
        })
    }
}

impl BaseBox for UnknownBox {
    fn box_type(&self) -> BoxType {
        self.box_type
    }

    fn box_size(&self) -> BoxSize {
        self.box_size
    }

    fn box_payload_size(&self) -> u64 {
        self.payload.len() as u64
    }

    fn is_opaque_payload(&self) -> bool {
        true
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Brand([u8; 4]);

impl Brand {
    pub const ISOM: Self = Self::new(*b"isom");
    pub const ISO2: Self = Self::new(*b"iso2");
    pub const MP71: Self = Self::new(*b"mp71");
    pub const ISO3: Self = Self::new(*b"iso3");
    pub const ISO4: Self = Self::new(*b"iso4");
    pub const ISO5: Self = Self::new(*b"iso5");
    pub const ISO6: Self = Self::new(*b"iso6");
    pub const ISO7: Self = Self::new(*b"iso7");
    pub const ISO8: Self = Self::new(*b"iso8");
    pub const ISO9: Self = Self::new(*b"iso9");
    pub const ISOA: Self = Self::new(*b"isoa");
    pub const ISOB: Self = Self::new(*b"isob");
    pub const RELO: Self = Self::new(*b"relo");

    pub const MP41: Self = Self::new(*b"mp41");
    pub const AVC1: Self = Self::new(*b"avc1");
    pub const AV01: Self = Self::new(*b"av01");

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
    pub const TYPE: BoxType = BoxType::Normal(*b"ftyp");

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

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
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
    fn actual_box(&self) -> &dyn BaseBox {
        match self {
            RootBox::Free(b) => b.actual_box(),
            RootBox::Mdat(b) => b.actual_box(),
            RootBox::Moov(b) => b.actual_box(),
            RootBox::Unknown(b) => b.actual_box(),
        }
    }

    fn box_type(&self) -> BoxType {
        self.actual_box().box_type()
    }

    fn box_size(&self) -> BoxSize {
        self.actual_box().box_size()
    }

    fn box_payload_size(&self) -> u64 {
        self.actual_box().box_payload_size()
    }

    fn is_opaque_payload(&self) -> bool {
        self.actual_box().is_opaque_payload()
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.actual_box().children()
    }
}

/// [ISO/IEC 14496-12] FreeSpaceBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeBox {
    pub payload: Vec<u8>,
}

impl FreeBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"free");
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

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] MediaDataBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdatBox {
    pub is_variable_size: bool,
    pub payload: Vec<u8>,
}

impl MdatBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"mdat");
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

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] MovieBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoovBox {
    pub mvhd_box: MvhdBox,
    pub trak_boxes: Vec<TrakBox>,
    pub udta_box: Option<UdtaBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MoovBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"moov");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.mvhd_box.encode(writer)?;
        for b in &self.trak_boxes {
            b.encode(writer)?;
        }
        if let Some(b) = &self.udta_box {
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
        let mut udta_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                MvhdBox::TYPE if mvhd_box.is_none() => {
                    mvhd_box = Some(Decode::decode(&mut reader)?);
                }
                TrakBox::TYPE => {
                    trak_boxes.push(Decode::decode(&mut reader)?);
                }
                UdtaBox::TYPE if udta_box.is_none() => {
                    udta_box = Some(Decode::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }

        let mvhd_box = mvhd_box.ok_or_else(|| Error::missing_box("mvhd", Self::TYPE))?;
        Ok(Self {
            mvhd_box,
            trak_boxes,
            udta_box,
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

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::once(self.mvhd_box.actual_box())
                .chain(self.trak_boxes.iter().map(BaseBox::actual_box))
                .chain(self.udta_box.iter().map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
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
    pub const TYPE: BoxType = BoxType::Normal(*b"mvhd");

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
        [0u8; 2 + 4 * 2].encode(writer)?;
        self.matrix.encode(writer)?;
        [0u8; 4 * 6].encode(writer)?;
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
            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
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

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
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

/// [ISO/IEC 14496-12] TrackBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrakBox {
    pub tkhd_box: TkhdBox,
    pub edts_box: Option<EdtsBox>,
    pub mdia_box: MdiaBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl TrakBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"trak");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.tkhd_box.encode(writer)?;
        if let Some(b) = &self.edts_box {
            b.encode(writer)?;
        }
        self.mdia_box.encode(writer)?;
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut tkhd_box = None;
        let mut edts_box = None;
        let mut mdia_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                TkhdBox::TYPE if tkhd_box.is_none() => {
                    tkhd_box = Some(TkhdBox::decode(&mut reader)?)
                }
                EdtsBox::TYPE if edts_box.is_none() => {
                    edts_box = Some(EdtsBox::decode(&mut reader)?)
                }
                MdiaBox::TYPE if mdia_box.is_none() => {
                    mdia_box = Some(MdiaBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }

        let tkhd_box = tkhd_box.ok_or_else(|| Error::missing_box("tkhd", Self::TYPE))?;
        let mdia_box = mdia_box.ok_or_else(|| Error::missing_box("mdia", Self::TYPE))?;
        Ok(Self {
            tkhd_box,
            edts_box,
            mdia_box,
            unknown_boxes,
        })
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

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.tkhd_box).map(BaseBox::actual_box))
                .chain(self.edts_box.iter().map(BaseBox::actual_box))
                .chain(std::iter::once(&self.mdia_box).map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

/// [ISO/IEC 14496-12] TrackHeaderBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TkhdBox {
    pub flag_track_enabled: bool,
    pub flag_track_in_movie: bool,
    pub flag_track_in_preview: bool,
    pub flag_track_size_is_aspect_ratio: bool,

    pub creation_time: Mp4FileTime,
    pub modification_time: Mp4FileTime,
    pub track_id: u32,
    pub duration: u64,
    pub layer: i16,
    pub alternate_group: i16,
    pub volume: FixedPointNumber<i8, u8>,
    pub matrix: [i32; 9],
    pub width: FixedPointNumber<i16, u16>,
    pub height: FixedPointNumber<i16, u16>,
}

impl TkhdBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"tkhd");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        if self.full_box_version() == 1 {
            self.creation_time.as_secs().encode(writer)?;
            self.modification_time.as_secs().encode(writer)?;
            self.track_id.encode(writer)?;
            [0u8; 4].encode(writer)?;
            self.duration.encode(writer)?;
        } else {
            (self.creation_time.as_secs() as u32).encode(writer)?;
            (self.modification_time.as_secs() as u32).encode(writer)?;
            self.track_id.encode(writer)?;
            [0u8; 4].encode(writer)?;
            (self.duration as u32).encode(writer)?;
        }
        [0u8; 4 * 2].encode(writer)?;
        self.layer.encode(writer)?;
        self.alternate_group.encode(writer)?;
        self.volume.encode(writer)?;
        [0u8; 2].encode(writer)?;
        self.matrix.encode(writer)?;
        self.width.encode(writer)?;
        self.height.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(reader)?;
        let mut this = Self::default();

        this.flag_track_enabled = full_header.flags.is_set(0);
        this.flag_track_in_movie = full_header.flags.is_set(1);
        this.flag_track_in_preview = full_header.flags.is_set(2);
        this.flag_track_size_is_aspect_ratio = full_header.flags.is_set(3);

        if full_header.version == 1 {
            this.creation_time = u64::decode(reader).map(Mp4FileTime::from_secs)?;
            this.modification_time = u64::decode(reader).map(Mp4FileTime::from_secs)?;
            this.track_id = u32::decode(reader)?;
            let _ = <[u8; 4]>::decode(reader)?;
            this.duration = u64::decode(reader)?;
        } else {
            this.creation_time = u32::decode(reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode(reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.track_id = u32::decode(reader)?;
            let _ = <[u8; 4]>::decode(reader)?;
            this.duration = u32::decode(reader)? as u64;
        }

        let _ = <[u8; 4 * 2]>::decode(reader)?;
        this.layer = i16::decode(reader)?;
        this.alternate_group = i16::decode(reader)?;
        this.volume = FixedPointNumber::decode(reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        this.matrix = <[i32; 9]>::decode(reader)?;
        this.width = FixedPointNumber::decode(reader)?;
        this.height = FixedPointNumber::decode(reader)?;

        Ok(this)
    }
}

impl Encode for TkhdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for TkhdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Default for TkhdBox {
    fn default() -> Self {
        Self {
            flag_track_enabled: true,
            flag_track_in_movie: true,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,

            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
            track_id: 0,
            duration: 0,
            layer: 0,
            alternate_group: 0,
            volume: FixedPointNumber::new(0, 0),
            matrix: [0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000],
            width: FixedPointNumber::new(0, 0),
            height: FixedPointNumber::new(0, 0),
        }
    }
}

impl BaseBox for TkhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for TkhdBox {
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
        FullBoxFlags::from_iter([
            (0, self.flag_track_enabled),
            (1, self.flag_track_in_movie),
            (2, self.flag_track_in_preview),
            (3, self.flag_track_size_is_aspect_ratio),
        ])
    }
}

/// [ISO/IEC 14496-12] EditBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtsBox {
    pub elst_box: Option<ElstBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl EdtsBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"edts");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        if let Some(b) = &self.elst_box {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut elst_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                ElstBox::TYPE if elst_box.is_none() => {
                    elst_box = Some(ElstBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        Ok(Self {
            elst_box,
            unknown_boxes,
        })
    }
}

impl Encode for EdtsBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for EdtsBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for EdtsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(self.elst_box.iter().map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElstEntry {
    pub edit_duration: u64,
    pub media_time: i64,
    pub media_rate: FixedPointNumber<i16, i16>,
}

/// [ISO/IEC 14496-12] EditListBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElstBox {
    pub entries: Vec<ElstEntry>,
}

impl ElstBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"elst");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;

        let version = self.full_box_version();
        (self.entries.len() as u32).encode(writer)?;
        for entry in &self.entries {
            if version == 1 {
                entry.edit_duration.encode(writer)?;
                entry.media_time.encode(writer)?;
            } else {
                (entry.edit_duration as u32).encode(writer)?;
                (entry.media_time as i32).encode(writer)?;
            }
            entry.media_rate.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(reader)?;

        let mut entries = Vec::new();
        let count = u32::decode(reader)? as usize;
        for _ in 0..count {
            let edit_duration;
            let media_time;
            if full_header.version == 1 {
                edit_duration = u64::decode(reader)?;
                media_time = i64::decode(reader)?;
            } else {
                edit_duration = u32::decode(reader)? as u64;
                media_time = i32::decode(reader)? as i64;
            }
            let media_rate = FixedPointNumber::decode(reader)?;
            entries.push(ElstEntry {
                edit_duration,
                media_time,
                media_rate,
            });
        }

        Ok(Self { entries })
    }
}

impl Encode for ElstBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for ElstBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for ElstBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for ElstBox {
    fn full_box_version(&self) -> u8 {
        let large = self.entries.iter().any(|x| {
            u32::try_from(x.edit_duration).is_err() || i32::try_from(x.media_time).is_err()
        });
        if large {
            1
        } else {
            0
        }
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] MediaBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdiaBox {
    pub mdhd_box: MdhdBox,
    pub hdlr_box: HdlrBox,
    pub minf_box: MinfBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MdiaBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"mdia");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.mdhd_box.encode(writer)?;
        self.hdlr_box.encode(writer)?;
        self.minf_box.encode(writer)?;
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut mdhd_box = None;
        let mut hdlr_box = None;
        let mut minf_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                MdhdBox::TYPE if mdhd_box.is_none() => {
                    mdhd_box = Some(MdhdBox::decode(&mut reader)?);
                }
                HdlrBox::TYPE if hdlr_box.is_none() => {
                    hdlr_box = Some(HdlrBox::decode(&mut reader)?);
                }
                MinfBox::TYPE if minf_box.is_none() => {
                    minf_box = Some(MinfBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let mdhd_box = mdhd_box.ok_or_else(|| Error::missing_box("mdhd", Self::TYPE))?;
        let hdlr_box = hdlr_box.ok_or_else(|| Error::missing_box("hdlr", Self::TYPE))?;
        let minf_box = minf_box.ok_or_else(|| Error::missing_box("minf", Self::TYPE))?;
        Ok(Self {
            mdhd_box,
            hdlr_box,
            minf_box,
            unknown_boxes,
        })
    }
}

impl Encode for MdiaBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MdiaBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for MdiaBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.mdhd_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.hdlr_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.minf_box).map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

/// [ISO/IEC 14496-12] MediaHeaderBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdhdBox {
    pub creation_time: Mp4FileTime,
    pub modification_time: Mp4FileTime,
    pub timescale: u32,
    pub duration: u64,
    pub language: [u8; 3], // ISO-639-2/T language code
}

impl MdhdBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"mdhd");

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

        let mut language: u16 = 0;
        for l in &self.language {
            language = (language << 5)
                | l.checked_sub(0x60).ok_or_else(|| {
                    Error::invalid_input(&format!("Invalid language code: {:?}", self.language))
                })? as u16;
        }
        language.encode(writer)?;
        [0u8; 2].encode(writer)?;

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

        let language = u16::decode(reader)?;
        this.language = [
            ((language >> 10) & 0b11111) as u8 + 0x60,
            ((language >> 5) & 0b11111) as u8 + 0x60,
            (language & 0b11111) as u8 + 0x60,
        ];

        let _ = <[u8; 2]>::decode(reader)?;

        Ok(this)
    }
}

impl Encode for MdhdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MdhdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Default for MdhdBox {
    fn default() -> Self {
        Self {
            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
            timescale: 0,
            duration: 0,
            language: *b"und", // undefined
        }
    }
}

impl BaseBox for MdhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for MdhdBox {
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

/// [ISO/IEC 14496-12] HandlerBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HdlrBox {
    pub handler_type: [u8; 4],
    pub name: Utf8String,
}

impl HdlrBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"hdlr");

    pub const HANDLER_TYPE_SOUN: [u8; 4] = *b"soun";
    pub const HANDLER_TYPE_VIDE: [u8; 4] = *b"vide";

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        [0u8; 4].encode(writer)?;
        self.handler_type.encode(writer)?;
        [0u8; 4 * 3].encode(writer)?;
        self.name.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _full_header = FullBoxHeader::decode(reader)?;
        let _ = <[u8; 4]>::decode(reader)?;
        let handler_type = <[u8; 4]>::decode(reader)?;
        let _ = <[u8; 4 * 3]>::decode(reader)?;
        let name = Utf8String::decode(reader)?;
        Ok(Self { handler_type, name })
    }
}

impl Encode for HdlrBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for HdlrBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for HdlrBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for HdlrBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] MediaInformationBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinfBox {
    pub smhd_or_vmhd_box: Either<SmhdBox, VmhdBox>,
    pub dinf_box: DinfBox,
    pub stbl_box: StblBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MinfBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"minf");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        match &self.smhd_or_vmhd_box {
            Either::A(b) => b.encode(writer)?,
            Either::B(b) => b.encode(writer)?,
        }
        self.dinf_box.encode(writer)?;
        self.stbl_box.encode(writer)?;
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut smhd_box = None;
        let mut vmhd_box = None;
        let mut dinf_box = None;
        let mut stbl_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                SmhdBox::TYPE if smhd_box.is_none() => {
                    smhd_box = Some(SmhdBox::decode(&mut reader)?);
                }
                VmhdBox::TYPE if vmhd_box.is_none() => {
                    vmhd_box = Some(VmhdBox::decode(&mut reader)?);
                }
                DinfBox::TYPE if dinf_box.is_none() => {
                    dinf_box = Some(DinfBox::decode(&mut reader)?);
                }
                StblBox::TYPE if stbl_box.is_none() => {
                    stbl_box = Some(StblBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let smhd_or_vmhd_box = smhd_box
            .map(Either::A)
            .or(vmhd_box.map(Either::B))
            .ok_or_else(|| Error::missing_box("smhd | vmhd", Self::TYPE))?;
        let dinf_box = dinf_box.ok_or_else(|| Error::missing_box("dinf", Self::TYPE))?;
        let stbl_box = stbl_box.ok_or_else(|| Error::missing_box("stbl", Self::TYPE))?;
        Ok(Self {
            smhd_or_vmhd_box,
            dinf_box,
            stbl_box,
            unknown_boxes,
        })
    }
}

impl Encode for MinfBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MinfBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for MinfBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.smhd_or_vmhd_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.dinf_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.stbl_box).map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

/// [ISO/IEC 14496-12] SoundMediaHeaderBox class
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SmhdBox {
    pub balance: i16,
}

impl SmhdBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"smhd");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        self.balance.encode(writer)?;
        [0u8; 2].encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _full_header = FullBoxHeader::decode(reader)?;
        let balance = i16::decode(reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        Ok(Self { balance })
    }
}

impl Encode for SmhdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for SmhdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for SmhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for SmhdBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] VideoMediaHeaderBox class
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VmhdBox {
    pub graphicsmode: u16,
    pub opcolor: [u16; 3],
}

impl VmhdBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"vmhd");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        self.graphicsmode.encode(writer)?;
        self.opcolor.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _full_header = FullBoxHeader::decode(reader)?;
        let graphicsmode = u16::decode(reader)?;
        let opcolor = <[u16; 3]>::decode(reader)?;
        Ok(Self {
            graphicsmode,
            opcolor,
        })
    }
}

impl Encode for VmhdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for VmhdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for VmhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for VmhdBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(1)
    }
}

/// [ISO/IEC 14496-12] DataInformationBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DinfBox {
    pub dref_box: DrefBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl DinfBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"dinf");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.dref_box.encode(writer)?;
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut dref_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                DrefBox::TYPE if dref_box.is_none() => {
                    dref_box = Some(DrefBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let dref_box = dref_box.ok_or_else(|| Error::missing_box("dref", Self::TYPE))?;
        Ok(Self {
            dref_box,
            unknown_boxes,
        })
    }
}

impl Encode for DinfBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for DinfBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for DinfBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.dref_box).map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

/// [ISO/IEC 14496-12] DataReferenceBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrefBox {
    pub url_box: Option<UrlBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl DrefBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"dref");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        let entry_count = (self.url_box.is_some() as usize + self.unknown_boxes.len()) as u32;
        entry_count.encode(writer)?;
        if let Some(b) = &self.url_box {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let entry_count = u32::decode(reader)?;
        let mut url_box = None;
        let mut unknown_boxes = Vec::new();
        for _ in 0..entry_count {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                UrlBox::TYPE if url_box.is_none() => {
                    url_box = Some(UrlBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        Ok(Self {
            url_box,
            unknown_boxes,
        })
    }
}

impl Default for DrefBox {
    fn default() -> Self {
        Self {
            url_box: Some(UrlBox::default()),
            unknown_boxes: Vec::new(),
        }
    }
}

impl Encode for DrefBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for DrefBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for DrefBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(self.url_box.iter().map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

impl FullBox for DrefBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] DataEntryUrlBox class
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UrlBox {
    pub location: Option<Utf8String>,
}

impl UrlBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"url ");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        if let Some(l) = &self.location {
            l.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(reader)?;
        let location = if full_header.flags.is_set(0) {
            None
        } else {
            Some(Utf8String::decode(reader)?)
        };
        Ok(Self { location })
    }
}

impl Encode for UrlBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for UrlBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for UrlBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for UrlBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(self.location.is_none() as u32)
    }
}

/// [ISO/IEC 14496-12] SampleTableBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StblBox {
    pub stsd_box: StsdBox,
    pub stts_box: SttsBox,
    pub stsc_box: StscBox,
    pub stsz_box: StszBox,
    pub stco_or_co64_box: Either<StcoBox, Co64Box>,
    pub sgpd_box: Option<SgpdBox>,
    pub sbgp_box: Option<SbgpBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl StblBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"stbl");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.stsd_box.encode(writer)?;
        self.stts_box.encode(writer)?;
        self.stsc_box.encode(writer)?;
        self.stsz_box.encode(writer)?;
        match &self.stco_or_co64_box {
            Either::A(b) => b.encode(writer)?,
            Either::B(b) => b.encode(writer)?,
        }
        if let Some(b) = &self.sgpd_box {
            b.encode(writer)?;
        }
        if let Some(b) = &self.sbgp_box {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut stsd_box = None;
        let mut stts_box = None;
        let mut stsc_box = None;
        let mut stsz_box = None;
        let mut stco_box = None;
        let mut co64_box = None;
        let mut sgpd_box = None;
        let mut sbgp_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                StsdBox::TYPE if stsd_box.is_none() => {
                    stsd_box = Some(StsdBox::decode(&mut reader)?);
                }
                SttsBox::TYPE if stts_box.is_none() => {
                    stts_box = Some(SttsBox::decode(&mut reader)?);
                }
                StscBox::TYPE if stsc_box.is_none() => {
                    stsc_box = Some(StscBox::decode(&mut reader)?);
                }
                StszBox::TYPE if stsz_box.is_none() => {
                    stsz_box = Some(StszBox::decode(&mut reader)?);
                }
                StcoBox::TYPE if stco_box.is_none() => {
                    stco_box = Some(StcoBox::decode(&mut reader)?);
                }
                Co64Box::TYPE if co64_box.is_none() => {
                    co64_box = Some(Co64Box::decode(&mut reader)?);
                }
                SgpdBox::TYPE if sgpd_box.is_none() => {
                    sgpd_box = Some(SgpdBox::decode(&mut reader)?);
                }
                SbgpBox::TYPE if sbgp_box.is_none() => {
                    sbgp_box = Some(SbgpBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let stsd_box = stsd_box.ok_or_else(|| Error::missing_box("stsd", Self::TYPE))?;
        let stts_box = stts_box.ok_or_else(|| Error::missing_box("stts", Self::TYPE))?;
        let stsc_box = stsc_box.ok_or_else(|| Error::missing_box("stsc", Self::TYPE))?;
        let stsz_box = stsz_box.ok_or_else(|| Error::missing_box("stsz", Self::TYPE))?;
        let stco_or_co64_box = stco_box
            .map(Either::A)
            .or(co64_box.map(Either::B))
            .ok_or_else(|| Error::missing_box("stco | co64", Self::TYPE))?;
        Ok(Self {
            stsd_box,
            stts_box,
            stsc_box,
            stsz_box,
            stco_or_co64_box,
            sgpd_box,
            sbgp_box,
            unknown_boxes,
        })
    }
}

impl Encode for StblBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StblBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for StblBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.stsd_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.stts_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.stsc_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.stsz_box).map(BaseBox::actual_box))
                .chain(std::iter::once(&self.stco_or_co64_box).map(BaseBox::actual_box))
                .chain(self.sgpd_box.iter().map(BaseBox::actual_box))
                .chain(self.sbgp_box.iter().map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

/// [ISO/IEC 14496-12] SampleDescriptionBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StsdBox {
    pub entries: Vec<SampleEntry>,
}

impl StsdBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"stsd");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        let entry_count = (self.entries.len()) as u32;
        entry_count.encode(writer)?;
        for b in &self.entries {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let entry_count = u32::decode(reader)?;
        let mut entries = Vec::new();
        for _ in 0..entry_count {
            entries.push(SampleEntry::decode(reader)?);
        }
        Ok(Self { entries })
    }
}

impl Encode for StsdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StsdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for StsdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(self.entries.iter().map(BaseBox::actual_box))
    }
}

impl FullBox for StsdBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleEntry {
    Avc1(Avc1Box),
    Opus(OpusBox),
    Unknown(UnknownBox),
}

impl Encode for SampleEntry {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Self::Avc1(b) => b.encode(writer),
            Self::Opus(b) => b.encode(writer),
            Self::Unknown(b) => b.encode(writer),
        }
    }
}

impl Decode for SampleEntry {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let (header, mut reader) = BoxHeader::peek(reader)?;
        match header.box_type {
            Avc1Box::TYPE => Decode::decode(&mut reader).map(Self::Avc1),
            OpusBox::TYPE => Decode::decode(&mut reader).map(Self::Opus),
            _ => Decode::decode(&mut reader).map(Self::Unknown),
        }
    }
}

impl BaseBox for SampleEntry {
    fn actual_box(&self) -> &dyn BaseBox {
        match self {
            Self::Avc1(b) => b.actual_box(),
            Self::Opus(b) => b.actual_box(),
            Self::Unknown(b) => b.actual_box(),
        }
    }

    fn box_type(&self) -> BoxType {
        self.actual_box().box_type()
    }

    fn box_size(&self) -> BoxSize {
        self.actual_box().box_size()
    }

    fn box_payload_size(&self) -> u64 {
        self.actual_box().box_payload_size()
    }

    fn is_opaque_payload(&self) -> bool {
        self.actual_box().is_opaque_payload()
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.actual_box().children()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualSampleEntryFields {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: FixedPointNumber<u16, u16>,
    pub vertresolution: FixedPointNumber<u16, u16>,
    pub frame_count: u16,
    pub compressorname: [u8; 32],
    pub depth: u16,
}

impl Default for VisualSampleEntryFields {
    fn default() -> Self {
        Self {
            data_reference_index: 1,
            width: 0,
            height: 0,
            horizresolution: FixedPointNumber::new(0x48, 0), // 72 dpi
            vertresolution: FixedPointNumber::new(0x48, 0),  // 72 dpi
            frame_count: 1,
            compressorname: [0; 32],
            depth: 0x0018, // images are in colour with no alpha
        }
    }
}

impl Encode for VisualSampleEntryFields {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        [0u8; 6].encode(writer)?;
        self.data_reference_index.encode(writer)?;
        [0u8; 2 + 2 + 4 * 3].encode(writer)?;
        self.width.encode(writer)?;
        self.height.encode(writer)?;
        self.horizresolution.encode(writer)?;
        self.vertresolution.encode(writer)?;
        [0u8; 4].encode(writer)?;
        self.frame_count.encode(writer)?;
        self.compressorname.encode(writer)?;
        self.depth.encode(writer)?;
        (-1i16).encode(writer)?;
        Ok(())
    }
}

impl Decode for VisualSampleEntryFields {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let _ = <[u8; 6]>::decode(reader)?;
        let data_reference_index = u16::decode(reader)?;
        let _ = <[u8; 2 + 2 + 4 * 3]>::decode(reader)?;
        let width = u16::decode(reader)?;
        let height = u16::decode(reader)?;
        let horizresolution = FixedPointNumber::decode(reader)?;
        let vertresolution = FixedPointNumber::decode(reader)?;
        let _ = <[u8; 4]>::decode(reader)?;
        let frame_count = u16::decode(reader)?;
        let compressorname = <[u8; 32]>::decode(reader)?;
        let depth = u16::decode(reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        Ok(Self {
            data_reference_index,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            compressorname,
            depth,
        })
    }
}

/// [ISO/IEC 14496-15] AVCSampleEntry class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avc1Box {
    pub visual: VisualSampleEntryFields,
    pub avcc_box: AvccBox,
    pub pasp_box: Option<PaspBox>,
    pub btrt_box: Option<BtrtBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Avc1Box {
    pub const TYPE: BoxType = BoxType::Normal(*b"avc1");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.visual.encode(writer)?;
        self.avcc_box.encode(writer)?;
        if let Some(b) = &self.pasp_box {
            b.encode(writer)?;
        }
        if let Some(b) = &self.btrt_box {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let visual = VisualSampleEntryFields::decode(reader)?;
        let mut avcc_box = None;
        let mut pasp_box = None;
        let mut btrt_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                AvccBox::TYPE if avcc_box.is_none() => {
                    avcc_box = Some(AvccBox::decode(&mut reader)?);
                }
                PaspBox::TYPE if pasp_box.is_none() => {
                    pasp_box = Some(PaspBox::decode(&mut reader)?);
                }
                BtrtBox::TYPE if btrt_box.is_none() => {
                    btrt_box = Some(BtrtBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let avcc_box = avcc_box.ok_or_else(|| Error::missing_box("avcc", Self::TYPE))?;
        Ok(Self {
            visual,
            avcc_box,
            pasp_box,
            btrt_box,
            unknown_boxes,
        })
    }
}

impl Encode for Avc1Box {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Avc1Box {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Avc1Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.avcc_box).map(BaseBox::actual_box))
                .chain(self.pasp_box.iter().map(BaseBox::actual_box))
                .chain(self.btrt_box.iter().map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

/// [ISO/IEC 14496-15] AVCSampleEntry class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvccBox {
    pub configuration_version: u8,
    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: Uint<2>,
    pub sps_list: Vec<Vec<u8>>,
    pub pps_list: Vec<Vec<u8>>,
    pub chroma_format: Option<Uint<2>>,
    pub bit_depth_luma_minus8: Option<Uint<3>>,
    pub bit_depth_chroma_minus8: Option<Uint<3>>,
    pub sps_ext_list: Vec<Vec<u8>>,
}

impl AvccBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"avcC");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.configuration_version.encode(writer)?;
        self.avc_profile_indication.encode(writer)?;
        self.profile_compatibility.encode(writer)?;
        self.avc_level_indication.encode(writer)?;
        (0b111111_00 | self.length_size_minus_one.get()).encode(writer)?;

        let sps_count = u8::try_from(self.sps_list.len())
            .ok()
            .and_then(|n| Uint::<5>::checked_new(n))
            .ok_or_else(|| Error::invalid_input("Too many SPSs"))?;
        (0b111_00000 | sps_count.get()).encode(writer)?;
        for sps in &self.sps_list {
            let size = u16::try_from(sps.len())
                .map_err(|e| Error::invalid_input(&format!("Too long SPS: {e}")))?;
            size.encode(writer)?;
            writer.write_all(&sps)?;
        }

        let pps_count =
            u8::try_from(self.pps_list.len()).map_err(|_| Error::invalid_input("Too many PPSs"))?;
        pps_count.encode(writer)?;
        for pps in &self.pps_list {
            let size = u16::try_from(pps.len())
                .map_err(|e| Error::invalid_input(&format!("Too long PPS: {e}")))?;
            size.encode(writer)?;
            writer.write_all(&pps)?;
        }

        if !matches!(self.avc_profile_indication, 66 | 77 | 88) {
            let chroma_format = self.chroma_format.ok_or_else(|| {
                Error::invalid_input("Missing 'chroma_format' field in 'avcC' boc")
            })?;
            let bit_depth_luma_minus8 = self.bit_depth_luma_minus8.ok_or_else(|| {
                Error::invalid_input("Missing 'bit_depth_luma_minus8' field in 'avcC' boc")
            })?;
            let bit_depth_chroma_minus8 = self.bit_depth_chroma_minus8.ok_or_else(|| {
                Error::invalid_input("Missing 'bit_depth_chroma_minus8' field in 'avcC' boc")
            })?;
            (0b111111_00 | chroma_format.get()).encode(writer)?;
            (0b11111_000 | bit_depth_luma_minus8.get()).encode(writer)?;
            (0b11111_000 | bit_depth_chroma_minus8.get()).encode(writer)?;

            let sps_ext_count = u8::try_from(self.sps_ext_list.len())
                .map_err(|_| Error::invalid_input("Too many SPS EXTs"))?;
            sps_ext_count.encode(writer)?;
            for sps_ext in &self.sps_ext_list {
                let size = u16::try_from(sps_ext.len())
                    .map_err(|e| Error::invalid_input(&format!("Too long SPS EXT: {e}")))?;
                size.encode(writer)?;
                writer.write_all(&sps_ext)?;
            }
        }

        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let configuration_version = u8::decode(reader)?;
        let avc_profile_indication = u8::decode(reader)?;
        let profile_compatibility = u8::decode(reader)?;
        let avc_level_indication = u8::decode(reader)?;
        let length_size_minus_one = Uint::new(u8::decode(reader)?);

        let sps_count = Uint::<5>::new(u8::decode(reader)?).get() as usize;
        let mut sps_list = Vec::with_capacity(sps_count);
        for _ in 0..sps_count {
            let size = u16::decode(reader)? as usize;
            let mut sps = vec![0; size];
            reader.read_exact(&mut sps)?;
            sps_list.push(sps);
        }

        let pps_count = u8::decode(reader)? as usize;
        let mut pps_list = Vec::with_capacity(pps_count);
        for _ in 0..pps_count {
            let size = u16::decode(reader)? as usize;
            let mut pps = vec![0; size];
            reader.read_exact(&mut pps)?;
            pps_list.push(pps);
        }

        let mut chroma_format = None;
        let mut bit_depth_luma_minus8 = None;
        let mut bit_depth_chroma_minus8 = None;
        let mut sps_ext_list = Vec::new();
        if !matches!(avc_profile_indication, 66 | 77 | 88) {
            chroma_format = Some(Uint::new(u8::decode(reader)?));
            bit_depth_luma_minus8 = Some(Uint::new(u8::decode(reader)?));
            bit_depth_chroma_minus8 = Some(Uint::new(u8::decode(reader)?));

            let sps_ext_count = u8::decode(reader)? as usize;
            for _ in 0..sps_ext_count {
                let size = u16::decode(reader)? as usize;
                let mut pps = vec![0; size];
                reader.read_exact(&mut pps)?;
                sps_ext_list.push(pps);
            }
        }

        Ok(Self {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            sps_list,
            pps_list,
            chroma_format,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            sps_ext_list,
        })
    }
}

impl Default for AvccBox {
    fn default() -> Self {
        Self {
            configuration_version: 1,
            avc_profile_indication: 0,
            profile_compatibility: 0,
            avc_level_indication: 0,
            length_size_minus_one: Uint::new(0),
            sps_list: Vec::new(),
            pps_list: Vec::new(),
            chroma_format: None,
            bit_depth_luma_minus8: None,
            bit_depth_chroma_minus8: None,
            sps_ext_list: Vec::new(),
        }
    }
}

impl Encode for AvccBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for AvccBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for AvccBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] PixelAspectRatioBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaspBox {
    pub h_spacing: u32,
    pub v_spacing: u32,
}

impl PaspBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"pasp");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.h_spacing.encode(writer)?;
        self.v_spacing.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        Ok(Self {
            h_spacing: u32::decode(reader)?,
            v_spacing: u32::decode(reader)?,
        })
    }
}

impl Encode for PaspBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for PaspBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for PaspBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] BitRateBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtrtBox {
    pub buffer_size_db: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,
}

impl BtrtBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"btrt");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.buffer_size_db.encode(writer)?;
        self.max_bitrate.encode(writer)?;
        self.avg_bitrate.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        Ok(Self {
            buffer_size_db: u32::decode(reader)?,
            max_bitrate: u32::decode(reader)?,
            avg_bitrate: u32::decode(reader)?,
        })
    }
}

impl Encode for BtrtBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for BtrtBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for BtrtBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SttsEntry {
    pub sample_count: u32,
    pub sample_delta: u32,
}

/// [ISO/IEC 14496-12] TimeToSampleBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SttsBox {
    pub entries: Vec<SttsEntry>,
}

impl SttsBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"stts");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        (self.entries.len() as u32).encode(writer)?;
        for entry in &self.entries {
            entry.sample_count.encode(writer)?;
            entry.sample_delta.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let count = u32::decode(reader)? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(SttsEntry {
                sample_count: u32::decode(reader)?,
                sample_delta: u32::decode(reader)?,
            });
        }
        Ok(Self { entries })
    }
}

impl Encode for SttsBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for SttsBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for SttsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for SttsBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StscEntry {
    pub first_chunk: u32,
    pub sample_per_chunk: u32,
    pub sample_description_index: u32,
}

/// [ISO/IEC 14496-12] SampleToChunkBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StscBox {
    pub entries: Vec<StscEntry>,
}

impl StscBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"stsc");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        (self.entries.len() as u32).encode(writer)?;
        for entry in &self.entries {
            entry.first_chunk.encode(writer)?;
            entry.sample_per_chunk.encode(writer)?;
            entry.sample_description_index.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let count = u32::decode(reader)? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(StscEntry {
                first_chunk: u32::decode(reader)?,
                sample_per_chunk: u32::decode(reader)?,
                sample_description_index: u32::decode(reader)?,
            });
        }
        Ok(Self { entries })
    }
}

impl Encode for StscBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StscBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for StscBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for StscBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] SampleSizeBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StszBox {
    Fixed {
        sample_size: NonZeroU32,
        sample_count: u32,
    },
    Variable {
        entry_sizes: Vec<u32>,
    },
}

impl StszBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"stsz");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        match self {
            StszBox::Fixed {
                sample_size,
                sample_count,
            } => {
                sample_size.get().encode(writer)?;
                sample_count.encode(writer)?;
            }
            StszBox::Variable { entry_sizes } => {
                0u32.encode(writer)?;
                (entry_sizes.len() as u32).encode(writer)?;
                for size in entry_sizes {
                    size.encode(writer)?;
                }
            }
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let sample_size = u32::decode(reader)?;
        let sample_count = u32::decode(reader)?;
        if let Some(sample_size) = NonZeroU32::new(sample_size) {
            Ok(Self::Fixed {
                sample_size,
                sample_count,
            })
        } else {
            let mut entry_sizes = Vec::with_capacity(sample_count as usize);
            for _ in 0..sample_count {
                entry_sizes.push(u32::decode(reader)?);
            }
            Ok(Self::Variable { entry_sizes })
        }
    }
}

impl Encode for StszBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StszBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for StszBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for StszBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] ChunkOffsetBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StcoBox {
    pub chunk_offsets: Vec<u32>,
}

impl StcoBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"stco");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        (self.chunk_offsets.len() as u32).encode(writer)?;
        for offset in &self.chunk_offsets {
            offset.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let count = u32::decode(reader)? as usize;
        let mut chunk_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            chunk_offsets.push(u32::decode(reader)?);
        }
        Ok(Self { chunk_offsets })
    }
}

impl Encode for StcoBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StcoBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for StcoBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for StcoBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] ChunkLargeOffsetBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Co64Box {
    pub chunk_offsets: Vec<u64>,
}

impl Co64Box {
    pub const TYPE: BoxType = BoxType::Normal(*b"co64");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(writer)?;
        (self.chunk_offsets.len() as u32).encode(writer)?;
        for offset in &self.chunk_offsets {
            offset.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(reader)?;
        let count = u32::decode(reader)? as usize;
        let mut chunk_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            chunk_offsets.push(u64::decode(reader)?);
        }
        Ok(Self { chunk_offsets })
    }
}

impl Encode for Co64Box {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Co64Box {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Co64Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for Co64Box {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [ISO/IEC 14496-12] SampleGroupDescriptionBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgpdBox {
    // 必要になるまではこのボックスの中身は単なるバイト列として扱う
    pub payload: Vec<u8>,
}

impl SgpdBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"sgpd");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.payload)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;
        Ok(Self { payload })
    }
}

impl Encode for SgpdBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for SgpdBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for SgpdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        true
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] SampleToGroupBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbgpBox {
    // 必要になるまではこのボックスの中身は単なるバイト列として扱う
    pub payload: Vec<u8>,
}

impl SbgpBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"sbgp");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.payload)?;
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;
        Ok(Self { payload })
    }
}

impl Encode for SbgpBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for SbgpBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for SbgpBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        true
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] UserDataBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdtaBox {
    // 必要になるまではこのボックスの中身は単なるバイト列として扱う
    pub payload: Vec<u8>,
}

impl UdtaBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"udta");
}

impl Encode for UdtaBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for UdtaBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;

        let mut payload = Vec::new();
        header.with_box_payload_reader(reader, |reader| Ok(reader.read_to_end(&mut payload)?))?;
        Ok(Self { payload })
    }
}

impl BaseBox for UdtaBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        self.payload.len() as u64
    }

    fn is_opaque_payload(&self) -> bool {
        true
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [<https://gitlab.xiph.org/xiph/opus/-/blob/main/doc/opus_in_isobmff.html>] OpusSampleEntry class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpusBox {
    pub audio: AudioSampleEntryFields,
    pub dops_box: DopsBox,
    pub btrt_box: Option<BtrtBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl OpusBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"Opus");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.audio.encode(writer)?;
        self.dops_box.encode(writer)?;
        if let Some(b) = &self.btrt_box {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let audio = AudioSampleEntryFields::decode(reader)?;
        let mut dops_box = None;
        let mut btrt_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                DopsBox::TYPE if dops_box.is_none() => {
                    dops_box = Some(DopsBox::decode(&mut reader)?);
                }
                BtrtBox::TYPE if btrt_box.is_none() => {
                    btrt_box = Some(BtrtBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let dops_box = dops_box.ok_or_else(|| Error::missing_box("dops", Self::TYPE))?;
        Ok(Self {
            audio,
            dops_box,
            btrt_box,
            unknown_boxes,
        })
    }
}

impl Encode for OpusBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for OpusBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for OpusBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.dops_box).map(BaseBox::actual_box))
                .chain(self.btrt_box.iter().map(BaseBox::actual_box))
                .chain(self.unknown_boxes.iter().map(BaseBox::actual_box)),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSampleEntryFields {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: FixedPointNumber<u16, u16>,
}

impl Default for AudioSampleEntryFields {
    fn default() -> Self {
        Self {
            data_reference_index: 1,
            channelcount: 1,
            samplesize: 16,
            samplerate: FixedPointNumber::new(0, 0),
        }
    }
}

impl Encode for AudioSampleEntryFields {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        [0u8; 6].encode(writer)?;
        self.data_reference_index.encode(writer)?;
        [0u8; 4 * 2].encode(writer)?;
        self.channelcount.encode(writer)?;
        self.samplesize.encode(writer)?;
        [0u8; 2].encode(writer)?;
        [0u8; 2].encode(writer)?;
        self.samplerate.encode(writer)?;
        Ok(())
    }
}

impl Decode for AudioSampleEntryFields {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let _ = <[u8; 6]>::decode(reader)?;
        let data_reference_index = u16::decode(reader)?;
        let _ = <[u8; 4 * 2]>::decode(reader)?;
        let channelcount = u16::decode(reader)?;
        let samplesize = u16::decode(reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        let samplerate = FixedPointNumber::decode(reader)?;
        Ok(Self {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
        })
    }
}

/// [<https://gitlab.xiph.org/xiph/opus/-/blob/main/doc/opus_in_isobmff.html>] OpusSpecificBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DopsBox {
    pub version: u8,
    pub output_channel_count: u8,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
    pub output_gain: i16,
}

impl DopsBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"dOps");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.version.encode(writer)?;
        self.output_channel_count.encode(writer)?;
        self.pre_skip.encode(writer)?;
        self.input_sample_rate.encode(writer)?;
        self.output_gain.encode(writer)?;
        0u8.encode(writer)?; // ChannelMappingFamily
        Ok(())
    }

    fn decode_payload<R: Read>(reader: &mut std::io::Take<R>) -> Result<Self> {
        let version = u8::decode(reader)?;
        let output_channel_count = u8::decode(reader)?;
        let pre_skip = u16::decode(reader)?;
        let input_sample_rate = u32::decode(reader)?;
        let output_gain = i16::decode(reader)?;
        let channel_mapping_family = u8::decode(reader)?;
        if channel_mapping_family != 0 {
            return Err(Error::unsupported(
                "`ChannelMappingFamily != 0` in 'dOps' box is not supported",
            ));
        }
        Ok(Self {
            version,
            output_channel_count,
            pre_skip,
            input_sample_rate,
            output_gain,
        })
    }
}

impl Default for DopsBox {
    fn default() -> Self {
        Self {
            version: 0,
            output_channel_count: 1,
            pre_skip: 0,
            input_sample_rate: 0,
            output_gain: 0,
        }
    }
}

impl Encode for DopsBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        BoxHeader::from_box(self).encode(writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for DopsBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let header = BoxHeader::decode(reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for DopsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn is_opaque_payload(&self) -> bool {
        false
    }

    fn actual_box(&self) -> &dyn BaseBox {
        self
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}
