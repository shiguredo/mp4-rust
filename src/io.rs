//! no_std 環境用に [`std::io`] の代替コンポーネントを提供するためのモジュール

#[cfg(feature = "std")]
pub use std::io::{Chain, Error, ErrorKind, Read, Take, Write};

#[cfg(not(feature = "std"))]
pub use crate::io_no_std::{Chain, Error, ErrorKind, Read, Take, Write};
