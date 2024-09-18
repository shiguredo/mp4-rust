use std::io::{Read, Write};

use crate::{
    io::{ExternalBytes, PeekReader},
    BaseBox, BoxHeader, BoxType, Decode, Encode, RawBox, Result,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootBox {
    Free(FreeBox), // free, mdat, moov
    Unknown(RawBox),
}

impl Encode for RootBox {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            RootBox::Free(b) => b.encode(writer),
            RootBox::Unknown(b) => b.encode(writer),
        }
    }
}

impl Decode for RootBox {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut reader = PeekReader::<_, { BoxHeader::MAX_SIZE }>::new(reader);
        let header = BoxHeader::decode(&mut reader)?;
        match header.box_type {
            FreeBox::TYPE => Decode::decode(&mut reader.into_reader()).map(Self::Free),
            _ => Decode::decode(&mut reader.into_reader()).map(Self::Unknown),
        }
    }
}

impl BaseBox for RootBox {
    fn box_type(&self) -> BoxType {
        match self {
            RootBox::Free(b) => b.box_type(),
            RootBox::Unknown(b) => b.box_type(),
        }
    }

    fn box_payload_size(&self) -> u64 {
        match self {
            RootBox::Free(b) => b.box_payload_size(),
            RootBox::Unknown(b) => b.box_payload_size(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeBox {
    pub box_type: BoxType,
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
        let mut payload = Vec::new();
        header.with_box_payload_reader(reader, |reader| Ok(reader.read_to_end(&mut payload)?))?;
        Ok(Self {
            box_type: header.box_type,
            payload,
        })
    }
}

impl BaseBox for FreeBox {
    fn box_type(&self) -> BoxType {
        self.box_type
    }

    fn box_payload_size(&self) -> u64 {
        self.payload.len() as u64
    }
}
