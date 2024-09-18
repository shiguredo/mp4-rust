use std::{
    backtrace::Backtrace,
    io::{ErrorKind, Read, Write},
};

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    io: std::io::Error,
    trace: Backtrace,
}

impl Error {
    pub(crate) fn invalid_data(message: &str) -> Self {
        Self::from(std::io::Error::new(ErrorKind::InvalidData, message))
    }

    pub fn kind(&self) -> ErrorKind {
        self.io.kind()
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            io: value,
            trace: Backtrace::capture(),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.io)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.trace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "{}\n\nBacktrace:\n{}", self.io, self.trace)
        } else {
            write!(f, "{}", self.io)
        }
    }
}

pub trait Encode {
    fn encode<W: Write>(&self, writer: W) -> Result<()>;
}

impl Encode for u8 {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u16 {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u32 {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u64 {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

pub trait Decode: Sized {
    fn decode<R: Read>(reader: R) -> Result<Self>;
}

impl Decode for u8 {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u16 {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u32 {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u64 {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}
