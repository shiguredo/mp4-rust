//! no_std 環境用に [`std::io`] の代替コンポーネントを提供するためのモジュール

#[cfg(feature = "std")]
pub use std::io::{Chain, Error, ErrorKind, Read, Take, Write};

#[cfg(not(feature = "std"))]
pub use crate::io_no_std::{Chain, Error, ErrorKind, Read, Take, Write};

#[cfg(feature = "std")]
use std::io::Cursor;

#[cfg(not(feature = "std"))]
use crate::io_no_std::Cursor;

#[derive(Debug, Default)]
pub(crate) struct ExternalBytes(pub u64);

impl ExternalBytes {
    pub fn calc<F>(f: F) -> u64
    where
        F: FnOnce(&mut Self) -> crate::Result<()>,
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
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct PeekReader<R, const N: usize> {
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
        Cursor::new(self.buf)
            .take(self.buf_start as u64)
            .chain(self.inner)
    }
}

impl<R: Read, const N: usize> Read for PeekReader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if N < self.buf_start + buf.len() {
            return Err(Error::new(ErrorKind::InvalidData, "Peek buffer exhausted"));
        }

        let read_size = self
            .inner
            .read(&mut self.buf[self.buf_start..][..buf.len()])?;
        buf[..read_size].copy_from_slice(&self.buf[self.buf_start..][..read_size]);
        self.buf_start += read_size;

        Ok(read_size)
    }
}
