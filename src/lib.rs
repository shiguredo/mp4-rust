mod basic_types;
pub mod boxes;
mod io;

pub use basic_types::{
    BaseBox, BoxHeader, BoxPath, BoxSize, BoxType, Either, FixedPointNumber, FullBox, FullBoxFlags,
    FullBoxHeader, IterUnknownBoxes, Mp4File, Mp4FileTime, Uint, UnknownBox, Utf8String,
};
pub use io::{Decode, Encode, Error, Result};
