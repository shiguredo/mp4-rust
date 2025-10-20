use core::{
    ops::{BitAnd, Shl, Shr, Sub},
    time::Duration,
};

#[cfg(not(feature = "std"))]
use alloc::{borrow::ToOwned, boxed::Box, format, string::String, vec::Vec};

use crate::{
    Decode, Encode2, Error, Error2, Result, Result2,
    boxes::{FtypBox, RootBox},
    io::{PeekReader, Read, Take},
};

/// 全てのボックスが実装するトレイト
///
/// 本来なら `Box` という名前が適切だが、それだと標準ライブラリの [`std::boxed::Box`] と名前が
/// 衝突してしまうので、それを避けるために `BaseBox` としている
pub trait BaseBox {
    /// ボックスの種別
    fn box_type(&self) -> BoxType;

    /// 未知のボックスかどうか
    ///
    /// 基本的には `false` を返すデフォルト実装のままで問題ないが、
    /// [`UnknownBox`](crate::boxes::UnknownBox) や [`IgnoredBox`](crate::boxes::IgnoredBox) を
    /// 含む `enum` を定義する場合には、独自の実装が必要となる
    fn is_unknown_box(&self) -> bool {
        false
    }

    /// 子ボックスを走査するイテレーターを返す
    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>>;
}

pub(crate) fn as_box_object<T: BaseBox>(t: &T) -> &dyn BaseBox {
    t
}

/// フルボックスを表すトレイト
pub trait FullBox: BaseBox {
    /// フルボックスのバージョンを返す
    fn full_box_version(&self) -> u8;

    /// フルボックスのフラグを返す
    fn full_box_flags(&self) -> FullBoxFlags;
}

/// MP4 ファイルを表す構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mp4File<B = RootBox> {
    /// MP4 ファイルの先頭に位置する `ftyp` ボックス
    pub ftyp_box: FtypBox,

    /// `ftyp` に続くボックス群
    pub boxes: Vec<B>,
}

impl<B: BaseBox> Mp4File<B> {
    /// ファイル内のトップレベルのボックス群を走査するイテレーターを返す
    pub fn iter(&self) -> impl Iterator<Item = &dyn BaseBox> {
        core::iter::empty()
            .chain(core::iter::once(&self.ftyp_box).map(as_box_object))
            .chain(self.boxes.iter().map(as_box_object))
    }
}

impl<B: BaseBox + Decode> Decode for Mp4File<B> {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let ftyp_box = FtypBox::decode(&mut reader)?;

        let mut boxes = Vec::new();
        let mut buf = [0];
        while reader.read(&mut buf)? != 0 {
            let b = B::decode(&mut buf.chain(&mut reader))?;
            boxes.push(b);
        }
        Ok(Self { ftyp_box, boxes })
    }
}

/// [`BaseBox`] に共通のヘッダー
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxHeader {
    /// ボックスの種別
    pub box_type: BoxType,

    /// ボックスのサイズ
    pub box_size: BoxSize,
}

impl BoxHeader {
    const MAX_SIZE: usize = (4 + 8) + (4 + 16);

    /// TODO: doc
    pub fn new_variable_size(box_type: BoxType) -> Self {
        Self {
            box_type,
            box_size: BoxSize::VARIABLE_SIZE,
        }
    }

    /// TODO: doc
    pub fn finalize_box_size(mut self, box_bytes: &[u8]) -> Result2<Self> {
        if self.box_size != BoxSize::VARIABLE_SIZE {
            return Err(Error2::invalid_input(
                "box_size must be VARIABLE_SIZE before finalization",
            ));
        }

        self.box_size = BoxSize::with_payload_size(self.box_type, box_bytes.len() as u64);
        if !matches!(self.box_size, BoxSize::U32(_)) {
            // ヘッダーのサイズに変更があると box_bytes 全体のレイアウトが変わってしまうのでエラーにする
            return Err(Error2::invalid_input(
                "box payload too large: header size would require U64, making layout inconsistent",
            ));
        }

        Ok(self)
    }

    // TODO: remove
    /// ボックスへの参照を受け取って、対応するヘッダーを作成する
    pub fn from_box<B: BaseBox>(b: &B) -> Self {
        let box_type = b.box_type();
        todo!()
        //let box_size = b.box_size();
        //Self { box_type, box_size }
    }

    /// ヘッダーをエンコードした際のバイト数を返す
    pub fn external_size(self) -> usize {
        self.box_type.external_size() + self.box_size.external_size()
    }

    /// このヘッダーに対応するボックスのペイロード部分をデコードするためのリーダーを引数にして、指定された関数を呼び出す
    pub fn with_box_payload_reader<T, R: Read, F>(self, reader: R, f: F) -> Result<T>
    where
        F: FnOnce(&mut Take<R>) -> Result<T>,
    {
        let mut reader = if self.box_size.get() == 0 {
            reader.take(u64::MAX)
        } else {
            let payload_size = self
                .box_size
                .get()
                .checked_sub(self.external_size() as u64)
                .ok_or_else(|| {
                    Error::invalid_data(&format!(
                        "Too small box size: actual={}, expected={} or more",
                        self.box_size.get(),
                        self.external_size()
                    ))
                    .with_box_type(self.box_type)
                })?;
            reader.take(payload_size)
        };

        let value = f(&mut reader).map_err(|e| e.with_box_type(self.box_type))?;
        if reader.limit() != 0 {
            return Err(Error::invalid_data(&format!(
                "Unconsumed {} bytes at the end of the box '{}'",
                reader.limit(),
                self.box_type
            ))
            .with_box_type(self.box_type));
        }
        Ok(value)
    }

    /// ボックスのヘッダー部分を先読みする
    ///
    /// 返り値に含まれるリーダーには、ボックスのヘッダー部分のバイト列も含まれる
    pub fn peek<R: Read>(reader: R) -> Result<(Self, impl Read)> {
        let mut reader = PeekReader::<_, { BoxHeader::MAX_SIZE }>::new(reader);
        let header = BoxHeader::decode(&mut reader)?;
        Ok((header, reader.into_reader()))
    }
}

impl Encode2 for BoxHeader {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;

        let large_size = match self.box_size {
            BoxSize::U32(size) => {
                offset += size.encode2(&mut buf[offset..])?;
                None
            }
            BoxSize::U64(size) => {
                offset += 1u32.encode2(&mut buf[offset..])?;
                Some(size)
            }
        };

        match self.box_type {
            BoxType::Normal(ty) => {
                offset += ty.encode2(&mut buf[offset..])?;
            }
            BoxType::Uuid(ty) => {
                offset += b"uuid".encode2(&mut buf[offset..])?;
                offset += ty.encode2(&mut buf[offset..])?;
            }
        }

        if let Some(large_size) = large_size {
            offset += large_size.encode2(&mut buf[offset..])?;
        }

        Ok(offset)
    }
}

impl Decode for BoxHeader {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let box_size = u32::decode(&mut reader)?;

        let mut box_type = [0; 4];
        reader.read_exact(&mut box_type)?;

        let box_type = if box_type == [b'u', b'u', b'i', b'd'] {
            let mut box_type = [0; 16];
            reader.read_exact(&mut box_type)?;
            BoxType::Uuid(box_type)
        } else {
            BoxType::Normal(box_type)
        };

        let box_size = if box_size == 1 {
            BoxSize::U64(u64::decode(reader)?)
        } else {
            BoxSize::U32(box_size)
        };
        if box_size.get() != 0
            && box_size.get() < (box_size.external_size() + box_type.external_size()) as u64
        {
            return Err(Error::invalid_data(&format!(
                "Too small box size: actual={}, expected={} or more",
                box_size.get(),
                box_size.external_size() + box_type.external_size()
            ))
            .with_box_type(box_type));
        };

        Ok(Self { box_type, box_size })
    }
}

/// [`FullBox`] に共通のヘッダー
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullBoxHeader {
    /// バージョン
    pub version: u8,

    /// フラグ
    pub flags: FullBoxFlags,
}

impl FullBoxHeader {
    /// フルボックスへの参照を受け取って、対応するヘッダーを作成する
    pub fn from_box<B: FullBox>(b: &B) -> Self {
        Self {
            version: b.full_box_version(),
            flags: b.full_box_flags(),
        }
    }
}

impl Encode2 for FullBoxHeader {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += self.version.encode2(&mut buf[offset..])?;
        offset += self.flags.encode2(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl Decode for FullBoxHeader {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        Ok(Self {
            version: Decode::decode(&mut reader)?,
            flags: Decode::decode(reader)?,
        })
    }
}

/// [`FullBox`] のヘッダー部分に含まれるビットフラグ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullBoxFlags(u32);

impl FullBoxFlags {
    /// 空のビットフラグを作成する
    pub const fn empty() -> Self {
        Self(0)
    }

    /// [`u32`] を受け取って、対応するビットフラグを作成する
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }

    /// `(ビット位置、フラグがセットされているかどうか)` のイテレーターを受け取って、対応するビットフラグを作成する
    pub fn from_flags<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (usize, bool)>,
    {
        let flags = iter.into_iter().filter(|x| x.1).map(|x| 1 << x.0).sum();
        Self(flags)
    }

    /// このビットフラグに対応する [`u32`] 値を返す
    pub const fn get(self) -> u32 {
        self.0
    }

    /// 指定されたビット位置のフラグがセットされているかどうかを判定する
    pub const fn is_set(self, i: usize) -> bool {
        (self.0 & (1 << i)) != 0
    }
}

impl Encode2 for FullBoxFlags {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        self.0.to_be_bytes()[1..].encode2(buf)
    }
}

impl Decode for FullBoxFlags {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf[1..])?;
        Ok(Self(u32::from_be_bytes(buf)))
    }
}

/// [`BaseBox`] のサイズ
///
/// ボックスのサイズは原則として、ヘッダー部分とペイロード部分のサイズを足した値となる。
/// ただし、MP4 ファイルの末尾にあるボックスについてはサイズを 0 とすることで、ペイロードが可変長（追記可能）なボックスとして扱うことが可能となっている。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum BoxSize {
    U32(u32),
    U64(u64),
}

impl BoxSize {
    /// ファイル末尾に位置する可変長のボックスを表すための特別な値
    pub const VARIABLE_SIZE: Self = Self::U32(0);

    /// ボックス種別とペイロードサイズを受け取って、対応する [`BoxSize`] インスタンスを作成する
    pub fn with_payload_size(box_type: BoxType, payload_size: u64) -> Self {
        let mut size = 4 + box_type.external_size() as u64 + payload_size;
        if let Ok(size) = u32::try_from(size) {
            Self::U32(size)
        } else {
            size += 8;
            Self::U64(size)
        }
    }

    /// ボックスのサイズの値を取得する
    pub const fn get(self) -> u64 {
        match self {
            BoxSize::U32(v) => v as u64,
            BoxSize::U64(v) => v,
        }
    }

    /// [`BoxHeader`] 内のサイズフィールドをエンコードする際に必要となるバイト数を返す
    pub const fn external_size(self) -> usize {
        match self {
            BoxSize::U32(_) => 4,
            BoxSize::U64(_) => 4 + 8,
        }
    }
}

/// [`BaseBox`] の種別
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoxType {
    /// 四文字で表現される通常のボックス種別
    Normal([u8; 4]),

    /// UUID 形式のボックス種別
    Uuid([u8; 16]),
}

impl BoxType {
    /// 種別を表すバイト列を返す
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            BoxType::Normal(ty) => &ty[..],
            BoxType::Uuid(ty) => &ty[..],
        }
    }

    /// [`BoxHeader`] 内のボックス種別フィールドをエンコードする際に必要となるバイト数を返す
    pub const fn external_size(self) -> usize {
        if matches!(self, Self::Normal(_)) {
            4
        } else {
            4 + 16
        }
    }

    /// 自分が `expected` と同じ種別であるかをチェックする
    pub fn expect(self, expected: Self) -> Result<()> {
        if self == expected {
            Ok(())
        } else {
            Err(Error::invalid_data(&format!(
                "Expected box type `{expected}`, but got `{self}`"
            )))
        }
    }
}

impl core::fmt::Debug for BoxType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BoxType::Normal(ty) => {
                if let Ok(ty) = core::str::from_utf8(ty) {
                    f.debug_tuple("BoxType").field(&ty).finish()
                } else {
                    f.debug_tuple("BoxType").field(ty).finish()
                }
            }
            BoxType::Uuid(ty) => f.debug_tuple("BoxType").field(ty).finish(),
        }
    }
}

impl core::fmt::Display for BoxType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let BoxType::Normal(ty) = self
            && let Ok(ty) = core::str::from_utf8(&ty[..])
        {
            return write!(f, "{ty}");
        }
        write!(f, "{:?}", self.as_bytes())
    }
}

/// MP4 ファイル内で使われる時刻形式（1904/1/1 からの経過秒数）
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mp4FileTime(u64);

impl Mp4FileTime {
    /// 1904/1/1 からの経過秒数を引数にとって [`Mp4FileTime`] インスタンスを作成する
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    /// 1904/1/1 からの経過秒数を返す
    pub const fn as_secs(self) -> u64 {
        self.0
    }

    /// UNIX EPOCH (1970-01-01 00:00:00 UTC) を起点とした経過時間を受け取って、対応する [`Mp4FileTime`] インスタンスを作成する
    pub const fn from_unix_time(unix_time: Duration) -> Self {
        let delta = 2082844800; // 1904/1/1 から 1970/1/1 までの経過秒数
        let unix_time_secs = unix_time.as_secs();
        Self::from_secs(unix_time_secs + delta)
    }
}

/// 固定小数点数
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FixedPointNumber<I, F = I> {
    /// 整数部
    pub integer: I,

    /// 小数部
    pub fraction: F,
}

impl<I, F> FixedPointNumber<I, F> {
    /// 整数部と小数部を受け取って固定小数点数を返す
    pub const fn new(integer: I, fraction: F) -> Self {
        Self { integer, fraction }
    }
}

impl<I: Encode2, F: Encode2> Encode2 for FixedPointNumber<I, F> {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += self.integer.encode2(&mut buf[offset..])?;
        offset += self.fraction.encode2(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl<I: Decode, F: Decode> Decode for FixedPointNumber<I, F> {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        Ok(Self {
            integer: I::decode(&mut reader)?,
            fraction: F::decode(reader)?,
        })
    }
}

/// null 終端の UTF-8 文字列
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Utf8String(String);

impl Utf8String {
    /// 空文字列
    pub const EMPTY: Self = Utf8String(String::new());

    /// 終端の null を含まない文字列を受け取って [`Utf8String`] インスタンスを作成する
    ///
    /// 引数の文字列内の null 文字が含まれている場合には [`None`] が返される
    pub fn new(s: &str) -> Option<Self> {
        if s.as_bytes().contains(&0) {
            return None;
        }
        Some(Self(s.to_owned()))
    }

    /// このインスタンスが保持する、null 終端部分を含まない文字列を返す
    pub fn get(&self) -> &str {
        &self.0
    }

    /// このインスタンスを、null 終端部分を含むバイト列へと変換する
    pub fn into_null_terminated_bytes(self) -> Vec<u8> {
        let mut v = self.0.into_bytes();
        v.push(0);
        v
    }
}

impl Encode2 for Utf8String {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += self.0.as_bytes().encode2(&mut buf[offset..])?;
        offset += 0u8.encode2(&mut buf[offset..])?;
        Ok(offset)
    }
}

impl Decode for Utf8String {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let mut bytes = Vec::new();
        loop {
            let b = u8::decode(&mut reader)?;
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        let s = String::from_utf8(bytes).map_err(|e| {
            Error::invalid_data(&format!("Invalid UTF-8 string: {:?}", e.as_bytes()))
        })?;
        Ok(Self(s))
    }
}

/// `A` か `B` のどちらかの値を保持する列挙型
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A: BaseBox, B: BaseBox> Either<A, B> {
    fn inner_box(&self) -> &dyn BaseBox {
        match self {
            Self::A(x) => x,
            Self::B(x) => x,
        }
    }
}

impl<A: BaseBox, B: BaseBox> BaseBox for Either<A, B> {
    fn box_type(&self) -> BoxType {
        self.inner_box().box_type()
    }

    fn is_unknown_box(&self) -> bool {
        self.inner_box().is_unknown_box()
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.inner_box().children()
    }
}

/// 任意のビット数の非負の整数を表現するための型
///
/// - `T`: 数値の内部的な型。 最低限 `BITS` 分の数値を表現可能な型である必要がある。
/// - `BITS`: 数値のビット数
/// - `OFFSET`: 一つの `T` に複数の [`Uint`] 値がパックされる場合の、この数値のオフセット位置（ビット数）
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uint<T, const BITS: u32, const OFFSET: u32 = 0>(T);

impl<T, const BITS: u32, const OFFSET: u32> Uint<T, BITS, OFFSET>
where
    T: Shr<u32, Output = T>
        + Shl<u32, Output = T>
        + BitAnd<Output = T>
        + Sub<Output = T>
        + From<u8>,
{
    /// 指定された数値を受け取ってインスタンスを作成する
    pub const fn new(v: T) -> Self {
        Self(v)
    }

    /// このインスタンスが表現する整数値を返す
    pub fn get(self) -> T {
        self.0
    }

    /// `T` が保持するビット列の `OFFSET` 位置から `BITS` 分のビット列に対応する整数値を返す
    pub fn from_bits(v: T) -> Self {
        Self((v >> OFFSET) & ((T::from(1) << BITS) - T::from(1)))
    }

    /// このインスタンスに対応する `T` 内のビット列を返す
    ///
    /// なお `OFFSET` が `0` の場合には、このメソッドは [`Uint::get()`] と等価である
    pub fn to_bits(self) -> T {
        self.0 << OFFSET
    }
}
