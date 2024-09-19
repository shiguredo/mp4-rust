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
pub trait BaseBox: Encode + Decode {
    fn box_type(&self) -> BoxType;

    fn box_size(&self) -> BoxSize {
        BoxSize::with_payload_size(self.box_type(), self.box_payload_size())
    }

    fn box_payload_size(&self) -> u64;
}

pub trait FullBox: BaseBox {
    fn box_version(&self) -> u8;
    fn box_flags(&self) -> u32; // u24
}

pub trait IterUnknownBoxes {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoxPath(Vec<BoxType>);

impl BoxPath {
    pub fn new(box_type: BoxType) -> Self {
        Self(vec![box_type])
    }

    pub fn get(&self) -> &[BoxType] {
        &self.0
    }

    pub fn join(mut self, parent: BoxType) -> Self {
        self.0.push(parent);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Mp4File<B = RootBox> {
    pub ftyp_box: FtypBox,
    pub boxes: Vec<B>,
}

impl<B: BaseBox> Decode for Mp4File<B> {
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

impl<B: BaseBox> Encode for Mp4File<B> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.ftyp_box.encode(writer)?;

        for b in &self.boxes {
            b.encode(writer)?;
        }
        Ok(())
    }
}

impl<B: IterUnknownBoxes> IterUnknownBoxes for Mp4File<B> {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        self.boxes.iter().flat_map(|b| b.iter_unknown_boxes())
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
        Self(box_type.external_size() as u64 + payload_size)
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
}

impl IterUnknownBoxes for UnknownBox {
    fn iter_unknown_boxes(&self) -> impl '_ + Iterator<Item = (BoxPath, &UnknownBox)> {
        std::iter::once((BoxPath::new(self.box_type), self))
    }
}

/// 1904/1/1 からの経過秒数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

impl Encode for Mp4FileTime {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.0.encode(writer)
    }
}

impl Decode for Mp4FileTime {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        Decode::decode(reader).map(Self)
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
