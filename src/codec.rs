#[cfg(feature = "std")]
use std::{
    backtrace::Backtrace,
    num::{NonZeroU16, NonZeroU32},
    panic::Location,
};

#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

#[cfg(not(feature = "std"))]
use core::num::{NonZeroU16, NonZeroU32};

use crate::BoxType;
use crate::io::{ErrorKind, Read, Write};

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
    pub location: Option<&'static Location<'static>>,

    /// エラーが発生したボックスの種別
    pub box_type: Option<BoxType>,

    /// エラー発生箇所を示すバックトレース
    ///
    /// バックトレースは `RUST_BACKTRACE` 環境変数が設定されていない場合には取得されない
    #[cfg(feature = "std")]
    pub backtrace: Backtrace,
}

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
    pub(crate) fn invalid_input(message: &str) -> Self {
        Self::from(crate::io::Error::new(ErrorKind::InvalidInput, message))
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
pub trait Encode2 {
    /// TODO: doc
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize>;
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
