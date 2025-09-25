use alloc::{string::String, vec::Vec};

/// no_std 環境用の [`std::io::Error`] のサブセット実装
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    /// [`Error`] インスタンスを生成する
    pub fn new<E>(kind: ErrorKind, message: E) -> Self
    where
        E: Into<String>,
    {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// 汎用的なエラーを作成するメソッド
    pub fn other<E>(message: E) -> Self
    where
        E: Into<String>,
    {
        Self::new(ErrorKind::Other, message)
    }

    /// エラーの種類を返す
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

/// no_std 環境用の [`std::io::ErrorKind`] のサブセット実装
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs)]
pub enum ErrorKind {
    InvalidData,
    InvalidInput,
    UnexpectedEof,
    Other,
}

/// no_std 環境用の [`std::io::Read`] のサブセット実装
pub trait Read: Sized {
    /// バッファにデータを読み込み、読み込んだバイト数を返す
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;

    /// バッファを完全に埋めるまでデータを読み込む
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let mut pos = 0;
        while pos < buf.len() {
            match self.read(&mut buf[pos..])? {
                0 => {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Unexpected end of file",
                    ));
                }
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
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8"))?;
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

impl<R: Read> Read for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        (*self).read(buf)
    }
}

impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = core::cmp::min(buf.len(), self.len());
        buf[..len].copy_from_slice(&self[..len]);
        *self = &self[len..];
        Ok(len)
    }
}

/// no_std 環境用の [`std::io::Write`] のサブセット実装
pub trait Write: Sized {
    /// バッファからデータを書き込み、書き込んだバイト数を返す
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;

    /// バッファに残っているデータを強制的に出力先に送信する
    fn flush(&mut self) -> Result<(), Error>;

    /// バッファの全てのデータを完全に書き込む
    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        while !buf.is_empty() {
            match self.write(buf)? {
                0 => return Err(Error::new(ErrorKind::Other, "Write returned zero")),
                n => buf = &buf[n..],
            }
        }
        Ok(())
    }
}

impl<W: Write> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        (*self).write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        (*self).flush()
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

pub(crate) struct Cursor<const N: usize> {
    buf: [u8; N],
    pos: usize,
    len: usize,
}

impl<const N: usize> Cursor<N> {
    pub(crate) fn new(buf: [u8; N]) -> Self {
        let len = buf.len();
        Self { buf, pos: 0, len }
    }
}

impl<const N: usize> Read for Cursor<N> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let remaining = self.len.saturating_sub(self.pos);
        let to_read = core::cmp::min(buf.len(), remaining);
        buf[..to_read].copy_from_slice(&self.buf[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

/// no_std 環境用の [`std::io::Take`] のサブセット実装
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

/// no_std 環境用の [`std::io::Chain`] 実装
pub struct Chain<R1, R2> {
    first: R1,
    second: R2,
    reading_second: bool,
}

impl<R1: Read, R2: Read> Chain<R1, R2> {
    fn new(first: R1, second: R2) -> Self {
        Chain {
            first,
            second,
            reading_second: false,
        }
    }
}

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
