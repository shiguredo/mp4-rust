use std::io::{Read, Write};

// TODO: Add Error type

// 単なる `Box` だと Rust の標準ライブラリのそれと名前が衝突するので変えておく
pub trait BaseBox: Encode + Decode {
    fn box_type(&self) -> &[u8];

    fn box_size(&self) -> std::io::Result<u64> {
        let mut size = ByteSize(0);
        self.encode(&mut size)?;
        Ok(size.0)
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
