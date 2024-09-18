use std::{
    io::{ErrorKind, Read, Write},
    num::NonZeroU64,
};

// TODO: Add Error type

// 単なる `Box` だと Rust の標準ライブラリのそれと名前が衝突するので変えておく
pub trait BaseBox: Encode + Decode {
    fn box_type(&self) -> BoxType;

    fn box_size(&self) -> BoxSize {
        let mut size = ByteSize(0);
        if self.encode(&mut size).is_err() {
            BoxSize::Unknown
        } else if let Some(n) = NonZeroU64::new(size.0) {
            BoxSize::Known(n)
        } else {
            BoxSize::Unknown
        }
    }
}

pub trait FullBox: BaseBox {
    fn box_version(&self) -> u8;
    fn box_flags(&self) -> u32; // u24
}

pub trait Encode {
    fn encode<W: Write>(&self, writer: W) -> std::io::Result<()>;
}

pub trait Decode: Sized {
    fn decode<R: Read>(reader: R) -> std::io::Result<Self>;
}

#[derive(Debug)]
struct ByteSize(pub u64);

impl Write for ByteSize {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Mp4File<B> {
    // TODO: ftyp_box
    pub boxes: Vec<B>,
}

impl<B: BaseBox> Decode for Mp4File<B> {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut boxes = Vec::new();
        let mut buf = [0];
        while reader.read(&mut buf)? != 0 {
            let b = B::decode(buf.chain(&mut reader))?;
            boxes.push(b);
        }
        Ok(Self { boxes })
    }
}

impl<B: BaseBox> Encode for Mp4File<B> {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        for b in &self.boxes {
            b.encode(&mut writer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseBoxHeader {
    pub box_type: BoxType,
    pub box_size: BoxSize,
}

impl BaseBoxHeader {
    pub fn from_box<B: BaseBox>(b: &B) -> Self {
        Self {
            box_type: b.box_type(),
            box_size: b.box_size(),
        }
    }

    pub fn header_size(self) -> usize {
        let mut size = 0;

        if matches!(self.box_type, BoxType::Normal(_)) {
            size += 4;
        } else {
            size += 20;
        }

        if matches!(self.box_size, BoxSize::Known(_)) {
            size += 4;
        } else {
            size += 12;
        }

        size
    }

    pub fn payload_size(self) -> std::io::Result<Option<u64>> {
        match self.box_size {
            BoxSize::Unknown => Ok(None),
            BoxSize::Known(size) => {
                let payload_size = size
                    .get()
                    .checked_sub(self.header_size() as u64)
                    .ok_or_else(|| ErrorKind::InvalidData)?; // TODO: error message
                Ok(Some(payload_size))
            }
        }
    }
}

impl Encode for BaseBoxHeader {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        let large_size = self.box_size.get() > u32::MAX as u64;
        if large_size {
            writer.write_u32(1)?;
        } else {
            writer.write_u32(self.box_size.get() as u32)?;
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
            writer.write_u64(self.box_size.get())?;
        }

        Ok(())
    }
}

impl Decode for BaseBoxHeader {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut box_size = reader.read_u32()? as u64;

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
            box_size = reader.read_u64()?;
        }
        let box_size = if let Some(n) = NonZeroU64::new(box_size) {
            BoxSize::Known(n)
        } else {
            BoxSize::Unknown
        };

        Ok(Self { box_type, box_size })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoxSize {
    Unknown,
    Known(NonZeroU64),
}

impl BoxSize {
    pub const fn get(self) -> u64 {
        match self {
            BoxSize::Unknown => 0,
            BoxSize::Known(v) => v.get(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawBox {
    pub box_type: BoxType,
    pub box_size: BoxSize,
    pub payload: Vec<u8>,
}

impl Encode for RawBox {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        BaseBoxHeader::from_box(self).encode(&mut writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for RawBox {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let header = BaseBoxHeader::decode(&mut reader)?;
        dbg!(header);

        let mut payload = Vec::new();
        match header.payload_size()? {
            None => {
                reader.read_to_end(&mut payload)?;
            }
            Some(size) => {
                reader.take(size).read_to_end(&mut payload)?;
                if payload.len() as u64 != size {
                    // TODO: error message
                    return Err(std::io::ErrorKind::InvalidData.into());
                }
            }
        }
        Ok(Self {
            box_type: header.box_type,
            box_size: header.box_size,
            payload,
        })
    }
}

impl BaseBox for RawBox {
    fn box_type(&self) -> BoxType {
        self.box_type
    }

    fn box_size(&self) -> BoxSize {
        self.box_size
    }
}

pub trait WriteExt {
    fn write_u8(&mut self, v: u8) -> std::io::Result<()>;
    fn write_u16(&mut self, v: u16) -> std::io::Result<()>;
    fn write_u32(&mut self, v: u32) -> std::io::Result<()>;
    fn write_u64(&mut self, v: u64) -> std::io::Result<()>;
}

impl<T: Write> WriteExt for T {
    fn write_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.write_all(&[v])
    }

    fn write_u16(&mut self, v: u16) -> std::io::Result<()> {
        self.write_all(&v.to_be_bytes())
    }

    fn write_u32(&mut self, v: u32) -> std::io::Result<()> {
        self.write_all(&v.to_be_bytes())
    }

    fn write_u64(&mut self, v: u64) -> std::io::Result<()> {
        self.write_all(&v.to_be_bytes())
    }
}

pub trait ReadExt {
    fn read_u8(&mut self) -> std::io::Result<u8>;
    fn read_u16(&mut self) -> std::io::Result<u16>;
    fn read_u32(&mut self) -> std::io::Result<u32>;
    fn read_u64(&mut self) -> std::io::Result<u64>;
}

impl<T: Read> ReadExt for T {
    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }
}
