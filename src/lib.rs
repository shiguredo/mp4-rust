//! MP4 のボックスのエンコードおよびデコードを行うためのライブラリ
#![warn(missing_docs)]
pub mod aux;
mod basic_types;
pub mod boxes;
mod io;

pub use basic_types::{
    BaseBox, BoxHeader, BoxSize, BoxType, Either, FixedPointNumber, FullBox, FullBoxFlags,
    FullBoxHeader, Mp4File, Mp4FileTime, Uint, Utf8String,
};
pub use io::{Decode, Encode, Error, Result};
