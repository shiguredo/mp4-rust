use std::io::{Read, Write};

mod basic_types;
pub mod boxes;
mod io;

pub use basic_types::{BaseBox, BoxHeader, BoxSize, BoxType, FullBox, Mp4File, RawBox};
pub use io::{Decode, Encode, Error, Result};
