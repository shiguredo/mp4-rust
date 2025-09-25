//! I/O 関連のコンポーネントを提供するモジュール
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
pub use std::io::Error;

/// no-std 環境用の [`std::io::Error`] のサブセット実装
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone)]
pub struct Error {
    /// エラーの種類
    pub kind: ErrorKind,
    /// エラーメッセージ
    pub message: &'static str,
}

#[cfg(not(feature = "std"))]
impl Error {
    fn unexpected_eof() -> Self {
        Error {
            kind: ErrorKind::UnexpectedEof,
            message: "Unexpected end of file",
        }
    }

    fn write_zero() -> Self {
        Error {
            kind: ErrorKind::Other,
            message: "Write returned zero",
        }
    }

    fn invalid_data(message: &'static str) -> Self {
        Error {
            kind: ErrorKind::InvalidData,
            message,
        }
    }
}

#[cfg(feature = "std")]
pub use std::io::ErrorKind;

/// no-std 環境用の [`std::io::ErrorKind`] のサブセット実装
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs)]
pub enum ErrorKind {
    InvalidData,
    InvalidInput,
    UnexpectedEof,
    Other,
}

#[cfg(feature = "std")]
pub use std::io::Read;

/// no-std 環境用の [`std::io::Read`] のサブセット実装
#[cfg(not(feature = "std"))]
pub trait Read: Sized {
    /// バッファにデータを読み込み、読み込んだバイト数を返す
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;

    /// バッファを完全に埋めるまでデータを読み込む
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let mut pos = 0;
        while pos < buf.len() {
            match self.read(&mut buf[pos..])? {
                0 => return Err(Error::unexpected_eof()),
                n => pos += n,
            }
        }
        Ok(())
    }

    /// EOF まで全てのデータをベクターに読み込み、読み込んだバイト数を返す
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error> {
        let start = buf.len();
        let mut tmp = [0u8; 512];
        loop {
            match self.read(&mut tmp)? {
                0 => return Ok(buf.len() - start),
                n => buf.extend_from_slice(&tmp[..n]),
            }
        }
    }

    /// EOF まで全てのデータを文字列として読み込み、読み込んだバイト数を返す
    fn read_to_string(&mut self, buf: &mut alloc::string::String) -> Result<usize, Error> {
        let mut bytes = Vec::new();
        let size = self.read_to_end(&mut bytes)?;
        let s = alloc::string::String::from_utf8(bytes)
            .map_err(|_| Error::invalid_data("Invalid UTF-8"))?;
        buf.push_str(&s);
        Ok(size)
    }

    /// 指定されたバイト数まで読み込みを制限する [`Take`] アダプターを作成する
    fn take(self, limit: u64) -> Take<Self> {
        Take::new(self, limit)
    }

    /// このリーダーと別のリーダーを連結する [`Chain`] アダプターを作成する
    fn chain<R: Read>(self, next: R) -> Chain<Self, R> {
        Chain::new(self, next)
    }
}

#[cfg(not(feature = "std"))]
impl<R: Read> Read for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        (*self).read(buf)
    }
}

#[cfg(feature = "std")]
pub use std::io::Write;

/// no-std 環境用の [`std::io::Write`] のサブセット実装
#[cfg(not(feature = "std"))]
pub trait Write: Sized {
    /// バッファからデータを書き込み、書き込んだバイト数を返す
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;

    /// バッファに残っているデータを強制的に出力先に送信する
    fn flush(&mut self) -> Result<(), Error>;

    /// バッファの全てのデータを完全に書き込む
    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        while !buf.is_empty() {
            match self.write(buf)? {
                0 => return Err(Error::write_zero()),
                n => buf = &buf[n..],
            }
        }
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl<W: Write> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        (*self).write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        (*self).flush()
    }
}

#[cfg(not(feature = "std"))]
impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
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
        // TODO: CursorReader のインタフェースをCursorにあわせる
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

impl<R: Read, const N: usize> Read for PeekReader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
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

/// no_std 用の [`std::io::Take`] のサブセット実装
pub struct Take<R> {
    inner: R,
    limit: u64,
}

impl<R: Read> Take<R> {
    /// 指定されたリーダーとバイト制限で新しいTakeインスタンスを作成する
    pub fn new(inner: R, limit: u64) -> Self {
        Take { inner, limit }
    }

    /// 残りの読み取り可能バイト数を返す
    pub fn limit(&self) -> u64 {
        self.limit
    }
}

#[cfg(feature = "std")]
impl<R: Read> std::io::Read for Take<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        let max = core::cmp::min(buf.len() as u64, self.limit) as usize;
        let n = self.inner.read(&mut buf[..max])?;
        self.limit -= n as u64;
        Ok(n)
    }
}

#[cfg(not(feature = "std"))]
impl<R: Read> Read for Take<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if self.limit == 0 {
            return Ok(0);
        }
        let max = core::cmp::min(buf.len() as u64, self.limit) as usize;
        let n = self.inner.read(&mut buf[..max])?;
        self.limit -= n as u64;
        Ok(n)
    }
}

/// no-std 環境用の [`std::io::Chain`] 実装
#[cfg(not(feature = "std"))]
pub struct Chain<R1, R2> {
    first: R1,
    second: R2,
    reading_second: bool,
}

#[cfg(not(feature = "std"))]
impl<R1: Read, R2: Read> Chain<R1, R2> {
    fn new(first: R1, second: R2) -> Self {
        Chain {
            first,
            second,
            reading_second: false,
        }
    }
}

#[cfg(not(feature = "std"))]
impl<R1: Read, R2: Read> Read for Chain<R1, R2> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
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

#[cfg(not(feature = "std"))]
impl<const N: usize> Read for [u8; N] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = core::cmp::min(buf.len(), self.len());
        buf[..len].copy_from_slice(&self[..len]);
        Ok(len)
    }
}

#[cfg(not(feature = "std"))]
impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = core::cmp::min(buf.len(), self.len());
        buf[..len].copy_from_slice(&self[..len]);
        *self = &self[len..];
        Ok(len)
    }
}
