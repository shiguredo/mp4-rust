use std::io::{Read, Write};

use crate::{Decode, Encode, Result};

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
//
