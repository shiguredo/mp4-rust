//! ボックス群
use core::num::{NonZeroU16, NonZeroU32};

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, format, vec, vec::Vec};

use crate::{
    BaseBox, BoxHeader, BoxSize, BoxType, Decode, Decode2, Either, Encode, Error, Error2,
    FixedPointNumber, FullBox, FullBoxFlags, FullBoxHeader, Mp4FileTime, Result, Result2, Uint,
    Utf8String,
    basic_types::as_box_object,
    descriptors::EsDescriptor,
    io::{Read, Take},
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = BoxHeader::new(self.box_type, self.box_size).encode(buf)?;
        offset += self.payload.encode(&mut buf[offset..])?;
        Ok(offset)
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

impl Decode2 for UnknownBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        Ok((
            Self {
                box_type: header.box_type,
                box_size: header.box_size,
                payload: payload.to_vec(),
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for UnknownBox {
    fn box_type(&self) -> BoxType {
        self.box_type
    }

    fn is_unknown_box(&self) -> bool {
        true
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

impl Decode2 for IgnoredBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        Ok((
            Self {
                box_type: header.box_type,
                box_size: header.box_size,
                box_payload_size: payload.len() as u64,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for IgnoredBox {
    fn box_type(&self) -> BoxType {
        self.box_type
    }

    fn is_unknown_box(&self) -> bool {
        true
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

impl core::fmt::Debug for Brand {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Ok(s) = core::str::from_utf8(&self.0) {
            f.debug_tuple("Brand").field(&s).finish()
        } else {
            f.debug_tuple("Brand").field(&self.0).finish()
        }
    }
}

impl Encode for Brand {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        self.0.encode(buf)
    }
}

impl Decode for Brand {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

impl Decode2 for Brand {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (bytes, offset) = <[u8; 4]>::decode2(buf)?;
        Ok((Self(bytes), offset))
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
}

impl Encode for FtypBox {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.major_brand.encode(&mut buf[offset..])?;
        offset += self.minor_version.encode(&mut buf[offset..])?;
        for brand in &self.compatible_brands {
            offset += brand.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
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

impl Decode2 for FtypBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let major_brand = Brand::decode_at(payload, &mut offset)?;
        let minor_version = u32::decode_at(payload, &mut offset)?;

        let mut compatible_brands = Vec::new();
        while offset < payload.len() {
            compatible_brands.push(Brand::decode_at(payload, &mut offset)?);
        }

        Ok((
            Self {
                major_brand,
                minor_version,
                compatible_brands,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for FtypBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        match self {
            RootBox::Free(b) => b.encode(buf),
            RootBox::Mdat(b) => b.encode(buf),
            RootBox::Moov(b) => b.encode(buf),
            RootBox::Unknown(b) => b.encode(buf),
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

impl Decode2 for RootBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, _header_size) = BoxHeader::decode2(buf)?;
        match header.box_type {
            FreeBox::TYPE => Decode2::decode2(buf).map(|(b, n)| (RootBox::Free(b), n)),
            MdatBox::TYPE => Decode2::decode2(buf).map(|(b, n)| (RootBox::Mdat(b), n)),
            MoovBox::TYPE => Decode2::decode2(buf).map(|(b, n)| (RootBox::Moov(b), n)),
            _ => Decode2::decode2(buf).map(|(b, n)| (RootBox::Unknown(b), n)),
        }
    }
}

impl BaseBox for RootBox {
    fn box_type(&self) -> BoxType {
        self.inner_box().box_type()
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let box_size = BoxSize::with_payload_size(Self::TYPE, self.payload.len() as u64);
        let mut offset = BoxHeader::new(Self::TYPE, box_size).encode(buf)?;
        offset += self.payload.encode(&mut buf[offset..])?;
        Ok(offset)
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

impl Decode2 for FreeBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        Ok((
            Self {
                payload: payload.to_vec(),
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for FreeBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let box_size = if self.is_variable_size {
            BoxSize::VARIABLE_SIZE
        } else {
            BoxSize::with_payload_size(Self::TYPE, self.payload.len() as u64)
        };
        let mut offset = BoxHeader::new(Self::TYPE, box_size).encode(buf)?;
        offset += self.payload.encode(&mut buf[offset..])?;
        Ok(offset)
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

impl Decode2 for MdatBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        Ok((
            Self {
                is_variable_size: header.box_size == BoxSize::VARIABLE_SIZE,
                payload: payload.to_vec(),
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for MdatBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.mvhd_box.encode(&mut buf[offset..])?;
        for b in &self.trak_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for MoovBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for MoovBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut mvhd_box = None;
        let mut trak_boxes = Vec::new();
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                MvhdBox::TYPE if mvhd_box.is_none() => {
                    mvhd_box = Some(MvhdBox::decode_at(payload, &mut offset)?);
                }
                TrakBox::TYPE => {
                    trak_boxes.push(TrakBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                mvhd_box: check_mandatory_box(mvhd_box, "mvhd", "moov")?,
                trak_boxes,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for MoovBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.mvhd_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        if self.full_box_version() == 1 {
            offset += self.creation_time.as_secs().encode(&mut buf[offset..])?;
            offset += self
                .modification_time
                .as_secs()
                .encode(&mut buf[offset..])?;
            offset += self.timescale.encode(&mut buf[offset..])?;
            offset += self.duration.encode(&mut buf[offset..])?;
        } else {
            offset += (self.creation_time.as_secs() as u32).encode(&mut buf[offset..])?;
            offset += (self.modification_time.as_secs() as u32).encode(&mut buf[offset..])?;
            offset += self.timescale.encode(&mut buf[offset..])?;
            offset += (self.duration as u32).encode(&mut buf[offset..])?;
        }
        offset += self.rate.encode(&mut buf[offset..])?;
        offset += self.volume.encode(&mut buf[offset..])?;
        offset += [0u8; 2 + 4 * 2].encode(&mut buf[offset..])?;
        offset += self.matrix.encode(&mut buf[offset..])?;
        offset += [0u8; 4 * 6].encode(&mut buf[offset..])?;
        offset += self.next_track_id.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for MvhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for MvhdBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;

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
            this.creation_time =
                u64::decode_at(payload, &mut offset).map(Mp4FileTime::from_secs)?;
            this.modification_time =
                u64::decode_at(payload, &mut offset).map(Mp4FileTime::from_secs)?;
            this.timescale = NonZeroU32::decode_at(payload, &mut offset)?;
            this.duration = u64::decode_at(payload, &mut offset)?;
        } else {
            this.creation_time =
                u32::decode_at(payload, &mut offset).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode_at(payload, &mut offset).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.timescale = NonZeroU32::decode_at(payload, &mut offset)?;
            this.duration = u32::decode_at(payload, &mut offset).map(|v| v as u64)?;
        }

        this.rate = FixedPointNumber::decode_at(payload, &mut offset)?;
        this.volume = FixedPointNumber::decode_at(payload, &mut offset)?;
        let _ = <[u8; 2 + 4 * 2]>::decode_at(payload, &mut offset)?;
        this.matrix = <[i32; 9]>::decode_at(payload, &mut offset)?;
        let _ = <[u8; 4 * 6]>::decode_at(payload, &mut offset)?;
        this.next_track_id = u32::decode_at(payload, &mut offset)?;

        Ok((this, header.external_size() + payload.len()))
    }
}

impl BaseBox for MvhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.tkhd_box.encode(&mut buf[offset..])?;
        if let Some(b) = &self.edts_box {
            offset += b.encode(&mut buf[offset..])?;
        }
        offset += self.mdia_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for TrakBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for TrakBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut tkhd_box = None;
        let mut edts_box = None;
        let mut mdia_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                TkhdBox::TYPE if tkhd_box.is_none() => {
                    tkhd_box = Some(TkhdBox::decode_at(payload, &mut offset)?);
                }
                EdtsBox::TYPE if edts_box.is_none() => {
                    edts_box = Some(EdtsBox::decode_at(payload, &mut offset)?);
                }
                MdiaBox::TYPE if mdia_box.is_none() => {
                    mdia_box = Some(MdiaBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                tkhd_box: check_mandatory_box(tkhd_box, "tkhd", "trak")?,
                edts_box,
                mdia_box: check_mandatory_box(mdia_box, "mdia", "trak")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for TrakBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.tkhd_box).map(as_box_object))
                .chain(self.edts_box.iter().map(as_box_object))
                .chain(core::iter::once(&self.mdia_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        if self.full_box_version() == 1 {
            offset += self.creation_time.as_secs().encode(&mut buf[offset..])?;
            offset += self
                .modification_time
                .as_secs()
                .encode(&mut buf[offset..])?;
            offset += self.track_id.encode(&mut buf[offset..])?;
            offset += [0u8; 4].encode(&mut buf[offset..])?;
            offset += self.duration.encode(&mut buf[offset..])?;
        } else {
            offset += (self.creation_time.as_secs() as u32).encode(&mut buf[offset..])?;
            offset += (self.modification_time.as_secs() as u32).encode(&mut buf[offset..])?;
            offset += self.track_id.encode(&mut buf[offset..])?;
            offset += [0u8; 4].encode(&mut buf[offset..])?;
            offset += (self.duration as u32).encode(&mut buf[offset..])?;
        }
        offset += [0u8; 4 * 2].encode(&mut buf[offset..])?;
        offset += self.layer.encode(&mut buf[offset..])?;
        offset += self.alternate_group.encode(&mut buf[offset..])?;
        offset += self.volume.encode(&mut buf[offset..])?;
        offset += [0u8; 2].encode(&mut buf[offset..])?;
        offset += self.matrix.encode(&mut buf[offset..])?;
        offset += self.width.encode(&mut buf[offset..])?;
        offset += self.height.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for TkhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for TkhdBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;

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
            this.creation_time =
                u64::decode_at(payload, &mut offset).map(Mp4FileTime::from_secs)?;
            this.modification_time =
                u64::decode_at(payload, &mut offset).map(Mp4FileTime::from_secs)?;
            this.track_id = u32::decode_at(payload, &mut offset)?;
            let _ = <[u8; 4]>::decode_at(payload, &mut offset)?;
            this.duration = u64::decode_at(payload, &mut offset)?;
        } else {
            this.creation_time =
                u32::decode_at(payload, &mut offset).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode_at(payload, &mut offset).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.track_id = u32::decode_at(payload, &mut offset)?;
            let _ = <[u8; 4]>::decode_at(payload, &mut offset)?;
            this.duration = u32::decode_at(payload, &mut offset).map(|v| v as u64)?;
        }

        let _ = <[u8; 4 * 2]>::decode_at(payload, &mut offset)?;
        this.layer = i16::decode_at(payload, &mut offset)?;
        this.alternate_group = i16::decode_at(payload, &mut offset)?;
        this.volume = FixedPointNumber::decode_at(payload, &mut offset)?;
        let _ = <[u8; 2]>::decode_at(payload, &mut offset)?;
        this.matrix = <[i32; 9]>::decode_at(payload, &mut offset)?;
        this.width = FixedPointNumber::decode_at(payload, &mut offset)?;
        this.height = FixedPointNumber::decode_at(payload, &mut offset)?;

        Ok((this, header.external_size() + payload.len()))
    }
}

impl BaseBox for TkhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        if let Some(b) = &self.elst_box {
            offset += b.encode(&mut buf[offset..])?;
        }
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for EdtsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for EdtsBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut elst_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                ElstBox::TYPE if elst_box.is_none() => {
                    elst_box = Some(ElstBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                elst_box,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for EdtsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;

        let version = self.full_box_version();
        offset += (self.entries.len() as u32).encode(&mut buf[offset..])?;
        for entry in &self.entries {
            if version == 1 {
                offset += entry.edit_duration.encode(&mut buf[offset..])?;
                offset += entry.media_time.encode(&mut buf[offset..])?;
            } else {
                offset += (entry.edit_duration as u32).encode(&mut buf[offset..])?;
                offset += (entry.media_time as i32).encode(&mut buf[offset..])?;
            }
            offset += entry.media_rate.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for ElstBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for ElstBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;

        let mut entries = Vec::new();
        let count = u32::decode_at(payload, &mut offset)?;
        for _ in 0..count {
            let edit_duration;
            let media_time;
            if full_header.version == 1 {
                edit_duration = u64::decode_at(payload, &mut offset)?;
                media_time = i64::decode_at(payload, &mut offset)?;
            } else {
                edit_duration = u32::decode_at(payload, &mut offset)? as u64;
                media_time = i32::decode_at(payload, &mut offset)? as i64;
            }
            let media_rate = FixedPointNumber::decode_at(payload, &mut offset)?;
            entries.push(ElstEntry {
                edit_duration,
                media_time,
                media_rate,
            });
        }

        Ok((Self { entries }, header.external_size() + payload.len()))
    }
}

impl BaseBox for ElstBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
    }
}

impl FullBox for ElstBox {
    fn full_box_version(&self) -> u8 {
        let large = self.entries.iter().any(|x| {
            u32::try_from(x.edit_duration).is_err() || i32::try_from(x.media_time).is_err()
        });
        if large { 1 } else { 0 }
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.mdhd_box.encode(&mut buf[offset..])?;
        offset += self.hdlr_box.encode(&mut buf[offset..])?;
        offset += self.minf_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for MdiaBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for MdiaBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut mdhd_box = None;
        let mut hdlr_box = None;
        let mut minf_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                MdhdBox::TYPE if mdhd_box.is_none() => {
                    mdhd_box = Some(MdhdBox::decode_at(payload, &mut offset)?);
                }
                HdlrBox::TYPE if hdlr_box.is_none() => {
                    hdlr_box = Some(HdlrBox::decode_at(payload, &mut offset)?);
                }
                MinfBox::TYPE if minf_box.is_none() => {
                    minf_box = Some(MinfBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                mdhd_box: check_mandatory_box(mdhd_box, "mdhd", "mdia")?,
                hdlr_box: check_mandatory_box(hdlr_box, "hdlr", "mdia")?,
                minf_box: check_mandatory_box(minf_box, "minf", "mdia")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for MdiaBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.mdhd_box).map(as_box_object))
                .chain(core::iter::once(&self.hdlr_box).map(as_box_object))
                .chain(core::iter::once(&self.minf_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        if self.full_box_version() == 1 {
            offset += self.creation_time.as_secs().encode(&mut buf[offset..])?;
            offset += self
                .modification_time
                .as_secs()
                .encode(&mut buf[offset..])?;
            offset += self.timescale.encode(&mut buf[offset..])?;
            offset += self.duration.encode(&mut buf[offset..])?;
        } else {
            offset += (self.creation_time.as_secs() as u32).encode(&mut buf[offset..])?;
            offset += (self.modification_time.as_secs() as u32).encode(&mut buf[offset..])?;
            offset += self.timescale.encode(&mut buf[offset..])?;
            offset += (self.duration as u32).encode(&mut buf[offset..])?;
        }

        let mut language: u16 = 0;
        for l in &self.language {
            let Some(code) = l.checked_sub(0x60) else {
                return Err(Error2::invalid_input(format!(
                    "Invalid language code: {:?}",
                    self.language
                )));
            };
            language = (language << 5) | code as u16;
        }
        offset += language.encode(&mut buf[offset..])?;
        offset += [0u8; 2].encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for MdhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for MdhdBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;

        let mut this = Self {
            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
            timescale: NonZeroU32::MIN,
            duration: 0,
            language: Self::LANGUAGE_UNDEFINED,
        };

        if full_header.version == 1 {
            this.creation_time =
                u64::decode_at(payload, &mut offset).map(Mp4FileTime::from_secs)?;
            this.modification_time =
                u64::decode_at(payload, &mut offset).map(Mp4FileTime::from_secs)?;
            this.timescale = NonZeroU32::decode_at(payload, &mut offset)?;
            this.duration = u64::decode_at(payload, &mut offset)?;
        } else {
            this.creation_time =
                u32::decode_at(payload, &mut offset).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.modification_time =
                u32::decode_at(payload, &mut offset).map(|v| Mp4FileTime::from_secs(v as u64))?;
            this.timescale = NonZeroU32::decode_at(payload, &mut offset)?;
            this.duration = u32::decode_at(payload, &mut offset).map(|v| v as u64)?;
        }

        let language = u16::decode_at(payload, &mut offset)?;
        this.language = [
            ((language >> 10) & 0b11111) as u8 + 0x60,
            ((language >> 5) & 0b11111) as u8 + 0x60,
            (language & 0b11111) as u8 + 0x60,
        ];

        let _ = <[u8; 2]>::decode_at(payload, &mut offset)?;

        Ok((this, header.external_size() + payload.len()))
    }
}

impl BaseBox for MdhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += [0u8; 4].encode(&mut buf[offset..])?;
        offset += self.handler_type.encode(&mut buf[offset..])?;
        offset += [0u8; 4 * 3].encode(&mut buf[offset..])?;
        offset += self.name.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for HdlrBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for HdlrBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let _ = <[u8; 4]>::decode_at(payload, &mut offset)?;
        let handler_type = <[u8; 4]>::decode_at(payload, &mut offset)?;
        let _ = <[u8; 4 * 3]>::decode_at(payload, &mut offset)?;
        let name = payload[offset..].to_vec();

        Ok((
            Self { handler_type, name },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for HdlrBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        match &self.smhd_or_vmhd_box {
            Either::A(b) => offset += b.encode(&mut buf[offset..])?,
            Either::B(b) => offset += b.encode(&mut buf[offset..])?,
        }
        offset += self.dinf_box.encode(&mut buf[offset..])?;
        offset += self.stbl_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for MinfBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for MinfBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut smhd_box = None;
        let mut vmhd_box = None;
        let mut dinf_box = None;
        let mut stbl_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                SmhdBox::TYPE if smhd_box.is_none() => {
                    smhd_box = Some(SmhdBox::decode_at(payload, &mut offset)?);
                }
                VmhdBox::TYPE if vmhd_box.is_none() => {
                    vmhd_box = Some(VmhdBox::decode_at(payload, &mut offset)?);
                }
                DinfBox::TYPE if dinf_box.is_none() => {
                    dinf_box = Some(DinfBox::decode_at(payload, &mut offset)?);
                }
                StblBox::TYPE if stbl_box.is_none() => {
                    stbl_box = Some(StblBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                smhd_or_vmhd_box: {
                    let smhd = smhd_box.map(Either::A);
                    let vmhd = vmhd_box.map(Either::B);
                    check_mandatory_box(smhd.or(vmhd), "smhd' or 'vmhd", "box")?
                },
                dinf_box: check_mandatory_box(dinf_box, "dinf", "minf")?,
                stbl_box: check_mandatory_box(stbl_box, "stbl", "minf")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for MinfBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.smhd_or_vmhd_box).map(as_box_object))
                .chain(core::iter::once(&self.dinf_box).map(as_box_object))
                .chain(core::iter::once(&self.stbl_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
        let _full_header = FullBoxHeader::decode(&mut reader)?;
        let balance = FixedPointNumber::decode(&mut reader)?;
        let _ = <[u8; 2]>::decode(reader)?;
        Ok(Self { balance })
    }
}

impl Encode for SmhdBox {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += self.balance.encode(&mut buf[offset..])?;
        offset += [0u8; 2].encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for SmhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for SmhdBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let balance = FixedPointNumber::decode_at(payload, &mut offset)?;
        let _ = <[u8; 2]>::decode_at(payload, &mut offset)?;

        Ok((Self { balance }, header.external_size() + payload.len()))
    }
}

impl BaseBox for SmhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += self.graphicsmode.encode(&mut buf[offset..])?;
        offset += self.opcolor.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for VmhdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for VmhdBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        if full_header.flags.get() != 1 {
            return Err(Error2::invalid_data(format!(
                "Unexpected FullBox header flags of 'vmhd' box: {}",
                full_header.flags.get()
            )));
        }

        let graphicsmode = u16::decode_at(payload, &mut offset)?;
        let opcolor = <[u16; 3]>::decode_at(payload, &mut offset)?;

        Ok((
            Self {
                graphicsmode,
                opcolor,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for VmhdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.dref_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for DinfBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for DinfBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut dref_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                DrefBox::TYPE if dref_box.is_none() => {
                    dref_box = Some(DrefBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                dref_box: check_mandatory_box(dref_box, "dref", "dinf")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for DinfBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.dref_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        let entry_count = (self.url_box.is_some() as usize + self.unknown_boxes.len()) as u32;
        offset += entry_count.encode(&mut buf[offset..])?;
        if let Some(b) = &self.url_box {
            offset += b.encode(&mut buf[offset..])?;
        }
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for DrefBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for DrefBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let entry_count = u32::decode_at(payload, &mut offset)?;

        let mut url_box = None;
        let mut unknown_boxes = Vec::new();

        for _ in 0..entry_count {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                UrlBox::TYPE if url_box.is_none() => {
                    url_box = Some(UrlBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                url_box,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for DrefBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        if let Some(l) = &self.location {
            offset += l.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for UrlBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for UrlBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let location = if full_header.flags.is_set(0) {
            None
        } else {
            Some(Utf8String::decode_at(payload, &mut offset)?)
        };

        Ok((Self { location }, header.external_size() + payload.len()))
    }
}

impl BaseBox for UrlBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.stsd_box.encode(&mut buf[offset..])?;
        offset += self.stts_box.encode(&mut buf[offset..])?;
        offset += self.stsc_box.encode(&mut buf[offset..])?;
        offset += self.stsz_box.encode(&mut buf[offset..])?;
        match &self.stco_or_co64_box {
            Either::A(b) => offset += b.encode(&mut buf[offset..])?,
            Either::B(b) => offset += b.encode(&mut buf[offset..])?,
        }
        if let Some(b) = &self.stss_box {
            offset += b.encode(&mut buf[offset..])?;
        }
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for StblBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for StblBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let mut stsd_box = None;
        let mut stts_box = None;
        let mut stsc_box = None;
        let mut stsz_box = None;
        let mut stco_box = None;
        let mut co64_box = None;
        let mut stss_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                StsdBox::TYPE if stsd_box.is_none() => {
                    stsd_box = Some(StsdBox::decode_at(payload, &mut offset)?);
                }
                SttsBox::TYPE if stts_box.is_none() => {
                    stts_box = Some(SttsBox::decode_at(payload, &mut offset)?);
                }
                StscBox::TYPE if stsc_box.is_none() => {
                    stsc_box = Some(StscBox::decode_at(payload, &mut offset)?);
                }
                StszBox::TYPE if stsz_box.is_none() => {
                    stsz_box = Some(StszBox::decode_at(payload, &mut offset)?);
                }
                StcoBox::TYPE if stco_box.is_none() => {
                    stco_box = Some(StcoBox::decode_at(payload, &mut offset)?);
                }
                Co64Box::TYPE if co64_box.is_none() => {
                    co64_box = Some(Co64Box::decode_at(payload, &mut offset)?);
                }
                StssBox::TYPE if stss_box.is_none() => {
                    stss_box = Some(StssBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                stsd_box: check_mandatory_box(stsd_box, "stsd", "stbl")?,
                stts_box: check_mandatory_box(stts_box, "stts", "stbl")?,
                stsc_box: check_mandatory_box(stsc_box, "stsc", "stbl")?,
                stsz_box: check_mandatory_box(stsz_box, "stsz", "stbl")?,
                stco_or_co64_box: check_mandatory_box(
                    stco_box.map(Either::A).or(co64_box.map(Either::B)),
                    "stco' or 'co64",
                    "stbl",
                )?,
                stss_box,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for StblBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.stsd_box).map(as_box_object))
                .chain(core::iter::once(&self.stts_box).map(as_box_object))
                .chain(core::iter::once(&self.stsc_box).map(as_box_object))
                .chain(core::iter::once(&self.stsz_box).map(as_box_object))
                .chain(core::iter::once(&self.stco_or_co64_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        let entry_count = (self.entries.len()) as u32;
        offset += entry_count.encode(&mut buf[offset..])?;
        for b in &self.entries {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for StsdBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for StsdBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let entry_count = u32::decode_at(payload, &mut offset)?;

        let mut entries = Vec::new();
        for _ in 0..entry_count {
            entries.push(SampleEntry::decode_at(payload, &mut offset)?);
        }

        Ok((Self { entries }, header.external_size() + payload.len()))
    }
}

impl BaseBox for StsdBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
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
    Mp4a(Mp4aBox),
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
            Self::Mp4a(b) => b,
            Self::Unknown(b) => b,
        }
    }
}

impl Encode for SampleEntry {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        match self {
            Self::Avc1(b) => b.encode(buf),
            Self::Hev1(b) => b.encode(buf),
            Self::Vp08(b) => b.encode(buf),
            Self::Vp09(b) => b.encode(buf),
            Self::Av01(b) => b.encode(buf),
            Self::Opus(b) => b.encode(buf),
            Self::Mp4a(b) => b.encode(buf),
            Self::Unknown(b) => b.encode(buf),
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
            Mp4aBox::TYPE => Decode::decode(&mut reader).map(Self::Mp4a),
            _ => Decode::decode(&mut reader).map(Self::Unknown),
        }
    }
}

impl Decode2 for SampleEntry {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, _) = BoxHeader::decode2(buf)?;
        match header.box_type {
            Avc1Box::TYPE => Avc1Box::decode2(buf).map(|(b, n)| (Self::Avc1(b), n)),
            Hev1Box::TYPE => Hev1Box::decode2(buf).map(|(b, n)| (Self::Hev1(b), n)),
            Vp08Box::TYPE => Vp08Box::decode2(buf).map(|(b, n)| (Self::Vp08(b), n)),
            Vp09Box::TYPE => Vp09Box::decode2(buf).map(|(b, n)| (Self::Vp09(b), n)),
            Av01Box::TYPE => Av01Box::decode2(buf).map(|(b, n)| (Self::Av01(b), n)),
            OpusBox::TYPE => OpusBox::decode2(buf).map(|(b, n)| (Self::Opus(b), n)),
            Mp4aBox::TYPE => Mp4aBox::decode2(buf).map(|(b, n)| (Self::Mp4a(b), n)),
            _ => UnknownBox::decode2(buf).map(|(b, n)| (Self::Unknown(b), n)),
        }
    }
}

impl BaseBox for SampleEntry {
    fn box_type(&self) -> BoxType {
        self.inner_box().box_type()
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += [0u8; 6].encode(&mut buf[offset..])?;
        offset += self.data_reference_index.encode(&mut buf[offset..])?;
        offset += [0u8; 2 + 2 + 4 * 3].encode(&mut buf[offset..])?;
        offset += self.width.encode(&mut buf[offset..])?;
        offset += self.height.encode(&mut buf[offset..])?;
        offset += self.horizresolution.encode(&mut buf[offset..])?;
        offset += self.vertresolution.encode(&mut buf[offset..])?;
        offset += [0u8; 4].encode(&mut buf[offset..])?;
        offset += self.frame_count.encode(&mut buf[offset..])?;
        offset += self.compressorname.encode(&mut buf[offset..])?;
        offset += self.depth.encode(&mut buf[offset..])?;
        offset += (-1i16).encode(&mut buf[offset..])?;
        Ok(offset)
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

impl Decode2 for VisualSampleEntryFields {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let mut offset = 0;
        let _ = <[u8; 6]>::decode_at(buf, &mut offset)?;
        let data_reference_index = NonZeroU16::decode_at(buf, &mut offset)?;
        let _ = <[u8; 2 + 2 + 4 * 3]>::decode_at(buf, &mut offset)?;
        let width = u16::decode_at(buf, &mut offset)?;
        let height = u16::decode_at(buf, &mut offset)?;
        let horizresolution = FixedPointNumber::decode_at(buf, &mut offset)?;
        let vertresolution = FixedPointNumber::decode_at(buf, &mut offset)?;
        let _ = <[u8; 4]>::decode_at(buf, &mut offset)?;
        let frame_count = u16::decode_at(buf, &mut offset)?;
        let compressorname = <[u8; 32]>::decode_at(buf, &mut offset)?;
        let depth = u16::decode_at(buf, &mut offset)?;
        let _ = <[u8; 2]>::decode_at(buf, &mut offset)?;
        Ok((
            Self {
                data_reference_index,
                width,
                height,
                horizresolution,
                vertresolution,
                frame_count,
                compressorname,
                depth,
            },
            offset,
        ))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.visual.encode(&mut buf[offset..])?;
        offset += self.avcc_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Avc1Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Avc1Box {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let visual = VisualSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut avcc_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                AvccBox::TYPE if avcc_box.is_none() => {
                    avcc_box = Some(AvccBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                visual,
                avcc_box: check_mandatory_box(avcc_box, "avcc", "avc1")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Avc1Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.avcc_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;

        offset += Self::CONFIGURATION_VERSION.encode(&mut buf[offset..])?;
        offset += self.avc_profile_indication.encode(&mut buf[offset..])?;
        offset += self.profile_compatibility.encode(&mut buf[offset..])?;
        offset += self.avc_level_indication.encode(&mut buf[offset..])?;
        offset += (0b1111_1100 | self.length_size_minus_one.get()).encode(&mut buf[offset..])?;

        let sps_count = u8::try_from(self.sps_list.len())
            .map_err(|_| Error2::invalid_input("Too many SPSs"))?;
        offset += (0b1110_0000 | sps_count).encode(&mut buf[offset..])?;
        for sps in &self.sps_list {
            let size =
                u16::try_from(sps.len()).map_err(|_| Error2::invalid_input("Too long SPS"))?;
            offset += size.encode(&mut buf[offset..])?;
            offset += sps.encode(&mut buf[offset..])?;
        }

        let pps_count = u8::try_from(self.pps_list.len())
            .map_err(|_| Error2::invalid_input("Too many PPSs"))?;
        offset += pps_count.encode(&mut buf[offset..])?;
        for pps in &self.pps_list {
            let size =
                u16::try_from(pps.len()).map_err(|_| Error2::invalid_input("Too long PPS"))?;
            offset += size.encode(&mut buf[offset..])?;
            offset += pps.encode(&mut buf[offset..])?;
        }

        if !matches!(self.avc_profile_indication, 66 | 77 | 88) {
            let chroma_format = self.chroma_format.ok_or_else(|| {
                Error2::invalid_input("Missing 'chroma_format' field in 'avcC' box")
            })?;
            let bit_depth_luma_minus8 = self.bit_depth_luma_minus8.ok_or_else(|| {
                Error2::invalid_input("Missing 'bit_depth_luma_minus8' field in 'avcC' box")
            })?;
            let bit_depth_chroma_minus8 = self.bit_depth_chroma_minus8.ok_or_else(|| {
                Error2::invalid_input("Missing 'bit_depth_chroma_minus8' field in 'avcC' box")
            })?;
            offset += (0b1111_1100 | chroma_format.get()).encode(&mut buf[offset..])?;
            offset += (0b1111_1000 | bit_depth_luma_minus8.get()).encode(&mut buf[offset..])?;
            offset += (0b1111_1000 | bit_depth_chroma_minus8.get()).encode(&mut buf[offset..])?;

            let sps_ext_count = u8::try_from(self.sps_ext_list.len())
                .map_err(|_| Error2::invalid_input("Too many SPS EXTs"))?;
            offset += sps_ext_count.encode(&mut buf[offset..])?;
            for sps_ext in &self.sps_ext_list {
                let size = u16::try_from(sps_ext.len())
                    .map_err(|_| Error2::invalid_input("Too long SPS EXT"))?;
                offset += size.encode(&mut buf[offset..])?;
                offset += sps_ext.encode(&mut buf[offset..])?;
            }
        }

        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for AvccBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for AvccBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let configuration_version = u8::decode_at(payload, &mut offset)?;
        if configuration_version != Self::CONFIGURATION_VERSION {
            return Err(Error2::invalid_data(format!(
                "Unsupported avcC configuration version: {configuration_version}"
            )));
        }

        let avc_profile_indication = u8::decode_at(payload, &mut offset)?;
        let profile_compatibility = u8::decode_at(payload, &mut offset)?;
        let avc_level_indication = u8::decode_at(payload, &mut offset)?;
        let length_size_minus_one = Uint::from_bits(u8::decode_at(payload, &mut offset)?);

        let sps_count =
            Uint::<u8, 5>::from_bits(u8::decode_at(payload, &mut offset)?).get() as usize;
        let mut sps_list = Vec::with_capacity(sps_count);
        for _ in 0..sps_count {
            let size = u16::decode_at(payload, &mut offset)? as usize;
            let sps = payload[offset..offset + size].to_vec();
            offset += size;
            sps_list.push(sps);
        }

        let pps_count = u8::decode_at(payload, &mut offset)? as usize;
        let mut pps_list = Vec::with_capacity(pps_count);
        for _ in 0..pps_count {
            let size = u16::decode_at(payload, &mut offset)? as usize;
            let pps = payload[offset..offset + size].to_vec();
            offset += size;
            pps_list.push(pps);
        }

        let mut chroma_format = None;
        let mut bit_depth_luma_minus8 = None;
        let mut bit_depth_chroma_minus8 = None;
        let mut sps_ext_list = Vec::new();
        if !matches!(avc_profile_indication, 66 | 77 | 88) {
            chroma_format = Some(Uint::from_bits(u8::decode_at(payload, &mut offset)?));
            bit_depth_luma_minus8 = Some(Uint::from_bits(u8::decode_at(payload, &mut offset)?));
            bit_depth_chroma_minus8 = Some(Uint::from_bits(u8::decode_at(payload, &mut offset)?));

            let sps_ext_count = u8::decode_at(payload, &mut offset)? as usize;
            for _ in 0..sps_ext_count {
                let size = u16::decode_at(payload, &mut offset)? as usize;
                let sps_ext = payload[offset..offset + size].to_vec();
                offset += size;
                sps_ext_list.push(sps_ext);
            }
        }

        Ok((
            Self {
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
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for AvccBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.visual.encode(&mut buf[offset..])?;
        offset += self.hvcc_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Hev1Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Hev1Box {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let visual = VisualSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut hvcc_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                HvccBox::TYPE if hvcc_box.is_none() => {
                    hvcc_box = Some(HvccBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                visual,
                hvcc_box: check_mandatory_box(hvcc_box, "hvcc", "hev1")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Hev1Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.hvcc_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += Self::CONFIGURATION_VERSION.encode(&mut buf[offset..])?;
        offset += (self.general_profile_space.to_bits()
            | self.general_tier_flag.to_bits()
            | self.general_profile_idc.to_bits())
        .encode(&mut buf[offset..])?;
        offset += self
            .general_profile_compatibility_flags
            .encode(&mut buf[offset..])?;
        offset += self.general_constraint_indicator_flags.get().to_be_bytes()[2..]
            .encode(&mut buf[offset..])?;
        offset += self.general_level_idc.encode(&mut buf[offset..])?;
        offset += (0b1111_0000_0000_0000 | self.min_spatial_segmentation_idc.to_bits())
            .encode(&mut buf[offset..])?;
        offset += (0b1111_1100 | self.parallelism_type.to_bits()).encode(&mut buf[offset..])?;
        offset += (0b1111_1100 | self.chroma_format_idc.to_bits()).encode(&mut buf[offset..])?;
        offset +=
            (0b1111_1000 | self.bit_depth_luma_minus8.to_bits()).encode(&mut buf[offset..])?;
        offset +=
            (0b1111_1000 | self.bit_depth_chroma_minus8.to_bits()).encode(&mut buf[offset..])?;
        offset += self.avg_frame_rate.encode(&mut buf[offset..])?;
        offset += (self.constant_frame_rate.to_bits()
            | self.num_temporal_layers.to_bits()
            | self.temporal_id_nested.to_bits()
            | self.length_size_minus_one.to_bits())
        .encode(&mut buf[offset..])?;
        offset += u8::try_from(self.nalu_arrays.len())
            .map_err(|_| {
                Error2::invalid_input(format!("Too many NALU arrays: {}", self.nalu_arrays.len()))
            })?
            .encode(&mut buf[offset..])?;
        for nalu_array in &self.nalu_arrays {
            offset += (nalu_array.array_completeness.to_bits()
                | nalu_array.nal_unit_type.to_bits())
            .encode(&mut buf[offset..])?;
            offset += u16::try_from(nalu_array.nalus.len())
                .map_err(|_| {
                    Error2::invalid_input(format!("Too many NALUs: {}", nalu_array.nalus.len()))
                })?
                .encode(&mut buf[offset..])?;
            for nalu in &nalu_array.nalus {
                offset += u16::try_from(nalu.len())
                    .map_err(|_| Error2::invalid_input(format!("Too large NALU: {}", nalu.len())))?
                    .encode(&mut buf[offset..])?;
                offset += nalu.encode(&mut buf[offset..])?;
            }
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for HvccBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for HvccBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let configuration_version = u8::decode_at(payload, &mut offset)?;
        if configuration_version != Self::CONFIGURATION_VERSION {
            return Err(Error2::invalid_data(format!(
                "Unsupported hvcC version: {configuration_version}"
            )));
        }

        let b = u8::decode_at(payload, &mut offset)?;
        let general_profile_space = Uint::from_bits(b);
        let general_tier_flag = Uint::from_bits(b);
        let general_profile_idc = Uint::from_bits(b);

        let general_profile_compatibility_flags = u32::decode_at(payload, &mut offset)?;

        let mut buf_constraint = [0; 8];
        buf_constraint[2..].copy_from_slice(&payload[offset..offset + 6]);
        offset += 6;
        let general_constraint_indicator_flags =
            Uint::from_bits(u64::from_be_bytes(buf_constraint));

        let general_level_idc = u8::decode_at(payload, &mut offset)?;
        let min_spatial_segmentation_idc = Uint::from_bits(u16::decode_at(payload, &mut offset)?);
        let parallelism_type = Uint::from_bits(u8::decode_at(payload, &mut offset)?);
        let chroma_format_idc = Uint::from_bits(u8::decode_at(payload, &mut offset)?);
        let bit_depth_luma_minus8 = Uint::from_bits(u8::decode_at(payload, &mut offset)?);
        let bit_depth_chroma_minus8 = Uint::from_bits(u8::decode_at(payload, &mut offset)?);
        let avg_frame_rate = u16::decode_at(payload, &mut offset)?;

        let b = u8::decode_at(payload, &mut offset)?;
        let constant_frame_rate = Uint::from_bits(b);
        let num_temporal_layers = Uint::from_bits(b);
        let temporal_id_nested = Uint::from_bits(b);
        let length_size_minus_one = Uint::from_bits(b);

        let num_of_arrays = u8::decode_at(payload, &mut offset)?;
        let mut nalu_arrays = Vec::new();
        for _ in 0..num_of_arrays {
            let b = u8::decode_at(payload, &mut offset)?;
            let array_completeness = Uint::from_bits(b);
            let nal_unit_type = Uint::from_bits(b);

            let num_nalus = u16::decode_at(payload, &mut offset)?;
            let mut nalus = Vec::new();
            for _ in 0..num_nalus {
                let nal_unit_length = u16::decode_at(payload, &mut offset)? as usize;
                let nal_unit = payload[offset..offset + nal_unit_length].to_vec();
                offset += nal_unit_length;
                nalus.push(nal_unit);
            }
            nalu_arrays.push(HvccNalUintArray {
                array_completeness,
                nal_unit_type,
                nalus,
            });
        }

        Ok((
            Self {
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
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for HvccBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.visual.encode(&mut buf[offset..])?;
        offset += self.vpcc_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Vp08Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Vp08Box {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let visual = VisualSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut vpcc_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                VpccBox::TYPE if vpcc_box.is_none() => {
                    vpcc_box = Some(VpccBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                visual,
                vpcc_box: check_mandatory_box(vpcc_box, "vpcC", "vp08")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Vp08Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.vpcc_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.visual.encode(&mut buf[offset..])?;
        offset += self.vpcc_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Vp09Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Vp09Box {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let visual = VisualSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut vpcc_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                VpccBox::TYPE if vpcc_box.is_none() => {
                    vpcc_box = Some(VpccBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                visual,
                vpcc_box: check_mandatory_box(vpcc_box, "vpcC", "vp09")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Vp09Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.vpcc_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += self.profile.encode(&mut buf[offset..])?;
        offset += self.level.encode(&mut buf[offset..])?;
        offset += (self.bit_depth.to_bits()
            | self.chroma_subsampling.to_bits()
            | self.video_full_range_flag.to_bits())
        .encode(&mut buf[offset..])?;
        offset += self.colour_primaries.encode(&mut buf[offset..])?;
        offset += self.transfer_characteristics.encode(&mut buf[offset..])?;
        offset += self.matrix_coefficients.encode(&mut buf[offset..])?;
        offset += (self.codec_initialization_data.len() as u16).encode(&mut buf[offset..])?;
        offset += self.codec_initialization_data.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for VpccBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for VpccBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let header_obj = FullBoxHeader::decode_at(payload, &mut offset)?;
        if header_obj.version != 1 {
            return Err(Error2::invalid_data(format!(
                "Unexpected full box header version: box=vpcC, version={}",
                header_obj.version
            )));
        }

        let profile = u8::decode_at(payload, &mut offset)?;
        let level = u8::decode_at(payload, &mut offset)?;

        let b = u8::decode_at(payload, &mut offset)?;
        let bit_depth = Uint::from_bits(b);
        let chroma_subsampling = Uint::from_bits(b);
        let video_full_range_flag = Uint::from_bits(b);
        let colour_primaries = u8::decode_at(payload, &mut offset)?;
        let transfer_characteristics = u8::decode_at(payload, &mut offset)?;
        let matrix_coefficients = u8::decode_at(payload, &mut offset)?;
        let codec_init_size = u16::decode_at(payload, &mut offset)? as usize;
        let codec_initialization_data = payload[offset..offset + codec_init_size].to_vec();

        Ok((
            Self {
                profile,
                level,
                bit_depth,
                chroma_subsampling,
                video_full_range_flag,
                colour_primaries,
                transfer_characteristics,
                matrix_coefficients,
                codec_initialization_data,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for VpccBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.visual.encode(&mut buf[offset..])?;
        offset += self.av1c_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Av01Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Av01Box {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let visual = VisualSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut av1c_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                Av1cBox::TYPE if av1c_box.is_none() => {
                    av1c_box = Some(Av1cBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                visual,
                av1c_box: check_mandatory_box(av1c_box, "av1c", "av01")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Av01Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.av1c_box).map(as_box_object))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += (Self::MARKER.to_bits() | Self::VERSION.to_bits()).encode(&mut buf[offset..])?;
        offset += (self.seq_profile.to_bits() | self.seq_level_idx_0.to_bits())
            .encode(&mut buf[offset..])?;
        offset += (self.seq_tier_0.to_bits()
            | self.high_bitdepth.to_bits()
            | self.twelve_bit.to_bits()
            | self.monochrome.to_bits()
            | self.chroma_subsampling_x.to_bits()
            | self.chroma_subsampling_y.to_bits()
            | self.chroma_sample_position.to_bits())
        .encode(&mut buf[offset..])?;
        if let Some(v) = self.initial_presentation_delay_minus_one {
            offset += (0b1_0000 | v.to_bits()).encode(&mut buf[offset..])?;
        } else {
            offset += 0u8.encode(&mut buf[offset..])?;
        }
        offset += self.config_obus.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Av1cBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Av1cBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let b = u8::decode_at(payload, &mut offset)?;
        let marker = Uint::from_bits(b);
        let version = Uint::from_bits(b);
        if marker != Self::MARKER {
            return Err(Error2::invalid_data("Unexpected av1C marker"));
        }
        if version != Self::VERSION {
            return Err(Error2::invalid_data(format!(
                "Unsupported av1C version: {}",
                version.get()
            )));
        }

        let b = u8::decode_at(payload, &mut offset)?;
        let seq_profile = Uint::from_bits(b);
        let seq_level_idx_0 = Uint::from_bits(b);

        let b = u8::decode_at(payload, &mut offset)?;
        let seq_tier_0 = Uint::from_bits(b);
        let high_bitdepth = Uint::from_bits(b);
        let twelve_bit = Uint::from_bits(b);
        let monochrome = Uint::from_bits(b);
        let chroma_subsampling_x = Uint::from_bits(b);
        let chroma_subsampling_y = Uint::from_bits(b);
        let chroma_sample_position = Uint::from_bits(b);

        let b = u8::decode_at(payload, &mut offset)?;
        let initial_presentation_delay_minus_one = if Uint::<u8, 1, 4>::from_bits(b).get() == 1 {
            Some(Uint::from_bits(b))
        } else {
            None
        };

        let config_obus = payload[offset..].to_vec();

        Ok((
            Self {
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
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Av1cBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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
            if let Some(last) = entries.last_mut()
                && last.sample_delta == sample_delta
            {
                last.sample_count += 1;
                continue;
            }
            entries.push(SttsEntry {
                sample_count: 1,
                sample_delta,
            });
        }
        Self { entries }
    }

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += (self.entries.len() as u32).encode(&mut buf[offset..])?;
        for entry in &self.entries {
            offset += entry.sample_count.encode(&mut buf[offset..])?;
            offset += entry.sample_delta.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for SttsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for SttsBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let count = u32::decode_at(payload, &mut offset)? as usize;

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(SttsEntry {
                sample_count: u32::decode_at(payload, &mut offset)?,
                sample_delta: u32::decode_at(payload, &mut offset)?,
            });
        }

        Ok((Self { entries }, header.external_size() + payload.len()))
    }
}

impl BaseBox for SttsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += (self.entries.len() as u32).encode(&mut buf[offset..])?;
        for entry in &self.entries {
            offset += entry.first_chunk.encode(&mut buf[offset..])?;
            offset += entry.sample_per_chunk.encode(&mut buf[offset..])?;
            offset += entry.sample_description_index.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for StscBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for StscBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let count = u32::decode_at(payload, &mut offset)?;

        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            entries.push(StscEntry {
                first_chunk: NonZeroU32::decode_at(payload, &mut offset)?,
                sample_per_chunk: u32::decode_at(payload, &mut offset)?,
                sample_description_index: NonZeroU32::decode_at(payload, &mut offset)?,
            });
        }

        Ok((Self { entries }, header.external_size() + payload.len()))
    }
}

impl BaseBox for StscBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        match self {
            StszBox::Fixed {
                sample_size,
                sample_count,
            } => {
                offset += sample_size.get().encode(&mut buf[offset..])?;
                offset += sample_count.encode(&mut buf[offset..])?;
            }
            StszBox::Variable { entry_sizes } => {
                offset += 0u32.encode(&mut buf[offset..])?;
                offset += (entry_sizes.len() as u32).encode(&mut buf[offset..])?;
                for size in entry_sizes {
                    offset += size.encode(&mut buf[offset..])?;
                }
            }
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for StszBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for StszBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let sample_size = u32::decode_at(payload, &mut offset)?;
        let sample_count = u32::decode_at(payload, &mut offset)?;

        let stsz_box = if let Some(sample_size) = NonZeroU32::new(sample_size) {
            Self::Fixed {
                sample_size,
                sample_count,
            }
        } else {
            let mut entry_sizes = Vec::with_capacity(sample_count as usize);
            for _ in 0..sample_count {
                entry_sizes.push(u32::decode_at(payload, &mut offset)?);
            }
            Self::Variable { entry_sizes }
        };

        Ok((stsz_box, header.external_size() + payload.len()))
    }
}

impl BaseBox for StszBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += (self.chunk_offsets.len() as u32).encode(&mut buf[offset..])?;
        for offset_val in &self.chunk_offsets {
            offset += offset_val.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for StcoBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for StcoBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let count = u32::decode_at(payload, &mut offset)?;

        let mut chunk_offsets = Vec::with_capacity(count as usize);
        for _ in 0..count {
            chunk_offsets.push(u32::decode_at(payload, &mut offset)?);
        }

        Ok((
            Self { chunk_offsets },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for StcoBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += (self.chunk_offsets.len() as u32).encode(&mut buf[offset..])?;
        for offset_val in &self.chunk_offsets {
            offset += offset_val.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Co64Box {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Co64Box {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let count = u32::decode_at(payload, &mut offset)?;

        let mut chunk_offsets = Vec::with_capacity(count as usize);
        for _ in 0..count {
            chunk_offsets.push(u64::decode_at(payload, &mut offset)?);
        }

        Ok((
            Self { chunk_offsets },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Co64Box {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += (self.sample_numbers.len() as u32).encode(&mut buf[offset..])?;
        for offset_val in &self.sample_numbers {
            offset += offset_val.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for StssBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for StssBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let count = u32::decode_at(payload, &mut offset)?;

        let mut sample_numbers = Vec::with_capacity(count as usize);
        for _ in 0..count {
            sample_numbers.push(NonZeroU32::decode_at(payload, &mut offset)?);
        }

        Ok((
            Self { sample_numbers },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for StssBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.audio.encode(&mut buf[offset..])?;
        offset += self.dops_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for OpusBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for OpusBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let audio = AudioSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut dops_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                DopsBox::TYPE if dops_box.is_none() => {
                    dops_box = Some(DopsBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                audio,
                dops_box: check_mandatory_box(dops_box, "dops", "Opus")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for OpusBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.dops_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// [ISO/IEC 14496-14] MP4AudioSampleEntry class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Mp4aBox {
    pub audio: AudioSampleEntryFields,
    pub esds_box: EsdsBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl Mp4aBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"mp4a");

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
        let audio = AudioSampleEntryFields::decode(&mut reader)?;
        let mut esds_box = None;
        let mut unknown_boxes = Vec::new();
        while reader.limit() > 0 {
            let (header, mut reader) = BoxHeader::peek(&mut reader)?;
            match header.box_type {
                EsdsBox::TYPE if esds_box.is_none() => {
                    esds_box = Some(EsdsBox::decode(&mut reader)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode(&mut reader)?);
                }
            }
        }
        let esds_box = esds_box.ok_or_else(|| Error::missing_box("esds", Self::TYPE))?;
        Ok(Self {
            audio,
            esds_box,
            unknown_boxes,
        })
    }
}

impl Encode for Mp4aBox {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += self.audio.encode(&mut buf[offset..])?;
        offset += self.esds_box.encode(&mut buf[offset..])?;
        for b in &self.unknown_boxes {
            offset += b.encode(&mut buf[offset..])?;
        }
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for Mp4aBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for Mp4aBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let audio = AudioSampleEntryFields::decode_at(payload, &mut offset)?;

        let mut esds_box = None;
        let mut unknown_boxes = Vec::new();

        while offset < payload.len() {
            let (child_header, _) = BoxHeader::decode2(&payload[offset..])?;
            match child_header.box_type {
                EsdsBox::TYPE if esds_box.is_none() => {
                    esds_box = Some(EsdsBox::decode_at(payload, &mut offset)?);
                }
                _ => {
                    unknown_boxes.push(UnknownBox::decode_at(payload, &mut offset)?);
                }
            }
        }

        Ok((
            Self {
                audio,
                esds_box: check_mandatory_box(esds_box, "esds", "mp4a")?,
                unknown_boxes,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for Mp4aBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(core::iter::once(&self.esds_box).map(as_box_object))
                .chain(self.unknown_boxes.iter().map(as_box_object)),
        )
    }
}

/// 音声系の [`SampleEntry`] に共通のフィールドをまとめた構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct AudioSampleEntryFields {
    pub data_reference_index: NonZeroU16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: FixedPointNumber<u16, u16>,
}

impl AudioSampleEntryFields {
    /// [`AudioSampleEntryFields::data_reference_index`] のデフォルト値
    pub const DEFAULT_DATA_REFERENCE_INDEX: NonZeroU16 = NonZeroU16::MIN;

    /// [`AudioSampleEntryFields::sample_size`] のデフォルト値 (16)
    pub const DEFAULT_SAMPLESIZE: u16 = 16;
}

impl Encode for AudioSampleEntryFields {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += [0u8; 6].encode(&mut buf[offset..])?;
        offset += self.data_reference_index.encode(&mut buf[offset..])?;
        offset += [0u8; 4 * 2].encode(&mut buf[offset..])?;
        offset += self.channelcount.encode(&mut buf[offset..])?;
        offset += self.samplesize.encode(&mut buf[offset..])?;
        offset += [0u8; 2].encode(&mut buf[offset..])?;
        offset += [0u8; 2].encode(&mut buf[offset..])?;
        offset += self.samplerate.encode(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl Decode for AudioSampleEntryFields {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let _ = <[u8; 6]>::decode(&mut reader)?;
        let data_reference_index = NonZeroU16::decode(&mut reader)?;
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

impl Decode2 for AudioSampleEntryFields {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let mut offset = 0;
        let _ = <[u8; 6]>::decode_at(buf, &mut offset)?;
        let data_reference_index = NonZeroU16::decode_at(buf, &mut offset)?;
        let _ = <[u8; 4 * 2]>::decode_at(buf, &mut offset)?;
        let channelcount = u16::decode_at(buf, &mut offset)?;
        let samplesize = u16::decode_at(buf, &mut offset)?;
        let _ = <[u8; 2]>::decode_at(buf, &mut offset)?;
        let _ = <[u8; 2]>::decode_at(buf, &mut offset)?;
        let samplerate = FixedPointNumber::decode_at(buf, &mut offset)?;
        Ok((
            Self {
                data_reference_index,
                channelcount,
                samplesize,
                samplerate,
            },
            offset,
        ))
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

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
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
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += Self::VERSION.encode(&mut buf[offset..])?;
        offset += self.output_channel_count.encode(&mut buf[offset..])?;
        offset += self.pre_skip.encode(&mut buf[offset..])?;
        offset += self.input_sample_rate.encode(&mut buf[offset..])?;
        offset += self.output_gain.encode(&mut buf[offset..])?;
        offset += 0u8.encode(&mut buf[offset..])?; // ChannelMappingFamily
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for DopsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for DopsBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let version = u8::decode_at(payload, &mut offset)?;
        if version != Self::VERSION {
            return Err(Error2::invalid_data(format!(
                "Unsupported dOps version: {version}"
            )));
        }

        let output_channel_count = u8::decode_at(payload, &mut offset)?;
        let pre_skip = u16::decode_at(payload, &mut offset)?;
        let input_sample_rate = u32::decode_at(payload, &mut offset)?;
        let output_gain = i16::decode_at(payload, &mut offset)?;
        let channel_mapping_family = u8::decode_at(payload, &mut offset)?;
        if channel_mapping_family != 0 {
            return Err(Error2::unsupported(
                "`ChannelMappingFamily != 0` in 'dOps' box is not supported",
            ));
        }

        Ok((
            Self {
                output_channel_count,
                pre_skip,
                input_sample_rate,
                output_gain,
            },
            header.external_size() + payload.len(),
        ))
    }
}

impl BaseBox for DopsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
    }
}

/// [ISO/IEC 14496-14] ESDBox class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct EsdsBox {
    pub es: EsDescriptor,
}

impl EsdsBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"esds");

    fn decode_payload<R: Read>(mut reader: &mut Take<R>) -> Result<Self> {
        let _ = FullBoxHeader::decode(&mut reader)?;
        let es = EsDescriptor::decode(&mut reader)?;
        Ok(Self { es })
    }
}

impl Encode for EsdsBox {
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += self.es.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for EsdsBox {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        header.box_type.expect(Self::TYPE)?;
        header.with_box_payload_reader(reader, Self::decode_payload)
    }
}

impl Decode2 for EsdsBox {
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)> {
        let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
        header.box_type.expect2(Self::TYPE)?;

        let mut offset = 0;
        let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
        let es = EsDescriptor::decode_at(payload, &mut offset)?;

        Ok((Self { es }, header.external_size() + payload.len()))
    }
}

impl BaseBox for EsdsBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(core::iter::empty())
    }
}

impl FullBox for EsdsBox {
    fn full_box_version(&self) -> u8 {
        0
    }

    fn full_box_flags(&self) -> FullBoxFlags {
        FullBoxFlags::new(0)
    }
}

#[track_caller]
fn check_mandatory_box<T>(maybe_box: Option<T>, expected: &str, parent: &str) -> Result2<T> {
    maybe_box.ok_or_else(|| {
        Error2::invalid_data(format!(
            "Missing mandatory '{expected}' box in '{parent}' box"
        ))
    })
}
