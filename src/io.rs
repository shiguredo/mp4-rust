use std::io::{Read, Write};

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
