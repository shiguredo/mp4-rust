use std::{
    backtrace::Backtrace,
    io::{Cursor, ErrorKind, Read, Write},
    num::{NonZeroU16, NonZeroU32},
    panic::Location,
};

use crate::BoxType;

/// このライブラリ用の [`std::result::Result`] 型
pub type Result<T> = std::result::Result<T, Error>;

/// このライブラリ用のエラー型
pub struct Error {
    /// 具体的なエラー理由
    pub io_error: std::io::Error,

    /// エラー発生場所
    pub location: Option<&'static Location<'static>>,

    /// エラーが発生したボックスの種別
    pub box_type: Option<BoxType>,

    /// エラー発生箇所を示すバックトレース
    ///
    /// バックトレースは `RUST_BACKTRACE` 環境変数が設定されていない場合には取得されない
    pub backtrace: Backtrace,
}

impl Error {
    #[track_caller]
    pub(crate) fn invalid_data(message: &str) -> Self {
        Self::from(std::io::Error::new(ErrorKind::InvalidData, message))
    }

    #[track_caller]
    pub(crate) fn invalid_input(message: &str) -> Self {
        Self::from(std::io::Error::new(ErrorKind::InvalidInput, message))
    }

    #[track_caller]
    pub(crate) fn missing_box(missing_box: &str, parent_box: BoxType) -> Self {
        Self::invalid_data(&format!(
            "Missing mandatory '{missing_box}' box in '{parent_box}' box"
        ))
    }

    #[track_caller]
    pub(crate) fn unsupported(message: &str) -> Self {
        Self::from(std::io::Error::new(ErrorKind::Other, message))
    }

    pub(crate) fn with_box_type(mut self, box_type: BoxType) -> Self {
        if self.box_type.is_none() {
            self.box_type = Some(box_type);
        }
        self
    }
}

impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(value: std::io::Error) -> Self {
        Self {
            io_error: value,
            location: Some(std::panic::Location::caller()),
            box_type: None,
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
        if let Some(ty) = self.box_type {
            write!(f, "[{ty}] ")?;
        }

        write!(f, "{}", self.io_error)?;

        if let Some(l) = &self.location {
            write!(f, " (at {}:{})", l.file(), l.line())?;
        }

        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n\nBacktrace:\n{}", self.backtrace)?;
        }

        Ok(())
    }
}

/// `self` のバイト列への変換を行うためのトレイト
pub trait Encode {
    /// `self` をバイト列に変換して `writer` に書き込む
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()>;
}

impl Encode for u8 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u16 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u32 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u64 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i8 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i16 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i32 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i64 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for NonZeroU16 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.get().encode(writer)
    }
}

impl Encode for NonZeroU32 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.get().encode(writer)
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        for item in self {
            item.encode(writer)?;
        }
        Ok(())
    }
}

/// バイト列を `Self` に変換するためのトレイト
pub trait Decode: Sized {
    /// `reader` から読み込んだバイト列から `Self` を構築する
    fn decode<R: Read>(reader: &mut R) -> Result<Self>;
}

impl Decode for u8 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u16 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u32 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u64 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i8 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i16 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i32 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i64 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for NonZeroU16 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let v = u16::decode(reader)?;
        NonZeroU16::new(v)
            .ok_or_else(|| Error::invalid_data("Expected a non-zero integer, but got 0"))
    }
}

impl Decode for NonZeroU32 {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let v = u32::decode(reader)?;
        NonZeroU32::new(v)
            .ok_or_else(|| Error::invalid_data("Expected a non-zero integer, but got 0"))
    }
}

impl<T: Decode + Default + Copy, const N: usize> Decode for [T; N] {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut items = [T::default(); N];
        for item in &mut items {
            *item = T::decode(reader)?;
        }
        Ok(items)
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

        // エンコード処理が途中で失敗した場合には、失敗時点までに書き込まれたバイト数が採用される。
        // その失敗時の値は不正確であるが、いずれにせよここで失敗するということは、
        // 後続の実際のエンコード処理でも失敗するはずなので、その際のサイズ値が不正確でも問題はない。
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

#[derive(Debug)]
pub struct PeekReader<R, const N: usize> {
    buf: [u8; N],
    buf_start: usize,
    inner: R,
}

impl<R: Read, const N: usize> PeekReader<R, N> {
    pub fn new(inner: R) -> Self {
        Self {
            buf: [0; N],
            buf_start: 0,
            inner,
        }
    }

    pub fn into_reader(self) -> impl Read {
        Read::chain(
            Cursor::new(self.buf).take(self.buf_start as u64),
            self.inner,
        )
    }
}

impl<R: Read, const N: usize> Read for PeekReader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if N < self.buf_start + buf.len() {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                format!("[BUG] Peek buffer exhausted: buffer_size={N}"),
            ));
        }

        let read_size = self
            .inner
            .read(&mut self.buf[self.buf_start..][..buf.len()])?;
        buf[..read_size].copy_from_slice(&self.buf[self.buf_start..][..read_size]);
        self.buf_start += read_size;

        Ok(read_size)
    }
}
