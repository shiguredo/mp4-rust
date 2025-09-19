#[cfg(feature = "std")]
use std::{
    backtrace::Backtrace,
    io::{Cursor, ErrorKind},
    panic::Location,
};

#[cfg(feature = "std")]
use std::num::{NonZeroU16, NonZeroU32};

#[cfg(not(feature = "std"))]
use core::num::{NonZeroU16, NonZeroU32};

use crate::BoxType;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// no-std環境用のI/Oエラー型
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone)]
pub struct IoError {
    pub kind: ErrorKind,
    pub message: &'static str,
}

/// no-std環境用のエラー種別
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidData,
    InvalidInput,
    UnexpectedEof,
    Other,
}

/// no-std環境用のReadトレイト
#[cfg(not(feature = "std"))]
pub trait Read: Sized {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut pos = 0;
        while pos < buf.len() {
            match self.read(&mut buf[pos..])? {
                0 => return Err(Error::unexpected_eof()),
                n => pos += n,
            }
        }
        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let start = buf.len();
        let mut tmp = [0u8; 512];
        loop {
            match self.read(&mut tmp)? {
                0 => return Ok(buf.len() - start),
                n => buf.extend_from_slice(&tmp[..n]),
            }
        }
    }

    fn read_to_string(&mut self, buf: &mut alloc::string::String) -> Result<usize> {
        let mut bytes = Vec::new();
        let size = self.read_to_end(&mut bytes)?;
        let s = alloc::string::String::from_utf8(bytes)
            .map_err(|_| Error::invalid_data("Invalid UTF-8"))?;
        buf.push_str(&s);
        Ok(size)
    }
}

// &mut Rにもトレイトを実装
#[cfg(not(feature = "std"))]
impl<'a, R: Read + ?Sized> Read for &'a mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (*self).read(buf)
    }
}

/// no-std環境用のWriteトレイト
#[cfg(not(feature = "std"))]
pub trait Write: Sized {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf)? {
                0 => return Err(Error::write_zero()),
                n => buf = &buf[n..],
            }
        }
        Ok(())
    }
}

// &mut Wにもトレイトを実装
#[cfg(not(feature = "std"))]
impl<'a, W: Write + ?Sized> Write for &'a mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (*self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (*self).flush()
    }
}

// Vec<u8>にWriteトレイトを実装
#[cfg(not(feature = "std"))]
impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// std環境ではstd::io::{Read, Write}を再エクスポート
#[cfg(feature = "std")]
pub use std::io::{Read, Write};

/// no-std環境用のCursor風リーダー
#[cfg(not(feature = "std"))]
struct CursorReader<const N: usize> {
    buf: [u8; N],
    pos: usize,
    len: usize,
}

#[cfg(not(feature = "std"))]
impl<const N: usize> Read for CursorReader<N> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let remaining = self.len.saturating_sub(self.pos);
        let to_read = core::cmp::min(buf.len(), remaining);
        buf[..to_read].copy_from_slice(&self.buf[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

/// no-std環境用のChain風リーダー
#[cfg(not(feature = "std"))]
struct ChainReader<R1, R2> {
    first: R1,
    second: R2,
    reading_second: bool,
}

#[cfg(not(feature = "std"))]
impl<R1: Read, R2: Read> Read for ChainReader<R1, R2> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.reading_second {
            match self.first.read(buf)? {
                0 => {
                    self.reading_second = true;
                    self.second.read(buf)
                }
                n => Ok(n),
            }
        } else {
            self.second.read(buf)
        }
    }
}

/// このライブラリ用の Result 型
#[cfg(feature = "std")]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(not(feature = "std"))]
pub type Result<T> = core::result::Result<T, Error>;

/// このライブラリ用のエラー型
pub struct Error {
    /// 具体的なエラー理由
    #[cfg(feature = "std")]
    pub io_error: std::io::Error,

    #[cfg(not(feature = "std"))]
    pub io_error: IoError,

    /// エラー発生場所
    #[cfg(feature = "std")]
    pub location: Option<&'static Location<'static>>,

    #[cfg(not(feature = "std"))]
    pub location: Option<()>,

    /// エラーが発生したボックスの種別
    pub box_type: Option<BoxType>,

    /// エラー発生箇所を示すバックトレース
    ///
    /// バックトレースは `RUST_BACKTRACE` 環境変数が設定されていない場合には取得されない
    #[cfg(feature = "std")]
    pub backtrace: Backtrace,
}

impl Error {
    #[cfg(not(feature = "std"))]
    pub(crate) fn unexpected_eof() -> Self {
        Self {
            io_error: IoError {
                kind: ErrorKind::UnexpectedEof,
                message: "Unexpected end of file",
            },
            location: None,
            box_type: None,
        }
    }

    #[cfg(not(feature = "std"))]
    pub(crate) fn write_zero() -> Self {
        Self {
            io_error: IoError {
                kind: ErrorKind::Other,
                message: "Write returned zero",
            },
            location: None,
            box_type: None,
        }
    }

    #[track_caller]
    pub(crate) fn invalid_data(message: &str) -> Self {
        #[cfg(feature = "std")]
        return Self::from(std::io::Error::new(ErrorKind::InvalidData, message));

        #[cfg(not(feature = "std"))]
        return Self {
            io_error: IoError {
                kind: ErrorKind::InvalidData,
                message: "Invalid data",
            },
            location: None,
            box_type: None,
        };
    }

    #[track_caller]
    pub(crate) fn invalid_input(message: &str) -> Self {
        #[cfg(feature = "std")]
        return Self::from(std::io::Error::new(ErrorKind::InvalidInput, message));

        #[cfg(not(feature = "std"))]
        return Self {
            io_error: IoError {
                kind: ErrorKind::InvalidInput,
                message: "Invalid input",
            },
            location: None,
            box_type: None,
        };
    }

    #[track_caller]
    pub(crate) fn missing_box(missing_box: &str, parent_box: BoxType) -> Self {
        #[cfg(feature = "std")]
        return Self::invalid_data(&format!(
            "Missing mandatory '{missing_box}' box in '{parent_box}' box"
        ));

        #[cfg(not(feature = "std"))]
        return Self::invalid_data("Missing mandatory box");
    }

    #[track_caller]
    pub(crate) fn unsupported(message: &str) -> Self {
        #[cfg(feature = "std")]
        return Self::from(std::io::Error::other(message));

        #[cfg(not(feature = "std"))]
        return Self {
            io_error: IoError {
                kind: ErrorKind::Other,
                message: "Unsupported operation",
            },
            location: None,
            box_type: None,
        };
    }

    pub(crate) fn with_box_type(mut self, box_type: BoxType) -> Self {
        if self.box_type.is_none() {
            self.box_type = Some(box_type);
        }
        self
    }
}

#[cfg(feature = "std")]
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

#[cfg(not(feature = "std"))]
impl From<IoError> for Error {
    fn from(value: IoError) -> Self {
        Self {
            io_error: value,
            location: None,
            box_type: None,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.io_error)
    }
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(ty) = self.box_type {
            write!(f, "[{ty}] ")?;
        }

        #[cfg(feature = "std")]
        {
            write!(f, "{}", self.io_error)?;

            if let Some(l) = &self.location {
                write!(f, " (at {}:{})", l.file(), l.line())?;
            }

            if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                write!(f, "\n\nBacktrace:\n{}", self.backtrace)?;
            }
        }

        #[cfg(not(feature = "std"))]
        {
            write!(
                f,
                "{}: {}",
                match self.io_error.kind {
                    ErrorKind::InvalidData => "InvalidData",
                    ErrorKind::InvalidInput => "InvalidInput",
                    ErrorKind::UnexpectedEof => "UnexpectedEof",
                    ErrorKind::Other => "Other",
                },
                self.io_error.message
            )?;
        }

        Ok(())
    }
}

/// `self` のバイト列への変換を行うためのトレイト
pub trait Encode {
    /// `self` をバイト列に変換して `writer` に書き込む
    fn encode<W: Write>(&self, writer: W) -> Result<()>;
}

impl Encode for u8 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u16 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u32 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for u64 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i8 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i16 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i32 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for i64 {
    #[track_caller]
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl Encode for NonZeroU16 {
    #[track_caller]
    fn encode<W: Write>(&self, writer: W) -> Result<()> {
        self.get().encode(writer)
    }
}

impl Encode for NonZeroU32 {
    #[track_caller]
    fn encode<W: Write>(&self, writer: W) -> Result<()> {
        self.get().encode(writer)
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        for item in self {
            item.encode(&mut writer)?;
        }
        Ok(())
    }
}

/// バイト列を `Self` に変換するためのトレイト
pub trait Decode: Sized {
    /// `reader` から読み込んだバイト列から `Self` を構築する
    fn decode<R: Read>(reader: R) -> Result<Self>;
}

impl Decode for u8 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u16 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u32 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for u64 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i8 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i16 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i32 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for i64 {
    #[track_caller]
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; Self::BITS as usize / 8];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Decode for NonZeroU16 {
    #[track_caller]
    fn decode<R: Read>(reader: R) -> Result<Self> {
        let v = u16::decode(reader)?;
        NonZeroU16::new(v)
            .ok_or_else(|| Error::invalid_data("Expected a non-zero integer, but got 0"))
    }
}

impl Decode for NonZeroU32 {
    #[track_caller]
    fn decode<R: Read>(reader: R) -> Result<Self> {
        let v = u32::decode(reader)?;
        NonZeroU32::new(v)
            .ok_or_else(|| Error::invalid_data("Expected a non-zero integer, but got 0"))
    }
}

impl<T: Decode + Default + Copy, const N: usize> Decode for [T; N] {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut items = [T::default(); N];
        for item in &mut items {
            *item = T::decode(&mut reader)?;
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

#[cfg(feature = "std")]
impl std::io::Write for ExternalBytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl Write for ExternalBytes {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct PeekReader<R, const N: usize> {
    buf: [u8; N],
    buf_start: usize,
    inner: R,
}

#[cfg(feature = "std")]
impl<R: std::io::Read, const N: usize> PeekReader<R, N> {
    pub fn new(inner: R) -> Self {
        Self {
            buf: [0; N],
            buf_start: 0,
            inner,
        }
    }

    pub fn into_reader(self) -> impl std::io::Read {
        std::io::Read::chain(
            Cursor::new(self.buf).take(self.buf_start as u64),
            self.inner,
        )
    }
}

#[cfg(not(feature = "std"))]
impl<R: Read, const N: usize> PeekReader<R, N> {
    pub fn new(inner: R) -> Self {
        Self {
            buf: [0; N],
            buf_start: 0,
            inner,
        }
    }

    pub fn into_reader(self) -> impl Read {
        ChainReader {
            first: CursorReader {
                buf: self.buf,
                pos: 0,
                len: self.buf_start,
            },
            second: self.inner,
            reading_second: false,
        }
    }
}

#[cfg(feature = "std")]
impl<R: std::io::Read, const N: usize> std::io::Read for PeekReader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if N < self.buf_start + buf.len() {
            return Err(std::io::Error::other(format!(
                "[BUG] Peek buffer exhausted: buffer_size={N}"
            )));
        }

        let read_size = self
            .inner
            .read(&mut self.buf[self.buf_start..][..buf.len()])?;
        buf[..read_size].copy_from_slice(&self.buf[self.buf_start..][..read_size]);
        self.buf_start += read_size;

        Ok(read_size)
    }
}

#[cfg(not(feature = "std"))]
impl<R: Read, const N: usize> Read for PeekReader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if N < self.buf_start + buf.len() {
            return Err(Error::invalid_data("Peek buffer exhausted"));
        }

        let read_size = self
            .inner
            .read(&mut self.buf[self.buf_start..][..buf.len()])?;
        buf[..read_size].copy_from_slice(&self.buf[self.buf_start..][..read_size]);
        self.buf_start += read_size;

        Ok(read_size)
    }
}
