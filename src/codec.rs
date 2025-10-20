#[cfg(feature = "std")]
use std::{
    backtrace::Backtrace,
    num::{NonZeroU16, NonZeroU32},
    panic::Location,
};

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

#[cfg(not(feature = "std"))]
use core::num::{NonZeroU16, NonZeroU32};

use crate::BoxType;
use crate::io::{ErrorKind, Read};

/// このライブラリ用の Result 型
pub type Result<T> = core::result::Result<T, Error>;

/// このライブラリ用の Result 型
pub type Result2<T> = core::result::Result<T, Error2>;

/// TODO: doc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind2 {
    /// TODO: doc
    InvalidInput,
    /// TODO: doc
    InvalidData,
    /// TODO: doc
    InsufficientBuffer,
}

/// TODO: doc
pub struct Error2 {
    /// TODO: doc
    pub kind: ErrorKind2,

    /// TODO: doc
    pub reason: String,

    /// エラー発生場所
    #[cfg(feature = "std")]
    pub location: &'static Location<'static>,

    /// エラーが発生したボックスの種別
    pub box_type: Option<BoxType>,

    /// エラー発生箇所を示すバックトレース
    ///
    /// バックトレースは `RUST_BACKTRACE` 環境変数が設定されていない場合には取得されない
    #[cfg(feature = "std")]
    pub backtrace: Backtrace,
}

impl Error2 {
    /// TODO: doc
    #[track_caller]
    pub fn new(kind: ErrorKind2) -> Self {
        Self::with_reason(kind, String::new())
    }

    /// TODO: doc
    #[track_caller]
    pub fn with_reason<T: Into<String>>(kind: ErrorKind2, reason: T) -> Self {
        Self {
            kind,
            reason: reason.into(),
            #[cfg(feature = "std")]
            location: std::panic::Location::caller(),
            box_type: None,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// TODO: doc
    #[track_caller]
    pub fn invalid_input<T: Into<String>>(reason: T) -> Self {
        Self::with_reason(ErrorKind2::InvalidInput, reason)
    }

    /// TODO: doc
    #[track_caller]
    pub fn insufficient_buffer() -> Self {
        Self::new(ErrorKind2::InsufficientBuffer)
    }

    /// TODO: doc
    #[track_caller]
    pub fn check_buffer_size(required_size: usize, buf: &[u8]) -> Result2<()> {
        if buf.len() < required_size {
            Err(Self::insufficient_buffer())
        } else {
            Ok(())
        }
    }
}

impl core::fmt::Debug for Error2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
    }
}

impl core::fmt::Display for Error2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(ty) = self.box_type {
            write!(f, "[{ty}] ")?;
        }

        write!(f, "{:?}: {}", self.kind, self.reason)?;

        #[cfg(feature = "std")]
        {
            write!(f, " (at {}:{})", self.location.file(), self.location.line())?;
            if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                write!(f, "\n\nBacktrace:\n{}", self.backtrace)?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error2 {}

/// このライブラリ用のエラー型
pub struct Error {
    /// 具体的なエラー理由
    pub io_error: crate::io::Error,

    /// エラー発生場所
    #[cfg(feature = "std")]
    pub location: Option<&'static Location<'static>>,

    /// エラーが発生したボックスの種別
    pub box_type: Option<BoxType>,

    /// エラー発生箇所を示すバックトレース
    ///
    /// バックトレースは `RUST_BACKTRACE` 環境変数が設定されていない場合には取得されない
    #[cfg(feature = "std")]
    pub backtrace: Backtrace,
}

impl Error {
    #[track_caller]
    pub(crate) fn invalid_data(message: &str) -> Self {
        Self::from(crate::io::Error::new(ErrorKind::InvalidData, message))
    }

    #[track_caller]
    pub(crate) fn missing_box(missing_box: &str, parent_box: BoxType) -> Self {
        Self::invalid_data(&format!(
            "Missing mandatory '{missing_box}' box in '{parent_box}' box"
        ))
    }

    #[track_caller]
    pub(crate) fn unsupported(message: &str) -> Self {
        Self::from(crate::io::Error::other(message))
    }

    pub(crate) fn with_box_type(mut self, box_type: BoxType) -> Self {
        if self.box_type.is_none() {
            self.box_type = Some(box_type);
        }
        self
    }
}

impl From<Error2> for Error {
    fn from(value: Error2) -> Self {
        let io_error = crate::io::Error::new(
            match value.kind {
                ErrorKind2::InvalidInput => ErrorKind::InvalidInput,
                ErrorKind2::InvalidData => ErrorKind::InvalidData,
                ErrorKind2::InsufficientBuffer => ErrorKind::InvalidData,
            },
            value.reason,
        );

        #[cfg(feature = "std")]
        {
            Self {
                io_error,
                location: Some(value.location),
                box_type: value.box_type,
                backtrace: value.backtrace,
            }
        }

        #[cfg(not(feature = "std"))]
        {
            Self {
                io_error,
                box_type: value.box_type,
            }
        }
    }
}

#[cfg(feature = "std")]
impl From<crate::io::Error> for Error {
    #[track_caller]
    fn from(value: crate::io::Error) -> Self {
        Self {
            io_error: value,
            location: Some(std::panic::Location::caller()),
            box_type: None,
            backtrace: Backtrace::capture(),
        }
    }
}

#[cfg(not(feature = "std"))]
impl From<crate::io::Error> for Error {
    fn from(value: crate::io::Error) -> Self {
        Self {
            io_error: value,
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

        write!(f, "{}", self.io_error)?;

        #[cfg(feature = "std")]
        {
            if let Some(l) = &self.location {
                write!(f, " (at {}:{})", l.file(), l.line())?;
            }

            if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                write!(f, "\n\nBacktrace:\n{}", self.backtrace)?;
            }
        }

        Ok(())
    }
}

/// TODO: doc
pub trait Encode {
    /// TODO: doc
    fn encode(&self, buf: &mut [u8]) -> Result2<usize>;

    /// TODO: doc
    fn encode_to_vec(&self) -> Result2<Vec<u8>> {
        let mut buf = vec![0; 64];
        loop {
            match self.encode(&mut buf) {
                Ok(size) => {
                    buf.truncate(size);
                    return Ok(buf);
                }
                Err(e) if e.kind == ErrorKind2::InsufficientBuffer => {
                    buf.resize(buf.len() * 2, 0);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Encode for u8 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(1, buf)?;
        buf[0] = *self;
        Ok(1)
    }
}

impl Encode for u16 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(2, buf)?;
        buf[..2].copy_from_slice(&self.to_be_bytes());
        Ok(2)
    }
}

impl Encode for u32 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(4, buf)?;
        buf[..4].copy_from_slice(&self.to_be_bytes());
        Ok(4)
    }
}

impl Encode for u64 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(8, buf)?;
        buf[..8].copy_from_slice(&self.to_be_bytes());
        Ok(8)
    }
}

impl Encode for i8 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(1, buf)?;
        buf[0] = *self as u8;
        Ok(1)
    }
}

impl Encode for i16 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(2, buf)?;
        buf[..2].copy_from_slice(&self.to_be_bytes());
        Ok(2)
    }
}

impl Encode for i32 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(4, buf)?;
        buf[..4].copy_from_slice(&self.to_be_bytes());
        Ok(4)
    }
}

impl Encode for i64 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(8, buf)?;
        buf[..8].copy_from_slice(&self.to_be_bytes());
        Ok(8)
    }
}

impl Encode for NonZeroU16 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        self.get().encode(buf)
    }
}

impl Encode for NonZeroU32 {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        self.get().encode(buf)
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        for item in self {
            offset += item.encode(&mut buf[offset..])?;
        }
        Ok(offset)
    }
}

impl Encode for [u8] {
    #[track_caller]
    fn encode(&self, buf: &mut [u8]) -> Result2<usize> {
        Error2::check_buffer_size(self.len(), buf)?;
        buf[..self.len()].copy_from_slice(self);
        Ok(self.len())
    }
}

/// TODO: doc
pub trait Decode2: Sized {
    /// TODO: doc
    fn decode2(buf: &[u8]) -> Result2<(Self, usize)>;
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
