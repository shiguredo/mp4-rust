use std::io::{Read, Write};

// TODO: Add Error type

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

pub trait Encode {
    fn encode<W: Write>(&self, writer: W) -> std::io::Result<()>;
}

impl Encode for u8 {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl Encode for u16 {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl Encode for u32 {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl Encode for u64 {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

pub trait Decode: Sized {
    fn decode<R: Read>(reader: R) -> std::io::Result<Self>;
}

impl Decode for u8 {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u16 {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u32 {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u64 {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
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
pub struct BoxHeader {
    pub box_type: BoxType,
    pub box_size: BoxSize,
}

impl BoxHeader {
    pub fn from_box<B: BaseBox>(b: &B) -> Self {
        let box_type = b.box_type();
        let box_size = b.box_size();
        Self { box_type, box_size }
    }

    pub fn header_size(self) -> usize {
        self.box_type.external_size() + self.box_size.external_size()
    }

    pub fn with_box_payload_reader<T, R: Read, F>(self, reader: R, f: F) -> std::io::Result<T>
    where
        F: FnOnce(&mut std::io::Take<R>) -> std::io::Result<T>,
    {
        let mut reader = if self.box_size.get() == 0 {
            reader.take(u64::MAX)
        } else {
            let payload_size = self
                .box_size
                .get()
                .checked_sub(self.header_size() as u64)
                .ok_or_else(|| std::io::ErrorKind::InvalidData)?; // TODO: error message
            reader.take(payload_size)
        };

        let value = f(&mut reader)?;
        if reader.limit() != 0 {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        Ok(value)
    }
}

impl Encode for BoxHeader {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        let large_size = self.box_size.get() > u32::MAX as u64;
        if large_size {
            1u32.encode(&mut writer)?;
        } else {
            (self.box_size.get() as u32).encode(&mut writer)?;
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
            self.box_size.get().encode(&mut writer)?;
        }

        Ok(())
    }
}

impl Decode for BoxHeader {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut box_size = u32::decode(&mut reader)? as u64;

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
            box_size = u64::decode(&mut reader)?;
        }
        let box_size =
            BoxSize::new(box_type, box_size).ok_or_else(|| std::io::ErrorKind::InvalidData)?; // TODO: error message

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

    // TODO: rename
    pub const fn external_size(self) -> usize {
        if matches!(self, Self::Normal(_)) {
            4
        } else {
            4 + 16
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
pub struct RawBox {
    pub box_type: BoxType,
    pub box_size: BoxSize,
    pub payload: Vec<u8>,
}

impl Encode for RawBox {
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        BoxHeader::from_box(self).encode(&mut writer)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Decode for RawBox {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let header = BoxHeader::decode(&mut reader)?;
        dbg!(header);

        let mut payload = Vec::new();
        header.with_box_payload_reader(reader, |reader| reader.read_to_end(&mut payload))?;
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

    fn box_payload_size(&self) -> u64 {
        self.payload.len() as u64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Brand([u8; 4]);

impl Brand {
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
    fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(&self.0)
    }
}

impl Decode for Brand {
    fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

// /// [ISO/IEC 14496-12] FileTypeBox class
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct FtypBox {
//     pub major_brand: Brand,
//     pub minor_version: u32,
//     pub compatible_brands: Vec<Brand>,
// }

// impl Encode for FtypBox {
//     fn encode<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
//         BoxHeader::from_box(self).encode(&mut writer)?;

//         self.major_brand.encode(&mut writer)?;
//         writer.write_u32(self.minor_version)?;
//         for brand in &self.compatible_brands {
//             brand.encode(&mut writer)?;
//         }
//         Ok(())
//     }
// }

// impl Decode for FtypBox {
//     fn decode<R: Read>(mut reader: R) -> std::io::Result<Self> {
//         let major_brand = Brand::decode(&mut reader)?;
//         let minor_version = reader.read_u32()?;
//         todo!();
//     }
// }
