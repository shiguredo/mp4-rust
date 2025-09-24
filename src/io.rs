//! I/O 関連のコンポーネントを提供するモジュール
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
pub use std::io::Error;

/// no-std環境用のI/Oエラー型
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone)]
pub struct Error {
    /// エラーの種類
    pub kind: ErrorKind,
    /// エラーメッセージ
    pub message: &'static str,
}

#[cfg(feature = "std")]
pub use std::io::ErrorKind;

/// no-std環境用のエラー種別
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidData,
    InvalidInput,
    UnexpectedEof,
    Other,
}

#[cfg(feature = "std")]
pub use std::io::Read;

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
impl<R: Read> Read for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (*self).read(buf)
    }
}

#[cfg(feature = "std")]
pub use std::io::Write;

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
impl<W: Write> Write for &mut W {
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
pub(crate) struct PeekReader<R, const N: usize> {
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
            std::io::Cursor::new(self.buf).take(self.buf_start as u64),
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
