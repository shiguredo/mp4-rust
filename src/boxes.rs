use std::io::{Read, Write};

use crate::{
    io::ExternalBytes, BaseBox, BoxHeader, BoxPath, BoxSize, BoxType, Decode, Encode, Error,
    FixedPointNumber, FullBox, FullBoxFlags, FullBoxHeader, IterUnknownBoxes, Mp4FileTime, Result,
    UnknownBox, Utf8String,
};

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
            RootBox::Free(_) => Box::new(std::iter::empty()) as Box<dyn '_ + Iterator<Item = _>>,
            RootBox::Mdat(_) => Box::new(std::iter::empty()),
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
}

/// [ISO/IEC 14496-12] MovieBox class
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoovBox {
    pub mvhd_box: MvhdBox,
    pub trak_boxes: Vec<TrakBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MoovBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"moov");

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
                MvhdBox::TYPE if mvhd_box.is_none() => {
                    mvhd_box = Some(Decode::decode(&mut reader)?);
                }
                TrakBox::TYPE => {
                    trak_boxes.push(Decode::decode(&mut reader)?);
                }
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
        let iter0 = self.trak_boxes.iter().flat_map(|b| b.iter_unknown_boxes());
        let iter1 = self
            .unknown_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes());
        iter0
            .chain(iter1)
            .map(|(path, b)| (path.join(Self::TYPE), b))
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

        let tkhd_box = tkhd_box
            .ok_or_else(|| Error::invalid_data("Missing mandary 'tkhd' box in 'trak' box"))?;
        let mdia_box = mdia_box
            .ok_or_else(|| Error::invalid_data("Missing mandary 'mdia' box in 'trak' box"))?;
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
}

impl IterUnknownBoxes for TrakBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        let iter0 = self.edts_box.iter().flat_map(|b| b.iter_unknown_boxes());
        let iter1 = self.mdia_box.iter_unknown_boxes();
        let iter2 = self
            .unknown_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes());
        iter0
            .chain(iter1)
            .chain(iter2)
            .map(|(path, b)| (path.join(Self::TYPE), b))
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
            [0; 4].encode(writer)?;
            self.duration.encode(writer)?;
        } else {
            (self.creation_time.as_secs() as u32).encode(writer)?;
            (self.modification_time.as_secs() as u32).encode(writer)?;
            self.track_id.encode(writer)?;
            [0; 4].encode(writer)?;
            (self.duration as u32).encode(writer)?;
        }
        [0; 4 * 2].encode(writer)?;
        self.layer.encode(writer)?;
        self.alternate_group.encode(writer)?;
        self.volume.encode(writer)?;
        [0; 2].encode(writer)?;
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
            flag_track_enabled: false,
            flag_track_in_movie: false,
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
}

impl IterUnknownBoxes for EdtsBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        let iter0 = self
            .unknown_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes());
        iter0.map(|(path, b)| (path.join(Self::TYPE), b))
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
        let mdhd_box = mdhd_box
            .ok_or_else(|| Error::invalid_data("Missing mandary 'mdhd' box in 'trak' box"))?;
        let hdlr_box = hdlr_box
            .ok_or_else(|| Error::invalid_data("Missing mandary 'hdlr' box in 'trak' box"))?;
        let minf_box = minf_box
            .ok_or_else(|| Error::invalid_data("Missing mandary 'minf' box in 'trak' box"))?;
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
}

impl IterUnknownBoxes for MdiaBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        let iter0 = self.minf_box.iter_unknown_boxes();
        let iter1 = self
            .unknown_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes());
        iter0
            .chain(iter1)
            .map(|(path, b)| (path.join(Self::TYPE), b))
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

        let mut language = 0;
        for l in &self.language {
            language = (language << 5)
                | l.checked_sub(0x60).ok_or_else(|| {
                    Error::invalid_input(&format!("Invalid language code: {:?}", self.language))
                })?;
        }
        language.encode(writer)?;
        [0; 2].encode(writer)?;

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
            ((language >> 10) & 0b11111) as u8,
            ((language >> 5) & 0b11111) as u8,
            (language & 0b11111) as u8,
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
        [0; 4].encode(writer)?;
        self.handler_type.encode(writer)?;
        [0; 4 * 3].encode(writer)?;
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
    // smhd か vmhd のどちらかが必須
    pub smhd_box: Option<SmhdBox>,
    // pub vmhd_box:Option<VmhdBox>,
    // pub dinf_box: DinfBox,
    // pub stbl_box:StblBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MinfBox {
    pub const TYPE: BoxType = BoxType::Normal(*b"minf");

    fn encode_payload<W: Write>(&self, writer: &mut W) -> Result<()> {
        if let Some(b) = &self.smhd_box {
            b.encode(writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let mut smhd_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                SmhdBox::TYPE if smhd_box.is_none() => {
                    smhd_box = Some(SmhdBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        // let minf_box = minf_box
        //     .ok_or_else(|| Error::invalid_data("Missing mandary 'minf' box in 'trak' box"))?;
        Ok(Self {
            smhd_box,
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
}

impl IterUnknownBoxes for MinfBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        let iter0 = std::iter::empty();
        let iter1 = std::iter::empty();
        let iter2 = std::iter::empty();
        let iter3 = std::iter::empty();
        let iter4 = self
            .unknown_boxes
            .iter()
            .flat_map(|b| b.iter_unknown_boxes());
        iter0
            .chain(iter1)
            .chain(iter2)
            .chain(iter3)
            .chain(iter4)
            .map(|(path, b)| (path.join(Self::TYPE), b))
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
        [0; 2].encode(writer)?;
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
}

impl FullBox for SmhdBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}
