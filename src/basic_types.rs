use std::{
    io::{Read, Write},
    time::Duration,
};

use crate::{
    boxes::{FtypBox, RootBox},
    io::PeekReader,
    Decode, Encode, Error, Result,
};

// 単なる `Box` だと Rust の標準ライブラリのそれと名前が衝突するので変えておく
pub trait BaseBox {
    fn box_type(&self) -> BoxType;

    fn box_size(&self) -> BoxSize {
        BoxSize::with_payload_size(self.box_type(), self.box_payload_size())
    }

    fn box_payload_size(&self) -> u64;

    fn is_unknown_box(&self) -> bool;

    // TODO: remove
    fn actual_box(&self) -> &dyn BaseBox;

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>>;
}

pub trait FullBox: BaseBox {
    fn full_box_version(&self) -> u8;
    fn full_box_flags(&self) -> FullBoxFlags;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mp4File<B = RootBox> {
    pub ftyp_box: FtypBox,
    pub boxes: Vec<B>,
}

impl<B: BaseBox> Mp4File<B> {
    pub fn iter(&self) -> impl Iterator<Item = &dyn BaseBox> {
        std::iter::empty()
            .chain(std::iter::once(&self.ftyp_box).map(BaseBox::actual_box))
            .chain(self.boxes.iter().map(BaseBox::actual_box))
    }
}

impl<B: BaseBox + Decode> Decode for Mp4File<B> {
    fn decode<R: Read>(mut reader: &mut R) -> Result<Self> {
        let ftyp_box = FtypBox::decode(reader)?;

        let mut boxes = Vec::new();
        let mut buf = [0];
        while reader.read(&mut buf)? != 0 {
            let b = B::decode(&mut buf.chain(&mut reader))?;
            boxes.push(b);
        }
        Ok(Self { ftyp_box, boxes })
    }
}

impl<B: BaseBox + Encode> Encode for Mp4File<B> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.ftyp_box.encode(writer)?;

        for b in &self.boxes {
            b.encode(writer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxHeader {
    pub box_type: BoxType,
    pub box_size: BoxSize,
}

impl BoxHeader {
    pub const MAX_SIZE: usize = (4 + 8) + (4 + 16);

    pub fn from_box<B: BaseBox>(b: &B) -> Self {
        let box_type = b.box_type();
        let box_size = b.box_size();
        Self { box_type, box_size }
    }

    pub fn header_size(self) -> usize {
        self.box_type.external_size() + self.box_size.external_size()
    }

    pub fn with_box_payload_reader<T, R: Read, F>(self, reader: R, f: F) -> Result<T>
    where
        F: FnOnce(&mut std::io::Take<R>) -> Result<T>,
    {
        let mut reader = if self.box_size.get() == 0 {
            reader.take(u64::MAX)
        } else {
            let payload_size = self
                .box_size
                .get()
                .checked_sub(self.header_size() as u64)
                .ok_or_else(|| {
                    Error::invalid_data(&format!(
                        "Too small box size: actual={}, expected={} or more",
                        self.box_size.get(),
                        self.header_size()
                    ))
                })?;
            reader.take(payload_size)
        };

        let value = f(&mut reader)?;
        if reader.limit() != 0 {
            return Err(Error::invalid_data(&format!(
                "Unconsumed {} bytes at the end of the box {:?}",
                reader.limit(),
                self.box_type
            )));
        }
        Ok(value)
    }

    pub fn peek<R: Read>(reader: R) -> Result<(Self, impl Read)> {
        let mut reader = PeekReader::<_, { BoxHeader::MAX_SIZE }>::new(reader);
        let header = BoxHeader::decode(&mut reader)?;
        Ok((header, reader.into_reader()))
    }
}

impl Encode for BoxHeader {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        let large_size = self.box_size.get() > u32::MAX as u64;
        if large_size {
            1u32.encode(writer)?;
        } else {
            (self.box_size.get() as u32).encode(writer)?;
        }

        match self.box_type {
            BoxType::Normal(ty) => {
                writer.write_all(&ty)?;
            }
            BoxType::Uuid(ty) => {
                writer.write_all("uuid".as_bytes())?;
                writer.write_all(&ty)?;
            }
        }

        if large_size {
            self.box_size.get().encode(writer)?;
        }

        Ok(())
    }
}

impl Decode for BoxHeader {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut box_size = u32::decode(reader)? as u64;

        let mut box_type = [0; 4];
        reader.read_exact(&mut box_type)?;

        let box_type = if box_type == [b'u', b'u', b'i', b'd'] {
            let mut box_type = [0; 16];
            reader.read_exact(&mut box_type)?;
            BoxType::Uuid(box_type)
        } else {
            BoxType::Normal(box_type)
        };

        if box_size == 1 {
            box_size = u64::decode(reader)?;
        }
        let box_size = BoxSize::new(box_type, box_size).ok_or_else(|| {
            Error::invalid_data(&format!(
                "Too small box size: actual={}, expected={} or more",
                box_size,
                4 + box_type.external_size()
            ))
        })?;

        Ok(Self { box_type, box_size })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullBoxHeader {
    pub version: u8,
    pub flags: FullBoxFlags,
}

impl FullBoxHeader {
    pub fn from_box<B: FullBox>(b: &B) -> Self {
        Self {
            version: b.full_box_version(),
            flags: b.full_box_flags(),
        }
    }
}

impl Encode for FullBoxHeader {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.version.encode(writer)?;
        self.flags.encode(writer)?;
        Ok(())
    }
}

impl Decode for FullBoxHeader {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            version: Decode::decode(reader)?,
            flags: Decode::decode(reader)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullBoxFlags(u32);

impl FullBoxFlags {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (usize, bool)>,
    {
        let flags = iter.into_iter().filter(|x| x.1).map(|x| 1 << x.0).sum();
        Self(flags)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn is_set(self, i: usize) -> bool {
        (self.0 & (1 << i)) != 0
    }
}

impl Encode for FullBoxFlags {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.0.to_be_bytes()[1..])?;
        Ok(())
    }
}

impl Decode for FullBoxFlags {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf[1..])?;
        Ok(Self(u32::from_be_bytes(buf)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoxSize(u64);

impl BoxSize {
    pub const VARIABLE_SIZE: Self = Self(0);

    pub fn new(box_type: BoxType, box_size: u64) -> Option<Self> {
        if box_size == 0 {
            return Some(Self(0));
        }

        if box_size < 4 + box_type.external_size() as u64 {
            None
        } else {
            Some(Self(box_size))
        }
    }

    pub const fn with_payload_size(box_type: BoxType, payload_size: u64) -> Self {
        let mut size = 4 + box_type.external_size() as u64 + payload_size;
        if size > u32::MAX as u64 {
            size += 8;
        }
        Self(size)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn external_size(self) -> usize {
        if self.0 > u32::MAX as u64 {
            4 + 8
        } else {
            4
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoxType {
    Normal([u8; 4]),
    Uuid([u8; 16]),
}

impl BoxType {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            BoxType::Normal(ty) => &ty[..],
            BoxType::Uuid(ty) => &ty[..],
        }
    }

    pub const fn external_size(self) -> usize {
        if matches!(self, Self::Normal(_)) {
            4
        } else {
            4 + 16
        }
    }

    pub fn expect(self, expected: Self) -> Result<()> {
        if self == expected {
            Ok(())
        } else {
            Err(Error::invalid_data(&format!(
                "Expected box type {:?}, but got {:?}",
                expected, self
            )))
        }
    }
}

impl std::fmt::Debug for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoxType::Normal(ty) => {
                if let Ok(ty) = std::str::from_utf8(ty) {
                    f.debug_tuple("BoxType").field(&ty).finish()
                } else {
                    f.debug_tuple("BoxType").field(ty).finish()
                }
            }
            BoxType::Uuid(ty) => f.debug_tuple("BoxType").field(ty).finish(),
        }
    }
}

impl std::fmt::Display for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let BoxType::Normal(ty) = self {
            if let Ok(ty) = std::str::from_utf8(&ty[..]) {
                return write!(f, "{ty}");
            }
        }
        write!(f, "{:?}", self.as_bytes())
    }
}

/// 1904/1/1 からの経過秒数
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mp4FileTime(u64);

impl Mp4FileTime {
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    pub const fn as_secs(self) -> u64 {
        self.0
    }

    pub const fn from_unix_time(unix_time: Duration) -> Self {
        let delta = 2082844800; // 1904/1/1 から 1970/1/1 までの経過秒数
        let unix_time_secs = unix_time.as_secs();
        Self::from_secs(unix_time_secs + delta)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FixedPointNumber<I, F = I> {
    pub integer: I,
    pub fraction: F,
}

impl<I, F> FixedPointNumber<I, F> {
    pub const fn new(integer: I, fraction: F) -> Self {
        Self { integer, fraction }
    }
}

impl<I: Encode, F: Encode> Encode for FixedPointNumber<I, F> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.integer.encode(writer)?;
        self.fraction.encode(writer)?;
        Ok(())
    }
}

impl<I: Decode, F: Decode> Decode for FixedPointNumber<I, F> {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            integer: I::decode(reader)?,
            fraction: F::decode(reader)?,
        })
    }
}

// エンコード時には終端 null が付与される文字列
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Utf8String(String);

impl Utf8String {
    pub fn new(s: &str) -> Option<Self> {
        if s.as_bytes().contains(&0) {
            return None;
        }
        Some(Self(s.to_owned()))
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl Encode for Utf8String {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(self.0.as_bytes())?;
        writer.write_all(&[0])?;
        Ok(())
    }
}

impl Decode for Utf8String {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut bytes = Vec::new();
        loop {
            let b = u8::decode(reader)?;
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        let s = String::from_utf8(bytes).map_err(|e| {
            Error::invalid_data(&format!("Invalid UTF-8 string: {:?}", e.as_bytes()))
        })?;
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A: BaseBox, B: BaseBox> BaseBox for Either<A, B> {
    fn box_type(&self) -> BoxType {
        self.actual_box().box_type()
    }

    fn box_size(&self) -> BoxSize {
        self.actual_box().box_size()
    }

    fn box_payload_size(&self) -> u64 {
        self.actual_box().box_payload_size()
    }

    fn is_unknown_box(&self) -> bool {
        self.actual_box().is_unknown_box()
    }

    fn actual_box(&self) -> &dyn BaseBox {
        match self {
            Self::A(x) => x.actual_box(),
            Self::B(x) => x.actual_box(),
        }
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.actual_box().children()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uint<const BITS: u32>(u8);

impl<const BITS: u32> Uint<BITS> {
    pub const fn new(v: u8) -> Self {
        Self(v & (1 << BITS) - 1)
    }

    pub const fn checked_new(v: u8) -> Option<Self> {
        if v.leading_zeros() < u8::BITS - BITS {
            None
        } else {
            Some(Self(v))
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}
