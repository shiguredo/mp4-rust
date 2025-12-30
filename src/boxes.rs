//! ボックス群
use alloc::{boxed::Box, format, vec::Vec};
use core::num::NonZeroU32;

use crate::{
    BaseBox, BoxHeader, BoxSize, BoxType, Decode, Either, Encode, Error, FixedPointNumber, FullBox,
    FullBoxFlags, FullBoxHeader, Mp4FileTime, Result, Utf8String, basic_types::as_box_object,
    descriptors::EsDescriptor,
};

pub use crate::boxes_sample_entry::{
    AudioSampleEntryFields, Av01Box, Av1cBox, Avc1Box, AvccBox, DflaBox, DopsBox, FlacBox,
    FlacMetadataBlock, Hev1Box, Hvc1Box, HvccBox, HvccNalUintArray, Mp4aBox, OpusBox, SampleEntry,
    VisualSampleEntryFields, Vp08Box, Vp09Box, VpccBox,
};

pub(crate) fn with_box_type<F, T>(ty: BoxType, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    f().map_err(|mut e| {
        if e.box_type.is_none() {
            // エラー発生時には、エラーの原因となった（最初の）ボックスの種別の情報をセットする
            e.box_type = Some(ty);
        }
        e
    })
}

#[track_caller]
pub(crate) fn check_mandatory_box<T>(
    maybe_box: Option<T>,
    expected: &str,
    parent: &str,
) -> Result<T> {
    // [NOTE]
    // ok_or_else() でも同じことができるが `Error::invalid_data()` をクロージャーで囲ってしまうと、
    // `check_mandatory_box()` 自体の `track_caller` 指定の意味がなくなってしまうので、あえて if-else で実装している
    if let Some(b) = maybe_box {
        Ok(b)
    } else {
        Err(Error::invalid_data(format!(
            "Missing mandatory '{expected}' box in '{parent}' box"
        )))
    }
}

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
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let mut offset = BoxHeader::new(self.box_type, self.box_size).encode(buf)?;
        offset += self.payload.encode(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl Decode for UnknownBox {
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
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
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        self.0.encode(buf)
    }
}

impl Decode for Brand {
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        let (bytes, offset) = <[u8; 4]>::decode(buf)?;
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
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
        })
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
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        match self {
            RootBox::Free(b) => b.encode(buf),
            RootBox::Mdat(b) => b.encode(buf),
            RootBox::Moov(b) => b.encode(buf),
            RootBox::Unknown(b) => b.encode(buf),
        }
    }
}

impl Decode for RootBox {
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        let (header, _header_size) = BoxHeader::decode(buf)?;
        match header.box_type {
            FreeBox::TYPE => FreeBox::decode(buf).map(|(b, n)| (RootBox::Free(b), n)),
            MdatBox::TYPE => MdatBox::decode(buf).map(|(b, n)| (RootBox::Mdat(b), n)),
            MoovBox::TYPE => MoovBox::decode(buf).map(|(b, n)| (RootBox::Moov(b), n)),
            _ => UnknownBox::decode(buf).map(|(b, n)| (RootBox::Unknown(b), n)),
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
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let box_size = BoxSize::with_payload_size(Self::TYPE, self.payload.len() as u64);
        let mut offset = BoxHeader::new(Self::TYPE, box_size).encode(buf)?;
        offset += self.payload.encode(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl Decode for FreeBox {
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            Ok((
                Self {
                    payload: payload.to_vec(),
                },
                header.external_size() + payload.len(),
            ))
        })
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
///
/// # NOTE
///
/// 可変長ペイロードを表現したい場合には、この構造体ではなく [`BoxHeader`] を直接使用する必要がある
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MdatBox {
    /// ペイロード
    pub payload: Vec<u8>,
}

impl MdatBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"mdat");
}

impl Encode for MdatBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let box_size = BoxSize::with_payload_size(Self::TYPE, self.payload.len() as u64);
        let mut offset = BoxHeader::new(Self::TYPE, box_size).encode(buf)?;
        offset += self.payload.encode(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl Decode for MdatBox {
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            Ok((
                Self {
                    payload: payload.to_vec(),
                },
                header.external_size() + payload.len(),
            ))
        })
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
}

impl Encode for MoovBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let mut mvhd_box = None;
            let mut trak_boxes = Vec::new();
            let mut unknown_boxes = Vec::new();

            while offset < payload.len() {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for MvhdBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
                this.creation_time = u32::decode_at(payload, &mut offset)
                    .map(|v| Mp4FileTime::from_secs(v as u64))?;
                this.modification_time = u32::decode_at(payload, &mut offset)
                    .map(|v| Mp4FileTime::from_secs(v as u64))?;
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
        })
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
}

impl Encode for TrakBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let mut tkhd_box = None;
            let mut edts_box = None;
            let mut mdia_box = None;
            let mut unknown_boxes = Vec::new();

            while offset < payload.len() {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for TkhdBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
                this.creation_time = u32::decode_at(payload, &mut offset)
                    .map(|v| Mp4FileTime::from_secs(v as u64))?;
                this.modification_time = u32::decode_at(payload, &mut offset)
                    .map(|v| Mp4FileTime::from_secs(v as u64))?;
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
        })
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
}

impl Encode for EdtsBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let mut elst_box = None;
            let mut unknown_boxes = Vec::new();

            while offset < payload.len() {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for ElstBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
        })
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
}

impl Encode for MdiaBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let mut mdhd_box = None;
            let mut hdlr_box = None;
            let mut minf_box = None;
            let mut unknown_boxes = Vec::new();

            while offset < payload.len() {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for MdhdBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
                return Err(Error::invalid_input(format!(
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
                this.creation_time = u32::decode_at(payload, &mut offset)
                    .map(|v| Mp4FileTime::from_secs(v as u64))?;
                this.modification_time = u32::decode_at(payload, &mut offset)
                    .map(|v| Mp4FileTime::from_secs(v as u64))?;
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
        })
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
}

impl Encode for HdlrBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
        })
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
    // 音声・映像トラック以外の場合は None になる
    pub smhd_or_vmhd_box: Option<Either<SmhdBox, VmhdBox>>,
    pub dinf_box: DinfBox,
    pub stbl_box: StblBox,
    pub unknown_boxes: Vec<UnknownBox>,
}

impl MinfBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"minf");
}

impl Encode for MinfBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        if let Some(smhd_or_vmhd_box) = &self.smhd_or_vmhd_box {
            match smhd_or_vmhd_box {
                Either::A(b) => offset += b.encode(&mut buf[offset..])?,
                Either::B(b) => offset += b.encode(&mut buf[offset..])?,
            }
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let mut smhd_box = None;
            let mut vmhd_box = None;
            let mut dinf_box = None;
            let mut stbl_box = None;
            let mut unknown_boxes = Vec::new();

            while offset < payload.len() {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
                    smhd_or_vmhd_box: smhd_box.map(Either::A).or(vmhd_box.map(Either::B)),
                    dinf_box: check_mandatory_box(dinf_box, "dinf", "minf")?,
                    stbl_box: check_mandatory_box(stbl_box, "stbl", "minf")?,
                    unknown_boxes,
                },
                header.external_size() + payload.len(),
            ))
        })
    }
}

impl BaseBox for MinfBox {
    fn box_type(&self) -> BoxType {
        Self::TYPE
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        Box::new(
            core::iter::empty()
                .chain(self.smhd_or_vmhd_box.iter().map(as_box_object))
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
}

impl Encode for SmhdBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let balance = FixedPointNumber::decode_at(payload, &mut offset)?;
            let _ = <[u8; 2]>::decode_at(payload, &mut offset)?;

            Ok((Self { balance }, header.external_size() + payload.len()))
        })
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
}

impl Encode for VmhdBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;

            // [NOTE]
            // ISO/IEC 14496-12 の仕様には「vmhd ボックスの flags は 1 になる」と記載があるが、
            // 実際には 0 となるファイルも存在するため、ここではそのチェックを行わないようにしている

            let graphicsmode = u16::decode_at(payload, &mut offset)?;
            let opcolor = <[u16; 3]>::decode_at(payload, &mut offset)?;

            Ok((
                Self {
                    graphicsmode,
                    opcolor,
                },
                header.external_size() + payload.len(),
            ))
        })
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
}

impl Encode for DinfBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let mut dref_box = None;
            let mut unknown_boxes = Vec::new();

            while offset < payload.len() {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for DrefBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let entry_count = u32::decode_at(payload, &mut offset)?;

            let mut url_box = None;
            let mut unknown_boxes = Vec::new();

            for _ in 0..entry_count {
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for UrlBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let location = if full_header.flags.is_set(0) {
                None
            } else {
                Some(Utf8String::decode_at(payload, &mut offset)?)
            };

            Ok((Self { location }, header.external_size() + payload.len()))
        })
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
}

impl Encode for StblBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
                let (child_header, _) = BoxHeader::decode(&payload[offset..])?;
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
        })
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
}

impl Encode for StsdBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let entry_count = u32::decode_at(payload, &mut offset)?;

            let mut entries = Vec::new();
            for _ in 0..entry_count {
                entries.push(SampleEntry::decode_at(payload, &mut offset)?);
            }

            Ok((Self { entries }, header.external_size() + payload.len()))
        })
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
}

impl Encode for SttsBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let count = u32::decode_at(payload, &mut offset)? as usize;

            let mut entries = Vec::new();
            for _ in 0..count {
                entries.push(SttsEntry {
                    sample_count: u32::decode_at(payload, &mut offset)?,
                    sample_delta: u32::decode_at(payload, &mut offset)?,
                });
            }

            Ok((Self { entries }, header.external_size() + payload.len()))
        })
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
}

impl Encode for StscBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let count = u32::decode_at(payload, &mut offset)?;

            let mut entries = Vec::new();
            for _ in 0..count {
                entries.push(StscEntry {
                    first_chunk: NonZeroU32::decode_at(payload, &mut offset)?,
                    sample_per_chunk: u32::decode_at(payload, &mut offset)?,
                    sample_description_index: NonZeroU32::decode_at(payload, &mut offset)?,
                });
            }

            Ok((Self { entries }, header.external_size() + payload.len()))
        })
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
}

impl Encode for StszBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

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
                let mut entry_sizes = Vec::new();
                for _ in 0..sample_count {
                    entry_sizes.push(u32::decode_at(payload, &mut offset)?);
                }
                Self::Variable { entry_sizes }
            };

            Ok((stsz_box, header.external_size() + payload.len()))
        })
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
}

impl Encode for StcoBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let count = u32::decode_at(payload, &mut offset)?;

            let mut chunk_offsets = Vec::new();
            for _ in 0..count {
                chunk_offsets.push(u32::decode_at(payload, &mut offset)?);
            }

            Ok((
                Self { chunk_offsets },
                header.external_size() + payload.len(),
            ))
        })
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
}

impl Encode for Co64Box {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let count = u32::decode_at(payload, &mut offset)?;

            let mut chunk_offsets = Vec::new();
            for _ in 0..count {
                chunk_offsets.push(u64::decode_at(payload, &mut offset)?);
            }

            Ok((
                Self { chunk_offsets },
                header.external_size() + payload.len(),
            ))
        })
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
}

impl Encode for StssBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
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
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let count = u32::decode_at(payload, &mut offset)?;

            let mut sample_numbers = Vec::new();
            for _ in 0..count {
                sample_numbers.push(NonZeroU32::decode_at(payload, &mut offset)?);
            }

            Ok((
                Self { sample_numbers },
                header.external_size() + payload.len(),
            ))
        })
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

/// [ISO/IEC 14496-14] ESDBox class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct EsdsBox {
    pub es: EsDescriptor,
}

impl EsdsBox {
    /// ボックス種別
    pub const TYPE: BoxType = BoxType::Normal(*b"esds");
}

impl Encode for EsdsBox {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let header = BoxHeader::new_variable_size(Self::TYPE);
        let mut offset = header.encode(buf)?;
        offset += FullBoxHeader::from_box(self).encode(&mut buf[offset..])?;
        offset += self.es.encode(&mut buf[offset..])?;
        header.finalize_box_size(&mut buf[..offset])?;
        Ok(offset)
    }
}

impl Decode for EsdsBox {
    fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        with_box_type(Self::TYPE, || {
            let (header, payload) = BoxHeader::decode_header_and_payload(buf)?;
            header.box_type.expect(Self::TYPE)?;

            let mut offset = 0;
            let _full_header = FullBoxHeader::decode_at(payload, &mut offset)?;
            let es = EsDescriptor::decode_at(payload, &mut offset)?;

            Ok((Self { es }, header.external_size() + payload.len()))
        })
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
