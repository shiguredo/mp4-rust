use std::{
    io::{Read, Write},
    ops::{BitAnd, Shl, Shr, Sub},
    time::Duration,
};

use crate::{
    boxes::{FtypBox, RootBox},
    io::PeekReader,
    Decode, Encode, Error, Result,
};

/// 全てのボックスが実装するトレイト
///
/// 本来なら `Box` という名前が適切だが、それだと標準ライブラリの [`std::boxed::Box`] と名前が
/// 衝突してしまうので、それを避けるために `BaseBox` としている
pub trait BaseBox {
    /// ボックスの種別
    fn box_type(&self) -> BoxType;

    /// ボックスのサイズ
    ///
    /// サイズが可変長になる可能性がある `mdat` ボックス以外はデフォルト実装のままで問題ない
    fn box_size(&self) -> BoxSize {
        BoxSize::with_payload_size(self.box_type(), self.box_payload_size())
    }

    /// ボックスのペイロードのバイト数
    fn box_payload_size(&self) -> u64;

    /// 未知のボックスかどうか
    ///
    /// 基本的には `false` を返すデフォルト実装のままで問題ないが、
    /// [`UnknownBox`](crate::boxes::UnknownBox) を含む `enum` を定義する場合には、
    /// 独自の実装が必要となる
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
        std::iter::empty()
            .chain(std::iter::once(&self.ftyp_box).map(as_box_object))
            .chain(self.boxes.iter().map(as_box_object))
    }
}

impl<B: BaseBox + Decode> Decode for Mp4File<B> {
    fn decode<R: Read>(mut reader: &mut R) -> Result<Self> {
        let ftyp_box = FtypBox::decode(reader)?;

        let mut boxes = Vec::new();
        let mut buf = [0];
        while reader.read(&mut buf)? != 0 {
            let b = B::decode(&mut buf.chain(&mut reader))?;
            boxes.push(b);
        }
        Ok(Self { ftyp_box, boxes })
    }
}

impl<B: BaseBox + Encode> Encode for Mp4File<B> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.ftyp_box.encode(writer)?;

        for b in &self.boxes {
            b.encode(writer)?;
        }
        Ok(())
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

    /// ボックスへの参照を受け取って、対応するヘッダーを作成する
    pub fn from_box<B: BaseBox>(b: &B) -> Self {
        let box_type = b.box_type();
        let box_size = b.box_size();
        Self { box_type, box_size }
    }

    /// ヘッダーをエンコードした際のバイト数を返す
    pub fn external_size(self) -> usize {
        self.box_type.external_size() + self.box_size.external_size()
    }

    /// このヘッダーに対応するボックスのペイロード部分をデコードするためのリーダーを引数にして、指定された関数を呼び出す
    pub fn with_box_payload_reader<T, R: Read, F>(self, reader: R, f: F) -> Result<T>
    where
        F: FnOnce(&mut std::io::Take<R>) -> Result<T>,
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
                })?;
            reader.take(payload_size)
        };

        let value = f(&mut reader)?;
        if reader.limit() != 0 {
            return Err(Error::invalid_data(&format!(
                "Unconsumed {} bytes at the end of the box {:?}",
                reader.limit(),
                self.box_type
            )));
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

impl Encode for BoxHeader {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        let large_size = self.box_size.get() > u32::MAX as u64;
        if large_size {
            1u32.encode(writer)?;
        } else {
            (self.box_size.get() as u32).encode(writer)?;
        }

        match self.box_type {
            BoxType::Normal(ty) => {
                writer.write_all(&ty)?;
            }
            BoxType::Uuid(ty) => {
                writer.write_all("uuid".as_bytes())?;
                writer.write_all(&ty)?;
            }
        }

        if large_size {
            self.box_size.get().encode(writer)?;
        }

        Ok(())
    }
}

impl Decode for BoxHeader {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut box_size = u32::decode(reader)? as u64;

        let mut box_type = [0; 4];
        reader.read_exact(&mut box_type)?;

        let box_type = if box_type == [b'u', b'u', b'i', b'd'] {
            let mut box_type = [0; 16];
            reader.read_exact(&mut box_type)?;
            BoxType::Uuid(box_type)
        } else {
            BoxType::Normal(box_type)
        };

        if box_size == 1 {
            box_size = u64::decode(reader)?;
        }
        let box_size = BoxSize::new(box_type, box_size).ok_or_else(|| {
            Error::invalid_data(&format!(
                "Too small box size: actual={}, expected={} or more",
                box_size,
                4 + box_type.external_size()
            ))
        })?;

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

impl Encode for FullBoxHeader {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.version.encode(writer)?;
        self.flags.encode(writer)?;
        Ok(())
    }
}

impl Decode for FullBoxHeader {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            version: Decode::decode(reader)?,
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
    pub fn from_iter<I>(iter: I) -> Self
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

impl Encode for FullBoxFlags {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.0.to_be_bytes()[1..])?;
        Ok(())
    }
}

impl Decode for FullBoxFlags {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
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
pub struct BoxSize(u64);

impl BoxSize {
    /// ファイル末尾に位置する可変長のボックスを表すための特別な値
    pub const VARIABLE_SIZE: Self = Self(0);

    /// [`u64`] のサイズ値を受け取って、それが適切な場合には `Some(BoxSize)` が返される
    ///
    /// `box_size` の値が、指定されたボックス種別を保持するために必要な最小サイズを下回っている場合には [`None`] が返される
    pub fn new(box_type: BoxType, box_size: u64) -> Option<Self> {
        if box_size == 0 {
            return Some(Self(0));
        }

        if box_size < 4 + box_type.external_size() as u64 {
            None
        } else {
            Some(Self(box_size))
        }
    }

    /// ボックス種別とペイロードサイズを受け取って、対応する [`BoxSize`] インスタンスを作成する
    pub const fn with_payload_size(box_type: BoxType, payload_size: u64) -> Self {
        let mut size = 4 + box_type.external_size() as u64 + payload_size;
        if size > u32::MAX as u64 {
            size += 8;
        }
        Self(size)
    }

    /// ボックスのサイズの値を取得する
    pub const fn get(self) -> u64 {
        self.0
    }

    /// [`BoxHeader`] 内のサイズフィールドをエンコードする際に必要となるバイト数を返す
    pub const fn external_size(self) -> usize {
        if self.0 > u32::MAX as u64 {
            4 + 8
        } else {
            4
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoxType {
    Normal([u8; 4]),
    Uuid([u8; 16]),
}

impl BoxType {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            BoxType::Normal(ty) => &ty[..],
            BoxType::Uuid(ty) => &ty[..],
        }
    }

    pub const fn external_size(self) -> usize {
        if matches!(self, Self::Normal(_)) {
            4
        } else {
            4 + 16
        }
    }

    pub fn expect(self, expected: Self) -> Result<()> {
        if self == expected {
            Ok(())
        } else {
            Err(Error::invalid_data(&format!(
                "Expected box type {:?}, but got {:?}",
                expected, self
            )))
        }
    }
}

impl std::fmt::Debug for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoxType::Normal(ty) => {
                if let Ok(ty) = std::str::from_utf8(ty) {
                    f.debug_tuple("BoxType").field(&ty).finish()
                } else {
                    f.debug_tuple("BoxType").field(ty).finish()
                }
            }
            BoxType::Uuid(ty) => f.debug_tuple("BoxType").field(ty).finish(),
        }
    }
}

impl std::fmt::Display for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let BoxType::Normal(ty) = self {
            if let Ok(ty) = std::str::from_utf8(&ty[..]) {
                return write!(f, "{ty}");
            }
        }
        write!(f, "{:?}", self.as_bytes())
    }
}

/// 1904/1/1 からの経過秒数
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mp4FileTime(u64);

impl Mp4FileTime {
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    pub const fn as_secs(self) -> u64 {
        self.0
    }

    pub const fn from_unix_time(unix_time: Duration) -> Self {
        let delta = 2082844800; // 1904/1/1 から 1970/1/1 までの経過秒数
        let unix_time_secs = unix_time.as_secs();
        Self::from_secs(unix_time_secs + delta)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FixedPointNumber<I, F = I> {
    pub integer: I,
    pub fraction: F,
}

impl<I, F> FixedPointNumber<I, F> {
    pub const fn new(integer: I, fraction: F) -> Self {
        Self { integer, fraction }
    }
}

impl<I: Encode, F: Encode> Encode for FixedPointNumber<I, F> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.integer.encode(writer)?;
        self.fraction.encode(writer)?;
        Ok(())
    }
}

impl<I: Decode, F: Decode> Decode for FixedPointNumber<I, F> {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            integer: I::decode(reader)?,
            fraction: F::decode(reader)?,
        })
    }
}

// エンコード時には終端 null が付与される文字列
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Utf8String(String);

impl Utf8String {
    pub fn new(s: &str) -> Option<Self> {
        if s.as_bytes().contains(&0) {
            return None;
        }
        Some(Self(s.to_owned()))
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl Encode for Utf8String {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(self.0.as_bytes())?;
        writer.write_all(&[0])?;
        Ok(())
    }
}

impl Decode for Utf8String {
    fn decode<R: Read>(reader: &mut R) -> Result<Self> {
        let mut bytes = Vec::new();
        loop {
            let b = u8::decode(reader)?;
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

    fn box_size(&self) -> BoxSize {
        self.inner_box().box_size()
    }

    fn box_payload_size(&self) -> u64 {
        self.inner_box().box_payload_size()
    }

    fn is_unknown_box(&self) -> bool {
        self.inner_box().is_unknown_box()
    }

    fn children<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &'a dyn BaseBox>> {
        self.inner_box().children()
    }
}

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
    // TODO: rename
    pub fn from_bits(v: T) -> Self {
        Self((v >> OFFSET) & (T::from(1) << BITS) - T::from(1))
    }

    pub fn to_bits(self) -> T {
        self.0 << OFFSET
    }

    pub fn get(self) -> T {
        self.0
    }
}
