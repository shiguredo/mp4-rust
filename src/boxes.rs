//! ボックス群
use std::{
    io::{Read, Write},
    num::{NonZeroU16, NonZeroU32},
};

use crate::{
    basic_types::as_box_object, io::ExternalBytes, BaseBox, BoxHeader, BoxSize, BoxType, Decode,
    Either, Encode, Error, FixedPointNumber, FullBox, FullBoxFlags, FullBoxHeader, Mp4FileTime,
    Result, Uint, Utf8String,
};

/// ペイロードの解釈方法が不明なボックスを保持するための構造体
///
/// ペイロードは単なるバイト列として扱われる
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnknownBox {
    /// ボックス種別
    pub box_type: BoxType,

    /// ボックスサイズ
    pub box_size: BoxSize,

    /// ペイロード
    pub payload: Vec<u8>,
}

impl Encode for UnknownBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for UnknownBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn is_unknown_box(&self) -> bool {
        true
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [`UnknownBox`] と似ているが、ボックスのペイロードデータを保持しない点が異なる構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IgnoredBox {
    /// ボックス種別
    pub box_type: BoxType,

    /// ボックスサイズ
    pub box_size: BoxSize,

    /// ペイロードサイズ
    pub box_payload_size: u64,
}

impl IgnoredBox {
    /// 次のボックスがデコード対象ならデコードし、そうではない場合には無視する
    pub fn decode_or_ignore<B, R, F>(reader: R, is_decode_target: F) -> Result<Either<B, Self>>
    where
        B: BaseBox + Decode,
        R: Read,
        F: FnOnce(BoxType) -> bool,
    {
        let (header, mut reader) = BoxHeader::peek(reader)?;
        if is_decode_target(header.box_type) {
            B::decode(&mut reader).map(Either::A)
        } else {
            Self::decode(&mut reader).map(Either::B)
        }
    }
}

impl Decode for IgnoredBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        let box_payload_size = header.with_box_payload_reader(reader, |reader| {
            let mut buf = [0; 1024];
            let mut box_payload_size = 0;
            loop {
                let size = reader.read(&mut buf)?;
                if size == 0 {
                    break;
                }
                box_payload_size += size as u64;
            }
            Ok(box_payload_size)
        })?;
        Ok(Self {
            box_type: header.box_type,
            box_size: header.box_size,
            box_payload_size,
        })
    }
}

impl BaseBox for IgnoredBox {
    fn box_type(&self) -> BoxType {
        self.box_type
    }

    fn box_size(&self) -> BoxSize {
        self.box_size
    }

    fn box_payload_size(&self) -> u64 {
        self.box_payload_size
    }

    fn is_unknown_box(&self) -> bool {
        true
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [`FtypBox`] で使われるブランド定義
///
/// ブランドは、対象の MP4 ファイルを読み込んで処理する際に必要となる要件（登場する可能性があるボックス群やハンドリングすべきフラグなど）を指定する
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Brand([u8; 4]);

impl Brand {
    /// [ISO/IEC 14496-12] `isom` ブランド
    pub const ISOM: Self = Self::new(*b"isom");

    /// [ISO/IEC 14496-12] `avc1` ブランド
    pub const AVC1: Self = Self::new(*b"avc1");

    /// [ISO/IEC 14496-12] `iso2` ブランド
    pub const ISO2: Self = Self::new(*b"iso2");

    /// [ISO/IEC 14496-12] `mp71` ブランド
    pub const MP71: Self = Self::new(*b"mp71");

    /// [ISO/IEC 14496-12] `iso3` ブランド
    pub const ISO3: Self = Self::new(*b"iso3");

    /// [ISO/IEC 14496-12] `iso4` ブランド
    pub const ISO4: Self = Self::new(*b"iso4");

    /// [ISO/IEC 14496-12] `iso5` ブランド
    pub const ISO5: Self = Self::new(*b"iso5");

    /// [ISO/IEC 14496-12] `iso6` ブランド
    pub const ISO6: Self = Self::new(*b"iso6");

    /// [ISO/IEC 14496-12] `iso7` ブランド
    pub const ISO7: Self = Self::new(*b"iso7");

    /// [ISO/IEC 14496-12] `iso8` ブランド
    pub const ISO8: Self = Self::new(*b"iso8");

    /// [ISO/IEC 14496-12] `iso9` ブランド
    pub const ISO9: Self = Self::new(*b"iso9");

    /// [ISO/IEC 14496-12] `isoa` ブランド
    pub const ISOA: Self = Self::new(*b"isoa");

    /// [ISO/IEC 14496-12] `isob` ブランド
    pub const ISOB: Self = Self::new(*b"isob");

    /// [ISO/IEC 14496-12] `relo` ブランド
    pub const RELO: Self = Self::new(*b"relo");

    /// [ISO/IEC 14496-14] `mp41` ブランド
    pub const MP41: Self = Self::new(*b"mp41");

    /// [<https://aomediacodec.github.io/av1-isobmff/>] `av01` ブランド
    pub const AV01: Self = Self::new(*b"av01");

    /// バイト列を渡して、対応するブランドを作成する
    pub const fn new(brand: [u8; 4]) -> Self {
        Self(brand)
    }

    /// このブランドを表すバイト列を取得する
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.0)?;
        Ok(())
    }
}

impl Decode for Brand {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

/// [ISO/IEC 14496-12] FileTypeBox class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct FtypBox {
    pub major_brand: Brand,
    pub minor_version: u32,
    pub compatible_brands: Vec<Brand>,
}

impl FtypBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"ftyp");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.major_brand.encode(&mut writer)?;
        self.minor_version.encode(&mut writer)?;
        for brand in &self.compatible_brands {
            brand.encode(&mut writer)?;
        }
        Ok(())
    }
}

impl Encode for FtypBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for FtypBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;

        header.with_box_payload_reader(reader, |mut reader| {
            let major_brand = Brand::decode(&mut reader)?;
            let minor_version = u32::decode(&mut reader)?;
            let mut compatible_brands = Vec::new();
            while reader.limit() > 0 {
                compatible_brands.push(Brand::decode(&mut reader)?);
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [`Mp4File`](crate::Mp4File) のトップレベルに位置するボックス群のデフォルト実装
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum RootBox {
    Free(FreeBox),
    Mdat(MdatBox),
    Moov(MoovBox),
    Unknown(UnknownBox),
}

impl RootBox {
    fn inner_box(&self) -> &dyn BaseBox {
        match self {
            RootBox::Free(b) => b,
            RootBox::Mdat(b) => b,
            RootBox::Moov(b) => b,
            RootBox::Unknown(b) => b,
        }
    }
}

impl Encode for RootBox {
    fn encode<W: Write>(&self, writer: W) -> Result<()> {
        match self {
            RootBox::Free(b) => b.encode(writer),
            RootBox::Mdat(b) => b.encode(writer),
            RootBox::Moov(b) => b.encode(writer),
            RootBox::Unknown(b) => b.encode(writer),
        }
    }
}

impl Decode for RootBox {
    fn decode<R: Read>(reader: R) -> Result<Self> {
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
        self.inner_box().box_type()
    }

    fn box_size(&self) -> BoxSize {
        self.inner_box().box_size()
    }

    fn box_payload_size(&self) -> u64 {
        self.inner_box().box_payload_size()
    }

    fn is_unknown_box(&self) -> bool {
        self.inner_box().is_unknown_box()
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.inner_box().children()
    }
}

/// [ISO/IEC 14496-12] FreeSpaceBox class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct FreeBox {
    pub payload: Vec<u8>,
}

impl FreeBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"free");
}

impl Encode for FreeBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for FreeBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] MediaDataBox class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MdatBox {
    /// ペイロードが可変長かどうか
    pub is_variable_size: bool,

    /// ペイロード
    pub payload: Vec<u8>,
}

impl MdatBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"mdat");
}

impl Encode for MdatBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for MdatBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-12] MovieBox class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct MoovBox {
    pub mvhd_box: MvhdBox,
    pub trak_boxes: Vec<TrakBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MoovBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"moov");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.mvhd_box.encode(&mut writer)?;
        for b in &self.trak_boxes {
            b.encode(&mut writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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

        let mvhd_box = mvhd_box.ok_or_else(|| Error::missing_box("mvhd", Self::TYPE))?;
        Ok(Self {
            mvhd_box,
            trak_boxes,
            unknown_boxes,
        })
    }
}

impl Encode for MoovBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MoovBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.mvhd_box).map(as_box_object))
                .chain(self.trak_boxes.iter().map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-12] MovieHeaderBox class (親: [`MoovBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct MvhdBox {
    pub creation_time: Mp4FileTime,
    pub modification_time: Mp4FileTime,
    pub timescale: NonZeroU32,
    pub duration: u64,
    pub rate: FixedPointNumber<i16, u16>,
    pub volume: FixedPointNumber<i8, u8>,
    pub matrix: [i32; 9],
    pub next_track_id: u32,
}

impl MvhdBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"mvhd");

    /// [`MvhdBox::rate`] のデフォルト値（通常の再生速度）
    pub const DEFAULT_RATE: FixedPointNumber<i16, u16> = FixedPointNumber::new(1, 0);

    /// [`MvhdBox::volume`] のデフォルト値（最大音量）
    pub const DEFAULT_VOLUME: FixedPointNumber<i8, u8> = FixedPointNumber::new(1, 0);

    /// [`MvhdBox::matrix`] のデフォルト値
    pub const DEFAULT_MATRIX: [i32; 9] = [0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000];

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        if self.full_box_version() == 1 {
            self.creation_time.as_secs().encode(&mut writer)?;
            self.modification_time.as_secs().encode(&mut writer)?;
            self.timescale.encode(&mut writer)?;
            self.duration.encode(&mut writer)?;
        } else {
            (self.creation_time.as_secs() as u32).encode(&mut writer)?;
            (self.modification_time.as_secs() as u32).encode(&mut writer)?;
            self.timescale.encode(&mut writer)?;
            (self.duration as u32).encode(&mut writer)?;
        }
        self.rate.encode(&mut writer)?;
        self.volume.encode(&mut writer)?;
        [0u8; 2 + 4 * 2].encode(&mut writer)?;
        self.matrix.encode(&mut writer)?;
        [0u8; 4 * 6].encode(&mut writer)?;
        self.next_track_id.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(&mut reader)?;
        let mut this = Self {
            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
            timescale: NonZeroU32::MIN,
            duration: 0,
            rate: Self::DEFAULT_RATE,
            volume: Self::DEFAULT_VOLUME,
            matrix: Self::DEFAULT_MATRIX,
            next_track_id: 0,
        };

        if full_header.version == 1 {
            this.creation_time = u64::decode(&mut reader).map(Mp4FileTime::from_secs)?;
            this.modification_time = u64::decode(&mut reader).map(Mp4FileTime::from_secs)?;
            this.timescale = NonZeroU32::decode(&mut reader)?;
            this.duration = u64::decode(&mut reader)?;
        } else {
            this.creation_time =
                u32::decode(&mut reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode(&mut reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.timescale = NonZeroU32::decode(&mut reader)?;
            this.duration = u32::decode(&mut reader)? as u64;
        }

        this.rate = FixedPointNumber::decode(&mut reader)?;
        this.volume = FixedPointNumber::decode(&mut reader)?;
        let _ = <[u8; 2 + 4 * 2]>::decode(&mut reader)?;
        this.matrix = <[i32; 9]>::decode(&mut reader)?;
        let _ = <[u8; 4 * 6]>::decode(&mut reader)?;
        this.next_track_id = u32::decode(reader)?;

        Ok(this)
    }
}

impl Encode for MvhdBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MvhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for MvhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
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

/// [ISO/IEC 14496-12] TrackBox class (親: [`MoovBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct TrakBox {
    pub tkhd_box: TkhdBox,
    pub edts_box: Option<EdtsBox>,
    pub mdia_box: MdiaBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl TrakBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"trak");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.tkhd_box.encode(&mut writer)?;
        if let Some(b) = &self.edts_box {
            b.encode(&mut writer)?;
        }
        self.mdia_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for TrakBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.tkhd_box).map(as_box_object))
                .chain(self.edts_box.iter().map(as_box_object))
                .chain(std::iter::once(&self.mdia_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-12] TrackHeaderBox class (親: [`TrakBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
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
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"tkhd");

    /// [`TkhdBox::layer`] のデフォルト値
    pub const DEFAULT_LAYER: i16 = 0;

    /// [`TkhdBox::alternate_group`] のデフォルト値
    pub const DEFAULT_ALTERNATE_GROUP: i16 = 0;

    /// 音声用の [`TkhdBox::volume`] のデフォルト値（最大音量）
    pub const DEFAULT_AUDIO_VOLUME: FixedPointNumber<i8, u8> = FixedPointNumber::new(1, 0);

    /// 映像用の [`TkhdBox::volume`] のデフォルト値（無音）
    pub const DEFAULT_VIDEO_VOLUME: FixedPointNumber<i8, u8> = FixedPointNumber::new(0, 0);

    /// [`TkhdBox::matrix`] のデフォルト値
    pub const DEFAULT_MATRIX: [i32; 9] = [0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000];

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        if self.full_box_version() == 1 {
            self.creation_time.as_secs().encode(&mut writer)?;
            self.modification_time.as_secs().encode(&mut writer)?;
            self.track_id.encode(&mut writer)?;
            [0u8; 4].encode(&mut writer)?;
            self.duration.encode(&mut writer)?;
        } else {
            (self.creation_time.as_secs() as u32).encode(&mut writer)?;
            (self.modification_time.as_secs() as u32).encode(&mut writer)?;
            self.track_id.encode(&mut writer)?;
            [0u8; 4].encode(&mut writer)?;
            (self.duration as u32).encode(&mut writer)?;
        }
        [0u8; 4 * 2].encode(&mut writer)?;
        self.layer.encode(&mut writer)?;
        self.alternate_group.encode(&mut writer)?;
        self.volume.encode(&mut writer)?;
        [0u8; 2].encode(&mut writer)?;
        self.matrix.encode(&mut writer)?;
        self.width.encode(&mut writer)?;
        self.height.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(&mut reader)?;
        let mut this = Self {
            flag_track_enabled: false,
            flag_track_in_movie: false,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,

            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
            track_id: 0,
            duration: 0,
            layer: Self::DEFAULT_LAYER,
            alternate_group: Self::DEFAULT_ALTERNATE_GROUP,
            volume: Self::DEFAULT_AUDIO_VOLUME,
            matrix: Self::DEFAULT_MATRIX,
            width: FixedPointNumber::new(0, 0),
            height: FixedPointNumber::new(0, 0),
        };

        this.flag_track_enabled = full_header.flags.is_set(0);
        this.flag_track_in_movie = full_header.flags.is_set(1);
        this.flag_track_in_preview = full_header.flags.is_set(2);
        this.flag_track_size_is_aspect_ratio = full_header.flags.is_set(3);

        if full_header.version == 1 {
            this.creation_time = u64::decode(&mut reader).map(Mp4FileTime::from_secs)?;
            this.modification_time = u64::decode(&mut reader).map(Mp4FileTime::from_secs)?;
            this.track_id = u32::decode(&mut reader)?;
            let _ = <[u8; 4]>::decode(&mut reader)?;
            this.duration = u64::decode(&mut reader)?;
        } else {
            this.creation_time =
                u32::decode(&mut reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode(&mut reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.track_id = u32::decode(&mut reader)?;
            let _ = <[u8; 4]>::decode(&mut reader)?;
            this.duration = u32::decode(&mut reader)? as u64;
        }

        let _ = <[u8; 4 * 2]>::decode(&mut reader)?;
        this.layer = i16::decode(&mut reader)?;
        this.alternate_group = i16::decode(&mut reader)?;
        this.volume = FixedPointNumber::decode(&mut reader)?;
        let _ = <[u8; 2]>::decode(&mut reader)?;
        this.matrix = <[i32; 9]>::decode(&mut reader)?;
        this.width = FixedPointNumber::decode(&mut reader)?;
        this.height = FixedPointNumber::decode(reader)?;

        Ok(this)
    }
}

impl Encode for TkhdBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for TkhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for TkhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
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
        FullBoxFlags::from_flags([
            (0, self.flag_track_enabled),
            (1, self.flag_track_in_movie),
            (2, self.flag_track_in_preview),
            (3, self.flag_track_size_is_aspect_ratio),
        ])
    }
}

/// [ISO/IEC 14496-12] EditBox class (親: [`TrakBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct EdtsBox {
    pub elst_box: Option<ElstBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl EdtsBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"edts");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        if let Some(b) = &self.elst_box {
            b.encode(&mut writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for EdtsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(self.elst_box.iter().map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [`ElstBox`] に含まれるエントリー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct ElstEntry {
    pub edit_duration: u64,
    pub media_time: i64,
    pub media_rate: FixedPointNumber<i16, i16>,
}

/// [ISO/IEC 14496-12] EditListBox class (親: [`EdtsBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct ElstBox {
    pub entries: Vec<ElstEntry>,
}

impl ElstBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"elst");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;

        let version = self.full_box_version();
        (self.entries.len() as u32).encode(&mut writer)?;
        for entry in &self.entries {
            if version == 1 {
                entry.edit_duration.encode(&mut writer)?;
                entry.media_time.encode(&mut writer)?;
            } else {
                (entry.edit_duration as u32).encode(&mut writer)?;
                (entry.media_time as i32).encode(&mut writer)?;
            }
            entry.media_rate.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(&mut reader)?;

        let mut entries = Vec::new();
        let count = u32::decode(&mut reader)? as usize;
        for _ in 0..count {
            let edit_duration;
            let media_time;
            if full_header.version == 1 {
                edit_duration = u64::decode(&mut reader)?;
                media_time = i64::decode(&mut reader)?;
            } else {
                edit_duration = u32::decode(&mut reader)? as u64;
                media_time = i32::decode(&mut reader)? as i64;
            }
            let media_rate = FixedPointNumber::decode(&mut reader)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for ElstBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] MediaBox class (親: [`TrakBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct MdiaBox {
    pub mdhd_box: MdhdBox,
    pub hdlr_box: HdlrBox,
    pub minf_box: MinfBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MdiaBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"mdia");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.mdhd_box.encode(&mut writer)?;
        self.hdlr_box.encode(&mut writer)?;
        self.minf_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MdiaBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.mdhd_box).map(as_box_object))
                .chain(std::iter::once(&self.hdlr_box).map(as_box_object))
                .chain(std::iter::once(&self.minf_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-12] MediaHeaderBox class (親: [`MdiaBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct MdhdBox {
    pub creation_time: Mp4FileTime,
    pub modification_time: Mp4FileTime,
    pub timescale: NonZeroU32,
    pub duration: u64,

    /// ISO-639-2/T language code
    pub language: [u8; 3],
}

impl MdhdBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"mdhd");

    /// 未定義を表す言語コード
    pub const LANGUAGE_UNDEFINED: [u8; 3] = *b"und";

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        if self.full_box_version() == 1 {
            self.creation_time.as_secs().encode(&mut writer)?;
            self.modification_time.as_secs().encode(&mut writer)?;
            self.timescale.encode(&mut writer)?;
            self.duration.encode(&mut writer)?;
        } else {
            (self.creation_time.as_secs() as u32).encode(&mut writer)?;
            (self.modification_time.as_secs() as u32).encode(&mut writer)?;
            self.timescale.encode(&mut writer)?;
            (self.duration as u32).encode(&mut writer)?;
        }

        let mut language: u16 = 0;
        for l in &self.language {
            language = (language << 5)
                | l.checked_sub(0x60).ok_or_else(|| {
                    Error::invalid_input(&format!("Invalid language code: {:?}", self.language))
                })? as u16;
        }
        language.encode(&mut writer)?;
        [0u8; 2].encode(writer)?;

        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(&mut reader)?;
        let mut this = Self {
            creation_time: Default::default(),
            modification_time: Default::default(),
            timescale: NonZeroU32::MIN,
            duration: Default::default(),
            language: Default::default(),
        };

        if full_header.version == 1 {
            this.creation_time = u64::decode(&mut reader).map(Mp4FileTime::from_secs)?;
            this.modification_time = u64::decode(&mut reader).map(Mp4FileTime::from_secs)?;
            this.timescale = NonZeroU32::decode(&mut reader)?;
            this.duration = u64::decode(&mut reader)?;
        } else {
            this.creation_time =
                u32::decode(&mut reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode(&mut reader).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.timescale = NonZeroU32::decode(&mut reader)?;
            this.duration = u32::decode(&mut reader)? as u64;
        }

        let language = u16::decode(&mut reader)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MdhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for MdhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
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

/// [ISO/IEC 14496-12] HandlerBox class (親: [`MdiaBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct HdlrBox {
    pub handler_type: [u8; 4],

    /// ハンドラ名
    ///
    /// ISO の仕様書上はここは [`Utf8String`] であるべきだが、
    /// 中身が UTF-8 ではなかったり、
    /// null 終端文字列ではなく先頭にサイズバイトを格納する形式で
    /// MP4 ファイルを作成する実装が普通に存在するため、
    /// ここでは単なるバイト列として扱っている
    pub name: Vec<u8>,
}

impl HdlrBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"hdlr");

    /// 音声用のハンドラー種別
    pub const HANDLER_TYPE_SOUN: [u8; 4] = *b"soun";

    /// 映像用のハンドラー種別
    pub const HANDLER_TYPE_VIDE: [u8; 4] = *b"vide";

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        [0u8; 4].encode(&mut writer)?;
        self.handler_type.encode(&mut writer)?;
        [0u8; 4 * 3].encode(&mut writer)?;
        writer.write_all(&self.name)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _full_header = FullBoxHeader::decode(&mut reader)?;
        let _ = <[u8; 4]>::decode(&mut reader)?;
        let handler_type = <[u8; 4]>::decode(&mut reader)?;
        let _ = <[u8; 4 * 3]>::decode(&mut reader)?;
        let mut name = Vec::new();
        reader.read_to_end(&mut name)?;
        Ok(Self { handler_type, name })
    }
}

impl Encode for HdlrBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for HdlrBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] MediaInformationBox class (親: [`MdiaBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct MinfBox {
    pub smhd_or_vmhd_box: Either<SmhdBox, VmhdBox>,
    pub dinf_box: DinfBox,
    pub stbl_box: StblBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MinfBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"minf");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        match &self.smhd_or_vmhd_box {
            Either::A(b) => b.encode(&mut writer)?,
            Either::B(b) => b.encode(&mut writer)?,
        }
        self.dinf_box.encode(&mut writer)?;
        self.stbl_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for MinfBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.smhd_or_vmhd_box).map(as_box_object))
                .chain(std::iter::once(&self.dinf_box).map(as_box_object))
                .chain(std::iter::once(&self.stbl_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-12] SoundMediaHeaderBox class (親: [`MinfBox`]）
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct SmhdBox {
    pub balance: FixedPointNumber<u8, u8>,
}

impl SmhdBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"smhd");

    /// [`SmhdBox::balance`] のデフォルト値（中央）
    pub const DEFAULT_BALANCE: FixedPointNumber<u8, u8> = FixedPointNumber::new(0, 0);

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        self.balance.encode(&mut writer)?;
        [0u8; 2].encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _full_header = FullBoxHeader::decode(&mut reader)?;
        let balance = FixedPointNumber::decode(&mut reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        Ok(Self { balance })
    }
}

impl Encode for SmhdBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for SmhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] VideoMediaHeaderBox class (親: [`MinfBox`]）
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct VmhdBox {
    pub graphicsmode: u16,
    pub opcolor: [u16; 3],
}

impl VmhdBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"vmhd");

    /// [`Vmhd::graphicsmode`] のデフォルト値（コピー）
    pub const DEFAULT_GRAPHICSMODE: u16 = 0;

    /// [`Vmhd::graphicsmode`] のデフォルト値
    pub const DEFAULT_OPCOLOR: [u16; 3] = [0, 0, 0];

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        self.graphicsmode.encode(&mut writer)?;
        self.opcolor.encode(writer)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(&mut reader)?;
        if full_header.flags.get() != 1 {
            return Err(Error::invalid_data(&format!(
                "Unexpected FullBox header flags of 'vmhd' box: {}",
                full_header.flags.get()
            )));
        }

        let graphicsmode = u16::decode(&mut reader)?;
        let opcolor = <[u16; 3]>::decode(reader)?;
        Ok(Self {
            graphicsmode,
            opcolor,
        })
    }
}

impl Encode for VmhdBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for VmhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] DataInformationBox class (親: [`MinfBox`]）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct DinfBox {
    pub dref_box: DrefBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl DinfBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"dinf");

    /// メディアデータが同じファイル内に格納されていることを示す [`DinfBox`] の値
    pub const LOCAL_FILE: Self = Self {
        dref_box: DrefBox::LOCAL_FILE,
        unknown_boxes: Vec::new(),
    };

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.dref_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for DinfBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.dref_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-12] DataReferenceBox class (親: [`DinfBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct DrefBox {
    pub url_box: Option<UrlBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl DrefBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"dref");

    /// メディアデータが同じファイル内に格納されていることを示す [`DrefBox`] の値
    pub const LOCAL_FILE: Self = Self {
        url_box: Some(UrlBox::LOCAL_FILE),
        unknown_boxes: Vec::new(),
    };

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        let entry_count = (self.url_box.is_some() as usize + self.unknown_boxes.len()) as u32;
        entry_count.encode(&mut writer)?;
        if let Some(b) = &self.url_box {
            b.encode(&mut writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let entry_count = u32::decode(&mut reader)?;
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

impl Encode for DrefBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for DrefBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(self.url_box.iter().map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
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

/// [ISO/IEC 14496-12] DataEntryUrlBox class (親: [`DrefBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct UrlBox {
    pub location: Option<Utf8String>,
}

impl UrlBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"url ");

    /// メディアデータが同じファイル内に格納されていることを示す [`UrlBox`] の値
    pub const LOCAL_FILE: Self = Self { location: None };

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        if let Some(l) = &self.location {
            l.encode(writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let full_header = FullBoxHeader::decode(&mut reader)?;
        let location = if full_header.flags.is_set(0) {
            None
        } else {
            Some(Utf8String::decode(reader)?)
        };
        Ok(Self { location })
    }
}

impl Encode for UrlBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for UrlBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] SampleTableBox class (親: [`MinfBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct StblBox {
    pub stsd_box: StsdBox,
    pub stts_box: SttsBox,
    pub stsc_box: StscBox,
    pub stsz_box: StszBox,
    pub stco_or_co64_box: Either<StcoBox, Co64Box>,
    pub stss_box: Option<StssBox>,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl StblBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stbl");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.stsd_box.encode(&mut writer)?;
        self.stts_box.encode(&mut writer)?;
        self.stsc_box.encode(&mut writer)?;
        self.stsz_box.encode(&mut writer)?;
        match &self.stco_or_co64_box {
            Either::A(b) => b.encode(&mut writer)?,
            Either::B(b) => b.encode(&mut writer)?,
        }
        if let Some(b) = &self.stss_box {
            b.encode(&mut writer)?;
        }
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
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
        let mut stss_box = None;
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
                StssBox::TYPE if stss_box.is_none() => {
                    stss_box = Some(StssBox::decode(&mut reader)?);
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
            stss_box,
            unknown_boxes,
        })
    }
}

impl Encode for StblBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StblBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.stsd_box).map(as_box_object))
                .chain(std::iter::once(&self.stts_box).map(as_box_object))
                .chain(std::iter::once(&self.stsc_box).map(as_box_object))
                .chain(std::iter::once(&self.stsz_box).map(as_box_object))
                .chain(std::iter::once(&self.stco_or_co64_box).map(as_box_object))
                .chain(self.stss_box.iter().map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

impl AsRef<StblBox> for StblBox {
    fn as_ref(&self) -> &StblBox {
        self
    }
}

/// [ISO/IEC 14496-12] SampleDescriptionBox class (親: [`StblBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct StsdBox {
    pub entries: Vec<SampleEntry>,
}

impl StsdBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stsd");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        let entry_count = (self.entries.len()) as u32;
        entry_count.encode(&mut writer)?;
        for b in &self.entries {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let entry_count = u32::decode(&mut reader)?;
        let mut entries = Vec::new();
        for _ in 0..entry_count {
            entries.push(SampleEntry::decode(&mut reader)?);
        }
        Ok(Self { entries })
    }
}

impl Encode for StsdBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StsdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(self.entries.iter().map(as_box_object))
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

/// [`StsdBox`] に含まれるエントリー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum SampleEntry {
    Avc1(Avc1Box),
    Hev1(Hev1Box),
    Vp08(Vp08Box),
    Vp09(Vp09Box),
    Av01(Av01Box),
    Opus(OpusBox),
    Unknown(UnknownBox),
}

impl SampleEntry {
    fn inner_box(&self) -> &dyn BaseBox {
        match self {
            Self::Avc1(b) => b,
            Self::Hev1(b) => b,
            Self::Vp08(b) => b,
            Self::Vp09(b) => b,
            Self::Av01(b) => b,
            Self::Opus(b) => b,
            Self::Unknown(b) => b,
        }
    }
}

impl Encode for SampleEntry {
    fn encode<W: Write>(&self, writer: W) -> Result<()> {
        match self {
            Self::Avc1(b) => b.encode(writer),
            Self::Hev1(b) => b.encode(writer),
            Self::Vp08(b) => b.encode(writer),
            Self::Vp09(b) => b.encode(writer),
            Self::Av01(b) => b.encode(writer),
            Self::Opus(b) => b.encode(writer),
            Self::Unknown(b) => b.encode(writer),
        }
    }
}

impl Decode for SampleEntry {
    fn decode<R: Read>(reader: R) -> Result<Self> {
        let (header, mut reader) = BoxHeader::peek(reader)?;
        match header.box_type {
            Avc1Box::TYPE => Decode::decode(&mut reader).map(Self::Avc1),
            Hev1Box::TYPE => Decode::decode(&mut reader).map(Self::Hev1),
            Vp08Box::TYPE => Decode::decode(&mut reader).map(Self::Vp08),
            Vp09Box::TYPE => Decode::decode(&mut reader).map(Self::Vp09),
            Av01Box::TYPE => Decode::decode(&mut reader).map(Self::Av01),
            OpusBox::TYPE => Decode::decode(&mut reader).map(Self::Opus),
            _ => Decode::decode(&mut reader).map(Self::Unknown),
        }
    }
}

impl BaseBox for SampleEntry {
    fn box_type(&self) -> BoxType {
        self.inner_box().box_type()
    }

    fn box_size(&self) -> BoxSize {
        self.inner_box().box_size()
    }

    fn box_payload_size(&self) -> u64 {
        self.inner_box().box_payload_size()
    }

    fn is_unknown_box(&self) -> bool {
        self.inner_box().is_unknown_box()
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.inner_box().children()
    }
}

/// 映像系の [`SampleEntry`] に共通のフィールドをまとめた構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct VisualSampleEntryFields {
    pub data_reference_index: NonZeroU16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: FixedPointNumber<u16, u16>,
    pub vertresolution: FixedPointNumber<u16, u16>,
    pub frame_count: u16,
    pub compressorname: [u8; 32],
    pub depth: u16,
}

impl VisualSampleEntryFields {
    /// [`VisualSampleEntryFields::data_reference_index`] のデフォルト値
    pub const DEFAULT_DATA_REFERENCE_INDEX: NonZeroU16 = NonZeroU16::MIN;

    /// [`VisualSampleEntryFields::horizresolution`] のデフォルト値 (72 dpi)
    pub const DEFAULT_HORIZRESOLUTION: FixedPointNumber<u16, u16> = FixedPointNumber::new(0x48, 0);

    /// [`VisualSampleEntryFields::vertresolution`] のデフォルト値 (72 dpi)
    pub const DEFAULT_VERTRESOLUTION: FixedPointNumber<u16, u16> = FixedPointNumber::new(0x48, 0);

    /// [`VisualSampleEntryFields::frame_count`] のデフォルト値 (1)
    pub const DEFAULT_FRAME_COUNT: u16 = 1;

    /// [`VisualSampleEntryFields::depth`] のデフォルト値 (images are in colour with no alpha)
    pub const DEFAULT_DEPTH: u16 = 0x0018;

    /// 名前なしを表す [`VisualSampleEntryFields::compressorname`] の値
    pub const NULL_COMPRESSORNAME: [u8; 32] = [0; 32];
}

impl Encode for VisualSampleEntryFields {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        [0u8; 6].encode(&mut writer)?;
        self.data_reference_index.encode(&mut writer)?;
        [0u8; 2 + 2 + 4 * 3].encode(&mut writer)?;
        self.width.encode(&mut writer)?;
        self.height.encode(&mut writer)?;
        self.horizresolution.encode(&mut writer)?;
        self.vertresolution.encode(&mut writer)?;
        [0u8; 4].encode(&mut writer)?;
        self.frame_count.encode(&mut writer)?;
        self.compressorname.encode(&mut writer)?;
        self.depth.encode(&mut writer)?;
        (-1i16).encode(writer)?;
        Ok(())
    }
}

impl Decode for VisualSampleEntryFields {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let _ = <[u8; 6]>::decode(&mut reader)?;
        let data_reference_index = NonZeroU16::decode(&mut reader)?;
        let _ = <[u8; 2 + 2 + 4 * 3]>::decode(&mut reader)?;
        let width = u16::decode(&mut reader)?;
        let height = u16::decode(&mut reader)?;
        let horizresolution = FixedPointNumber::decode(&mut reader)?;
        let vertresolution = FixedPointNumber::decode(&mut reader)?;
        let _ = <[u8; 4]>::decode(&mut reader)?;
        let frame_count = u16::decode(&mut reader)?;
        let compressorname = <[u8; 32]>::decode(&mut reader)?;
        let depth = u16::decode(&mut reader)?;
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

/// [ISO/IEC 14496-15] AVCSampleEntry class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Avc1Box {
    pub visual: VisualSampleEntryFields,
    pub avcc_box: AvccBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Avc1Box {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"avc1");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.visual.encode(&mut writer)?;
        self.avcc_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let visual = VisualSampleEntryFields::decode(&mut reader)?;
        let mut avcc_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                AvccBox::TYPE if avcc_box.is_none() => {
                    avcc_box = Some(AvccBox::decode(&mut reader)?);
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
            unknown_boxes,
        })
    }
}

impl Encode for Avc1Box {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Avc1Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.avcc_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-15] AVCConfigurationBox class (親: [`Avc1Box`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct AvccBox {
    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: Uint<u8, 2>,
    pub sps_list: Vec<Vec<u8>>,
    pub pps_list: Vec<Vec<u8>>,
    pub chroma_format: Option<Uint<u8, 2>>,
    pub bit_depth_luma_minus8: Option<Uint<u8, 3>>,
    pub bit_depth_chroma_minus8: Option<Uint<u8, 3>>,
    pub sps_ext_list: Vec<Vec<u8>>,
}

impl AvccBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"avcC");

    const CONFIGURATION_VERSION: u8 = 1;

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        Self::CONFIGURATION_VERSION.encode(&mut writer)?;
        self.avc_profile_indication.encode(&mut writer)?;
        self.profile_compatibility.encode(&mut writer)?;
        self.avc_level_indication.encode(&mut writer)?;
        (0b1111_1100 | self.length_size_minus_one.get()).encode(&mut writer)?;

        let sps_count =
            u8::try_from(self.sps_list.len()).map_err(|_| Error::invalid_input("Too many SPSs"))?;
        (0b1110_0000 | sps_count).encode(&mut writer)?;
        for sps in &self.sps_list {
            let size = u16::try_from(sps.len())
                .map_err(|e| Error::invalid_input(&format!("Too long SPS: {e}")))?;
            size.encode(&mut writer)?;
            writer.write_all(sps)?;
        }

        let pps_count =
            u8::try_from(self.pps_list.len()).map_err(|_| Error::invalid_input("Too many PPSs"))?;
        pps_count.encode(&mut writer)?;
        for pps in &self.pps_list {
            let size = u16::try_from(pps.len())
                .map_err(|e| Error::invalid_input(&format!("Too long PPS: {e}")))?;
            size.encode(&mut writer)?;
            writer.write_all(pps)?;
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
            (0b1111_1100 | chroma_format.get()).encode(&mut writer)?;
            (0b1111_1000 | bit_depth_luma_minus8.get()).encode(&mut writer)?;
            (0b1111_1000 | bit_depth_chroma_minus8.get()).encode(&mut writer)?;

            let sps_ext_count = u8::try_from(self.sps_ext_list.len())
                .map_err(|_| Error::invalid_input("Too many SPS EXTs"))?;
            sps_ext_count.encode(&mut writer)?;
            for sps_ext in &self.sps_ext_list {
                let size = u16::try_from(sps_ext.len())
                    .map_err(|e| Error::invalid_input(&format!("Too long SPS EXT: {e}")))?;
                size.encode(&mut writer)?;
                writer.write_all(sps_ext)?;
            }
        }

        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let configuration_version = u8::decode(&mut reader)?;
        if configuration_version != Self::CONFIGURATION_VERSION {
            return Err(Error::invalid_data(&format!(
                "Unsupported avcC configuration version: {configuration_version}"
            )));
        }

        let avc_profile_indication = u8::decode(&mut reader)?;
        let profile_compatibility = u8::decode(&mut reader)?;
        let avc_level_indication = u8::decode(&mut reader)?;
        let length_size_minus_one = Uint::from_bits(u8::decode(&mut reader)?);

        let sps_count = Uint::<u8, 5>::from_bits(u8::decode(&mut reader)?).get() as usize;
        let mut sps_list = Vec::with_capacity(sps_count);
        for _ in 0..sps_count {
            let size = u16::decode(&mut reader)? as usize;
            let mut sps = vec![0; size];
            reader.read_exact(&mut sps)?;
            sps_list.push(sps);
        }

        let pps_count = u8::decode(&mut reader)? as usize;
        let mut pps_list = Vec::with_capacity(pps_count);
        for _ in 0..pps_count {
            let size = u16::decode(&mut reader)? as usize;
            let mut pps = vec![0; size];
            reader.read_exact(&mut pps)?;
            pps_list.push(pps);
        }

        let mut chroma_format = None;
        let mut bit_depth_luma_minus8 = None;
        let mut bit_depth_chroma_minus8 = None;
        let mut sps_ext_list = Vec::new();
        if !matches!(avc_profile_indication, 66 | 77 | 88) {
            chroma_format = Some(Uint::from_bits(u8::decode(&mut reader)?));
            bit_depth_luma_minus8 = Some(Uint::from_bits(u8::decode(&mut reader)?));
            bit_depth_chroma_minus8 = Some(Uint::from_bits(u8::decode(&mut reader)?));

            let sps_ext_count = u8::decode(&mut reader)? as usize;
            for _ in 0..sps_ext_count {
                let size = u16::decode(&mut reader)? as usize;
                let mut pps = vec![0; size];
                reader.read_exact(&mut pps)?;
                sps_ext_list.push(pps);
            }
        }

        Ok(Self {
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

impl Encode for AvccBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for AvccBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [ISO/IEC 14496-15] HEVCSampleEntry class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Hev1Box {
    pub visual: VisualSampleEntryFields,
    pub hvcc_box: HvccBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Hev1Box {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"hev1");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.visual.encode(&mut writer)?;
        self.hvcc_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let visual = VisualSampleEntryFields::decode(&mut reader)?;
        let mut hvcc_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                HvccBox::TYPE if hvcc_box.is_none() => {
                    hvcc_box = Some(HvccBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let hvcc_box = hvcc_box.ok_or_else(|| Error::missing_box("hvcc", Self::TYPE))?;
        Ok(Self {
            visual,
            hvcc_box,
            unknown_boxes,
        })
    }
}

impl Encode for Hev1Box {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Hev1Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Hev1Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.hvcc_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [`HvccBox`] 内の NAL ユニット配列を保持する構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct HvccNalUintArray {
    pub array_completeness: Uint<u8, 1, 7>,
    pub nal_unit_type: Uint<u8, 6, 0>,
    pub nalus: Vec<Vec<u8>>,
}

/// [ISO/IEC 14496-15] HVCConfigurationBox class (親: [`Hev1Box`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct HvccBox {
    pub general_profile_space: Uint<u8, 2, 6>,
    pub general_tier_flag: Uint<u8, 1, 5>,
    pub general_profile_idc: Uint<u8, 5, 0>,
    pub general_profile_compatibility_flags: u32,
    pub general_constraint_indicator_flags: Uint<u64, 48>,
    pub general_level_idc: u8,
    pub min_spatial_segmentation_idc: Uint<u16, 12>,
    pub parallelism_type: Uint<u8, 2>,
    pub chroma_format_idc: Uint<u8, 2>,
    pub bit_depth_luma_minus8: Uint<u8, 3>,
    pub bit_depth_chroma_minus8: Uint<u8, 3>,
    pub avg_frame_rate: u16,
    pub constant_frame_rate: Uint<u8, 2, 6>,
    pub num_temporal_layers: Uint<u8, 3, 3>,
    pub temporal_id_nested: Uint<u8, 1, 2>,
    pub length_size_minus_one: Uint<u8, 2, 0>,
    pub nalu_arrays: Vec<HvccNalUintArray>,
}

impl HvccBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"hvcC");

    const CONFIGURATION_VERSION: u8 = 1;

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        Self::CONFIGURATION_VERSION.encode(&mut writer)?;
        (self.general_profile_space.to_bits()
            | self.general_tier_flag.to_bits()
            | self.general_profile_idc.to_bits())
        .encode(&mut writer)?;
        self.general_profile_compatibility_flags
            .encode(&mut writer)?;
        writer.write_all(&self.general_constraint_indicator_flags.get().to_be_bytes()[2..])?;
        self.general_level_idc.encode(&mut writer)?;
        (0b1111_0000_0000_0000 | self.min_spatial_segmentation_idc.to_bits())
            .encode(&mut writer)?;
        (0b1111_1100 | self.parallelism_type.to_bits()).encode(&mut writer)?;
        (0b1111_1100 | self.chroma_format_idc.to_bits()).encode(&mut writer)?;
        (0b1111_1000 | self.bit_depth_luma_minus8.to_bits()).encode(&mut writer)?;
        (0b1111_1000 | self.bit_depth_chroma_minus8.to_bits()).encode(&mut writer)?;
        self.avg_frame_rate.encode(&mut writer)?;
        (self.constant_frame_rate.to_bits()
            | self.num_temporal_layers.to_bits()
            | self.temporal_id_nested.to_bits()
            | self.length_size_minus_one.to_bits())
        .encode(&mut writer)?;
        u8::try_from(self.nalu_arrays.len())
            .map_err(|_| {
                Error::invalid_input(&format!("Too many NALU arrays: {}", self.nalu_arrays.len()))
            })?
            .encode(&mut writer)?;
        for nalu_array in &self.nalu_arrays {
            (nalu_array.array_completeness.to_bits() | nalu_array.nal_unit_type.to_bits())
                .encode(&mut writer)?;
            u16::try_from(nalu_array.nalus.len())
                .map_err(|_| {
                    Error::invalid_input(&format!("Too many NALUs: {}", self.nalu_arrays.len()))
                })?
                .encode(&mut writer)?;
            for nalu in &nalu_array.nalus {
                u16::try_from(nalu.len())
                    .map_err(|_| {
                        Error::invalid_input(&format!("Too large NALU: {}", self.nalu_arrays.len()))
                    })?
                    .encode(&mut writer)?;
                writer.write_all(nalu)?;
            }
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let configuration_version = u8::decode(&mut reader)?;
        if configuration_version != Self::CONFIGURATION_VERSION {
            return Err(Error::invalid_data(&format!(
                "Unsupported avcC version: {configuration_version}"
            )));
        }

        let b = u8::decode(&mut reader)?;
        let general_profile_space = Uint::from_bits(b);
        let general_tier_flag = Uint::from_bits(b);
        let general_profile_idc = Uint::from_bits(b);

        let general_profile_compatibility_flags = u32::decode(&mut reader)?;

        let mut buf = [0; 8];
        reader.read_exact(&mut buf[2..])?;
        let general_constraint_indicator_flags = Uint::from_bits(u64::from_be_bytes(buf));

        let general_level_idc = u8::decode(&mut reader)?;
        let min_spatial_segmentation_idc = Uint::from_bits(u16::decode(&mut reader)?);
        let parallelism_type = Uint::from_bits(u8::decode(&mut reader)?);
        let chroma_format_idc = Uint::from_bits(u8::decode(&mut reader)?);
        let bit_depth_luma_minus8 = Uint::from_bits(u8::decode(&mut reader)?);
        let bit_depth_chroma_minus8 = Uint::from_bits(u8::decode(&mut reader)?);
        let avg_frame_rate = u16::decode(&mut reader)?;

        let b = u8::decode(&mut reader)?;
        let constant_frame_rate = Uint::from_bits(b);
        let num_temporal_layers = Uint::from_bits(b);
        let temporal_id_nested = Uint::from_bits(b);
        let length_size_minus_one = Uint::from_bits(b);

        let num_of_arrays = u8::decode(&mut reader)?;
        let mut nalu_arrays = Vec::new();
        for _ in 0..num_of_arrays {
            let b = u8::decode(&mut reader)?;
            let array_completeness = Uint::from_bits(b);
            let nal_unit_type = Uint::from_bits(b);

            let num_nalus = u16::decode(&mut reader)?;
            let mut nalus = Vec::new();
            for _ in 0..num_nalus {
                let nal_unit_length = u16::decode(&mut reader)? as usize;
                let mut nal_unit = vec![0; nal_unit_length];
                reader.read_exact(&mut nal_unit)?;
                nalus.push(nal_unit);
            }
            nalu_arrays.push(HvccNalUintArray {
                array_completeness,
                nal_unit_type,
                nalus,
            });
        }

        Ok(Self {
            general_profile_space,
            general_tier_flag,
            general_profile_idc,
            general_profile_compatibility_flags,
            general_constraint_indicator_flags,
            general_level_idc,
            min_spatial_segmentation_idc,
            parallelism_type,
            chroma_format_idc,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            avg_frame_rate,
            constant_frame_rate,
            num_temporal_layers,
            temporal_id_nested,
            length_size_minus_one,
            nalu_arrays,
        })
    }
}

impl Encode for HvccBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for HvccBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for HvccBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [<https://www.webmproject.org/vp9/mp4/>] VP8SampleEntry class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Vp08Box {
    pub visual: VisualSampleEntryFields,
    pub vpcc_box: VpccBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Vp08Box {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"vp08");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.visual.encode(&mut writer)?;
        self.vpcc_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let visual = VisualSampleEntryFields::decode(&mut reader)?;
        let mut vpcc_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                VpccBox::TYPE if vpcc_box.is_none() => {
                    vpcc_box = Some(VpccBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let vpcc_box = vpcc_box.ok_or_else(|| Error::missing_box("vpcC", Self::TYPE))?;
        Ok(Self {
            visual,
            vpcc_box,
            unknown_boxes,
        })
    }
}

impl Encode for Vp08Box {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Vp08Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Vp08Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.vpcc_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [<https://www.webmproject.org/vp9/mp4/>] VP9SampleEntry class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Vp09Box {
    pub visual: VisualSampleEntryFields,
    pub vpcc_box: VpccBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Vp09Box {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"vp09");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.visual.encode(&mut writer)?;
        self.vpcc_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let visual = VisualSampleEntryFields::decode(&mut reader)?;
        let mut vpcc_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                VpccBox::TYPE if vpcc_box.is_none() => {
                    vpcc_box = Some(VpccBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let vpcc_box = vpcc_box.ok_or_else(|| Error::missing_box("vpcC", Self::TYPE))?;
        Ok(Self {
            visual,
            vpcc_box,
            unknown_boxes,
        })
    }
}

impl Encode for Vp09Box {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Vp09Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Vp09Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.vpcc_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [<https://www.webmproject.org/vp9/mp4/>] VPCodecConfigurationBox class (親: [`Vp08Box`], [`Vp09Box`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct VpccBox {
    pub profile: u8,
    pub level: u8,
    pub bit_depth: Uint<u8, 4, 4>,
    pub chroma_subsampling: Uint<u8, 3, 1>,
    pub video_full_range_flag: Uint<u8, 1>,
    pub colour_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
    pub codec_initialization_data: Vec<u8>,
}

impl VpccBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"vpcC");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        self.profile.encode(&mut writer)?;
        self.level.encode(&mut writer)?;
        (self.bit_depth.to_bits()
            | self.chroma_subsampling.to_bits()
            | self.video_full_range_flag.to_bits())
        .encode(&mut writer)?;
        self.colour_primaries.encode(&mut writer)?;
        self.transfer_characteristics.encode(&mut writer)?;
        self.matrix_coefficients.encode(&mut writer)?;
        (self.codec_initialization_data.len() as u16).encode(&mut writer)?;
        writer.write_all(&self.codec_initialization_data)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let header = FullBoxHeader::decode(&mut reader)?;
        if header.version != 1 {
            return Err(Error::invalid_data(&format!(
                "Unexpected full box header version: box=vpcC, version={}",
                header.version
            )));
        }

        let profile = u8::decode(&mut reader)?;
        let level = u8::decode(&mut reader)?;

        let b = u8::decode(&mut reader)?;
        let bit_depth = Uint::from_bits(b);
        let chroma_subsampling = Uint::from_bits(b);
        let video_full_range_flag = Uint::from_bits(b);
        let colour_primaries = u8::decode(&mut reader)?;
        let transfer_characteristics = u8::decode(&mut reader)?;
        let matrix_coefficients = u8::decode(&mut reader)?;
        let mut codec_initialization_data = vec![0; u16::decode(&mut reader)? as usize];
        reader.read_exact(&mut codec_initialization_data)?;

        Ok(Self {
            profile,
            level,
            bit_depth,
            chroma_subsampling,
            video_full_range_flag,
            colour_primaries,
            transfer_characteristics,
            matrix_coefficients,
            codec_initialization_data,
        })
    }
}

impl Encode for VpccBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for VpccBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for VpccBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for VpccBox {
    fn full_box_version(&self) -> u8 {
        1
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [<https://aomediacodec.github.io/av1-isobmff/>] AV1SampleEntry class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Av01Box {
    pub visual: VisualSampleEntryFields,
    pub av1c_box: Av1cBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Av01Box {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"av01");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.visual.encode(&mut writer)?;
        self.av1c_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let visual = VisualSampleEntryFields::decode(&mut reader)?;
        let mut av1c_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                Av1cBox::TYPE if av1c_box.is_none() => {
                    av1c_box = Some(Av1cBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let av1c_box = av1c_box.ok_or_else(|| Error::missing_box("av1c", Self::TYPE))?;
        Ok(Self {
            visual,
            av1c_box,
            unknown_boxes,
        })
    }
}

impl Encode for Av01Box {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Av01Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Av01Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.av1c_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [<https://aomediacodec.github.io/av1-isobmff/>] AV1CodecConfigurationBox class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Av1cBox {
    pub seq_profile: Uint<u8, 3, 5>,
    pub seq_level_idx_0: Uint<u8, 5, 0>,
    pub seq_tier_0: Uint<u8, 1, 7>,
    pub high_bitdepth: Uint<u8, 1, 6>,
    pub twelve_bit: Uint<u8, 1, 5>,
    pub monochrome: Uint<u8, 1, 4>,
    pub chroma_subsampling_x: Uint<u8, 1, 3>,
    pub chroma_subsampling_y: Uint<u8, 1, 2>,
    pub chroma_sample_position: Uint<u8, 2, 0>,
    pub initial_presentation_delay_minus_one: Option<Uint<u8, 4, 0>>,
    pub config_obus: Vec<u8>,
}

impl Av1cBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"av1C");

    const MARKER: Uint<u8, 1, 7> = Uint::new(1);
    const VERSION: Uint<u8, 7, 0> = Uint::new(1);

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        (Self::MARKER.to_bits() | Self::VERSION.to_bits()).encode(&mut writer)?;
        (self.seq_profile.to_bits() | self.seq_level_idx_0.to_bits()).encode(&mut writer)?;
        (self.seq_tier_0.to_bits()
            | self.high_bitdepth.to_bits()
            | self.twelve_bit.to_bits()
            | self.monochrome.to_bits()
            | self.chroma_subsampling_x.to_bits()
            | self.chroma_subsampling_y.to_bits()
            | self.chroma_sample_position.to_bits())
        .encode(&mut writer)?;
        if let Some(v) = self.initial_presentation_delay_minus_one {
            (0b1_0000 | v.to_bits()).encode(&mut writer)?;
        } else {
            0u8.encode(&mut writer)?;
        }
        writer.write_all(&self.config_obus)?;
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let b = u8::decode(&mut reader)?;
        let marker = Uint::from_bits(b);
        let version = Uint::from_bits(b);
        if marker != Self::MARKER {
            return Err(Error::invalid_data("Unexpected av1C marker"));
        }
        if version != Self::VERSION {
            return Err(Error::invalid_data(&format!(
                "Unsupported av1C version: {}",
                version.get()
            )));
        }

        let b = u8::decode(&mut reader)?;
        let seq_profile = Uint::from_bits(b);
        let seq_level_idx_0 = Uint::from_bits(b);

        let b = u8::decode(&mut reader)?;
        let seq_tier_0 = Uint::from_bits(b);
        let high_bitdepth = Uint::from_bits(b);
        let twelve_bit = Uint::from_bits(b);
        let monochrome = Uint::from_bits(b);
        let chroma_subsampling_x = Uint::from_bits(b);
        let chroma_subsampling_y = Uint::from_bits(b);
        let chroma_sample_position = Uint::from_bits(b);

        let b = u8::decode(&mut reader)?;
        let initial_presentation_delay_minus_one = if Uint::<u8, 1, 4>::from_bits(b).get() == 1 {
            Some(Uint::from_bits(b))
        } else {
            None
        };

        let mut config_obus = Vec::new();
        reader.read_to_end(&mut config_obus)?;

        Ok(Self {
            seq_profile,
            seq_level_idx_0,
            seq_tier_0,
            high_bitdepth,
            twelve_bit,
            monochrome,
            chroma_subsampling_x,
            chroma_subsampling_y,
            chroma_sample_position,
            initial_presentation_delay_minus_one,
            config_obus,
        })
    }
}

impl Encode for Av1cBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Av1cBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for Av1cBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

/// [`SttsBox`] が保持するエントリー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct SttsEntry {
    pub sample_count: u32,
    pub sample_delta: u32,
}

/// [ISO/IEC 14496-12] TimeToSampleBox class (親: [`StblBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct SttsBox {
    pub entries: Vec<SttsEntry>,
}

impl SttsBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stts");

    /// サンプル群の尺を走査するイテレーターを受け取って、対応する [`SttsBox`] インスタンスを作成する
    pub fn from_sample_deltas<I>(sample_deltas: I) -> Self
    where
        I: IntoIterator<Item = u32>,
    {
        let mut entries = Vec::<SttsEntry>::new();
        for sample_delta in sample_deltas {
            if let Some(last) = entries.last_mut() {
                if last.sample_delta == sample_delta {
                    last.sample_count += 1;
                    continue;
                }
            }
            entries.push(SttsEntry {
                sample_count: 1,
                sample_delta,
            });
        }
        Self { entries }
    }

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        (self.entries.len() as u32).encode(&mut writer)?;
        for entry in &self.entries {
            entry.sample_count.encode(&mut writer)?;
            entry.sample_delta.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let count = u32::decode(&mut reader)? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(SttsEntry {
                sample_count: u32::decode(&mut reader)?,
                sample_delta: u32::decode(&mut reader)?,
            });
        }
        Ok(Self { entries })
    }
}

impl Encode for SttsBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for SttsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [`StscBox`] が保持するエントリー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct StscEntry {
    pub first_chunk: NonZeroU32,
    pub sample_per_chunk: u32,
    pub sample_description_index: NonZeroU32,
}

/// [ISO/IEC 14496-12] SampleToChunkBox class (親: [`StblBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct StscBox {
    pub entries: Vec<StscEntry>,
}

impl StscBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stsc");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        (self.entries.len() as u32).encode(&mut writer)?;
        for entry in &self.entries {
            entry.first_chunk.encode(&mut writer)?;
            entry.sample_per_chunk.encode(&mut writer)?;
            entry.sample_description_index.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let count = u32::decode(&mut reader)? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(StscEntry {
                first_chunk: NonZeroU32::decode(&mut reader)?,
                sample_per_chunk: u32::decode(&mut reader)?,
                sample_description_index: NonZeroU32::decode(&mut reader)?,
            });
        }
        Ok(Self { entries })
    }
}

impl Encode for StscBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StscBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] SampleSizeBox class (親: [`StblBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
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
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stsz");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        match self {
            StszBox::Fixed {
                sample_size,
                sample_count,
            } => {
                sample_size.get().encode(&mut writer)?;
                sample_count.encode(writer)?;
            }
            StszBox::Variable { entry_sizes } => {
                0u32.encode(&mut writer)?;
                (entry_sizes.len() as u32).encode(&mut writer)?;
                for size in entry_sizes {
                    size.encode(&mut writer)?;
                }
            }
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let sample_size = u32::decode(&mut reader)?;
        let sample_count = u32::decode(&mut reader)?;
        if let Some(sample_size) = NonZeroU32::new(sample_size) {
            Ok(Self::Fixed {
                sample_size,
                sample_count,
            })
        } else {
            let mut entry_sizes = Vec::with_capacity(sample_count as usize);
            for _ in 0..sample_count {
                entry_sizes.push(u32::decode(&mut reader)?);
            }
            Ok(Self::Variable { entry_sizes })
        }
    }
}

impl Encode for StszBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StszBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] ChunkOffsetBox class (親: [`StblBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct StcoBox {
    pub chunk_offsets: Vec<u32>,
}

impl StcoBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stco");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        (self.chunk_offsets.len() as u32).encode(&mut writer)?;
        for offset in &self.chunk_offsets {
            offset.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let count = u32::decode(&mut reader)? as usize;
        let mut chunk_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            chunk_offsets.push(u32::decode(&mut reader)?);
        }
        Ok(Self { chunk_offsets })
    }
}

impl Encode for StcoBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StcoBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] ChunkLargeOffsetBox class (親: [`StblBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Co64Box {
    pub chunk_offsets: Vec<u64>,
}

impl Co64Box {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"co64");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        (self.chunk_offsets.len() as u32).encode(&mut writer)?;
        for offset in &self.chunk_offsets {
            offset.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let count = u32::decode(&mut reader)? as usize;
        let mut chunk_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            chunk_offsets.push(u64::decode(&mut reader)?);
        }
        Ok(Self { chunk_offsets })
    }
}

impl Encode for Co64Box {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for Co64Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

/// [ISO/IEC 14496-12] SyncSampleBox class (親: [`StssBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct StssBox {
    pub sample_numbers: Vec<NonZeroU32>,
}

impl StssBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"stss");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        FullBoxHeader::from_box(self).encode(&mut writer)?;
        (self.sample_numbers.len() as u32).encode(&mut writer)?;
        for offset in &self.sample_numbers {
            offset.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let count = u32::decode(&mut reader)? as usize;
        let mut sample_numbers = Vec::with_capacity(count);
        for _ in 0..count {
            sample_numbers.push(NonZeroU32::decode(&mut reader)?);
        }
        Ok(Self { sample_numbers })
    }
}

impl Encode for StssBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for StssBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl BaseBox for StssBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn box_payload_size(&self) -> u64 {
        ExternalBytes::calc(|writer| self.encode_payload(writer))
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}

impl FullBox for StssBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

/// [<https://gitlab.xiph.org/xiph/opus/-/blob/main/doc/opus_in_isobmff.html>] OpusSampleEntry class (親: [`StsdBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct OpusBox {
    pub audio: AudioSampleEntryFields,
    pub dops_box: DopsBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl OpusBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"Opus");

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        self.audio.encode(&mut writer)?;
        self.dops_box.encode(&mut writer)?;
        for b in &self.unknown_boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let audio = AudioSampleEntryFields::decode(&mut reader)?;
        let mut dops_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                DopsBox::TYPE if dops_box.is_none() => {
                    dops_box = Some(DopsBox::decode(&mut reader)?);
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
            unknown_boxes,
        })
    }
}

impl Encode for OpusBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for OpusBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            std::iter::empty()
                .chain(std::iter::once(&self.dops_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// 音声系の [`SampleEntry`] に共通のフィールドをまとめた構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct AudioSampleEntryFields {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: FixedPointNumber<u16, u16>,
}

impl AudioSampleEntryFields {
    /// [`AudioSampleEntryFields::sample_size`] のデフォルト値 (16)
    pub const DEFAULT_SAMPLESIZE: u16 = 16;
}

impl Encode for AudioSampleEntryFields {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        [0u8; 6].encode(&mut writer)?;
        self.data_reference_index.encode(&mut writer)?;
        [0u8; 4 * 2].encode(&mut writer)?;
        self.channelcount.encode(&mut writer)?;
        self.samplesize.encode(&mut writer)?;
        [0u8; 2].encode(&mut writer)?;
        [0u8; 2].encode(&mut writer)?;
        self.samplerate.encode(writer)?;
        Ok(())
    }
}

impl Decode for AudioSampleEntryFields {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let _ = <[u8; 6]>::decode(&mut reader)?;
        let data_reference_index = u16::decode(&mut reader)?;
        let _ = <[u8; 4 * 2]>::decode(&mut reader)?;
        let channelcount = u16::decode(&mut reader)?;
        let samplesize = u16::decode(&mut reader)?;
        let _ = <[u8; 2]>::decode(&mut reader)?;
        let _ = <[u8; 2]>::decode(&mut reader)?;
        let samplerate = FixedPointNumber::decode(reader)?;
        Ok(Self {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
        })
    }
}

/// [<https://gitlab.xiph.org/xiph/opus/-/blob/main/doc/opus_in_isobmff.html>] OpusSpecificBox class (親: [`OpusBox`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct DopsBox {
    pub output_channel_count: u8,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
    pub output_gain: i16,
}

impl DopsBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"dOps");

    const VERSION: u8 = 0;

    fn encode_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        Self::VERSION.encode(&mut writer)?;
        self.output_channel_count.encode(&mut writer)?;
        self.pre_skip.encode(&mut writer)?;
        self.input_sample_rate.encode(&mut writer)?;
        self.output_gain.encode(&mut writer)?;
        0u8.encode(writer)?; // ChannelMappingFamily
        Ok(())
    }

    fn decode_payload<R: Read>(mut reader: &mut std::io::Take<R>) -> Result<Self> {
        let version = u8::decode(&mut reader)?;
        if version != Self::VERSION {
            return Err(Error::invalid_data(&format!(
                "Unsupported dOps version: {version}"
            )));
        }

        let output_channel_count = u8::decode(&mut reader)?;
        let pre_skip = u16::decode(&mut reader)?;
        let input_sample_rate = u32::decode(&mut reader)?;
        let output_gain = i16::decode(&mut reader)?;
        let channel_mapping_family = u8::decode(reader)?;
        if channel_mapping_family != 0 {
            return Err(Error::unsupported(
                "`ChannelMappingFamily != 0` in 'dOps' box is not supported",
            ));
        }
        Ok(Self {
            output_channel_count,
            pre_skip,
            input_sample_rate,
            output_gain,
        })
    }
}

impl Encode for DopsBox {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        self.encode_payload(writer)?;
        Ok(())
    }
}

impl Decode for DopsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
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

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(std::iter::empty())
    }
}
