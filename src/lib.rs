//! MP4 のボックスのエンコードおよびデコードを行うためのライブラリ
#![warn(missing_docs)]
mod auxiliary;
mod basic_types;
pub mod boxes;
pub mod descriptors;
mod io;

pub use basic_types::{
    BaseBox, BoxHeader, BoxSize, BoxType, Either, FixedPointNumber, FullBox, FullBoxFlags,
    FullBoxHeader, Mp4File, Mp4FileTime, Uint, Utf8String,
};
pub use io::{Decode, Encode, Error, Result};

// [NOTE]
// Windows 環境では aux.rs というファイル名が予約語で、リポジトリに含まれていると git clone に失敗するため、
// ファイル名自体は auxiliary.rs にして lib.rs の中で aux モジュール以下に再エクスポートしている。
pub mod aux {
    //! MP4 の仕様とは直接は関係がない、実装上便利な補助的なコンポーネントを集めたモジュール

    pub use crate::auxiliary::{
        ChunkAccessor, SampleAccessor, SampleTableAccessor, SampleTableAccessorError,
    };
}
