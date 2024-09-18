use std::{
    backtrace::Backtrace,
    io::{ErrorKind, Read, Write},
};

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    pub io_error: std::io::Error,
    pub backtrace: Backtrace,
}

impl Error {
    pub(crate) fn invalid_data(message: &str) -> Self {
        Self::from(std::io::Error::new(ErrorKind::InvalidData, message))
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            io_error: value,
            backtrace: Backtrace::capture(),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.io_error)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "{}\n\nBacktrace:\n{}", self.io_error, self.backtrace)
        } else {
            write!(f, "{}", self.io_error)
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

#[derive(Debug, Default)]
pub struct ExternalBytes(pub u64);

impl ExternalBytes {
    pub fn calc<F>(f: F) -> u64
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        let mut external_bytes = Self(0);

        // TODO: 途中で失敗した場合は、それまでに書き込まれたサイズでいい理由を書く
        let _ = f(&mut external_bytes);
        external_bytes.0
    }
}

impl Write for ExternalBytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
