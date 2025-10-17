//! ISO_IEC_14496-1 で定義されているディスクリプター群
#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec, vec::Vec};

use crate::{
    Decode, Encode, Encode2, Error, Error2, Result, Result2, Uint,
    io::{Read, Write},
};

/// [ISO_IEC_14496-1] ES_Descriptor class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct EsDescriptor {
    pub es_id: u16,
    pub stream_priority: Uint<u8, 5>,
    pub depends_on_es_id: Option<u16>,
    pub url_string: Option<String>,
    pub ocr_es_id: Option<u16>,
    pub dec_config_descr: DecoderConfigDescriptor,
    pub sl_config_descr: SlConfigDescriptor,
}

impl EsDescriptor {
    const TAG: u8 = 3; // ES_DescrTag

    /// [`EsDescriptor::es_id`] の実質的な最小値 (0 は予約されている）
    pub const MIN_ES_ID: u16 = 1;

    /// [`EsDescriptor::stream_priority`] で一番優先度が低くなる値
    pub const LOWEST_STREAM_PRIORITY: Uint<u8, 5> = Uint::new(0);
}

impl Decode for EsDescriptor {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let (tag, _size) = decode_tag_and_size(&mut reader)?;
        if tag != Self::TAG {
            return Err(Error::invalid_data(&format!(
                "Unexpected descriptor tag: expected={}, actual={tag}",
                Self::TAG
            )));
        }

        let es_id = u16::decode(&mut reader)?;

        let b = u8::decode(&mut reader)?;
        let stream_dependence_flag: Uint<u8, 1, 7> = Uint::from_bits(b);
        let url_flag: Uint<u8, 1, 6> = Uint::from_bits(b);
        let ocr_stream_flag: Uint<u8, 1, 5> = Uint::from_bits(b);
        let stream_priority = Uint::from_bits(b);

        let depends_on_es_id = (stream_dependence_flag.get() == 1)
            .then(|| u16::decode(&mut reader))
            .transpose()?;

        let url_string = if url_flag.get() == 1 {
            let len = u8::decode(&mut reader)? as u64;
            let mut s = String::new();
            (&mut reader).take(len).read_to_string(&mut s)?;
            Some(s)
        } else {
            None
        };

        let ocr_es_id = (ocr_stream_flag.get() == 1)
            .then(|| u16::decode(&mut reader))
            .transpose()?;

        let dec_config_descr = DecoderConfigDescriptor::decode(&mut reader)?;
        let sl_config_descr = SlConfigDescriptor::decode(&mut reader)?;

        Ok(Self {
            es_id,
            stream_priority,
            depends_on_es_id,
            url_string,
            ocr_es_id,
            dec_config_descr,
            sl_config_descr,
        })
    }
}

impl Encode for EsDescriptor {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        let mut payload = Vec::new();

        self.es_id.encode(&mut payload)?;
        (Uint::<u8, 1, 7>::new(self.depends_on_es_id.is_some() as u8).to_bits()
            | Uint::<u8, 1, 6>::new(self.url_string.is_some() as u8).to_bits()
            | Uint::<u8, 1, 5>::new(self.ocr_es_id.is_some() as u8).to_bits()
            | self.stream_priority.to_bits())
        .encode(&mut payload)?;
        if let Some(v) = self.depends_on_es_id {
            v.encode(&mut payload)?;
        }
        if let Some(v) = &self.url_string {
            (v.len() as u8).encode(&mut payload)?;
            payload.write_all(v.as_bytes())?;
        }
        if let Some(v) = self.ocr_es_id {
            v.encode(&mut payload)?;
        }
        self.dec_config_descr.encode(&mut payload)?;
        self.sl_config_descr.encode(&mut payload)?;

        encode_tag_and_size(&mut writer, Self::TAG, payload.len())?;
        writer.write_all(&payload)?;

        Ok(())
    }
}

impl Encode2 for EsDescriptor {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += self.es_id.encode2(&mut buf[offset..])?;
        offset += (Uint::<u8, 1, 7>::new(self.depends_on_es_id.is_some() as u8).to_bits()
            | Uint::<u8, 1, 6>::new(self.url_string.is_some() as u8).to_bits()
            | Uint::<u8, 1, 5>::new(self.ocr_es_id.is_some() as u8).to_bits()
            | self.stream_priority.to_bits())
        .encode2(&mut buf[offset..])?;

        if let Some(v) = self.depends_on_es_id {
            offset += v.encode2(&mut buf[offset..])?;
        }
        if let Some(v) = &self.url_string {
            offset += (v.len() as u8).encode2(&mut buf[offset..])?;
            offset += v.as_bytes().encode2(&mut buf[offset..])?;
        }
        if let Some(v) = self.ocr_es_id {
            offset += v.encode2(&mut buf[offset..])?;
        }

        offset += self.dec_config_descr.encode2(&mut buf[offset..])?;
        offset += self.sl_config_descr.encode2(&mut buf[offset..])?;

        encode_tag_and_payload(buf, Self::TAG, offset)
    }
}

/// [ISO_IEC_14496-1] DecoderConfigDescriptor class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct DecoderConfigDescriptor {
    pub object_type_indication: u8,
    pub stream_type: Uint<u8, 6, 2>,
    pub up_stream: Uint<u8, 1, 1>,
    pub buffer_size_db: Uint<u32, 24>,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,
    pub dec_specific_info: DecoderSpecificInfo,
}

impl DecoderConfigDescriptor {
    const TAG: u8 = 4; // DecoderConfigDescrTag

    /// AAC 用の [`DecoderConfigDescriptor::object_type_indication`] の値
    pub const OBJECT_TYPE_INDICATION_AUDIO_ISO_IEC_14496_3: u8 = 0x40;

    /// 音声用の [`DecoderConfigDescriptor::stream_type`] の値
    pub const STREAM_TYPE_AUDIO: Uint<u8, 6, 2> = Uint::new(0x05);

    /// 通常の再生用メディアファイル向けの [`DecoderConfigDescriptor::up_stream`] の値
    pub const UP_STREAM_FALSE: Uint<u8, 1, 1> = Uint::new(0);
}

impl Decode for DecoderConfigDescriptor {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let (tag, _size) = decode_tag_and_size(&mut reader)?;
        if tag != Self::TAG {
            return Err(Error::invalid_data(&format!(
                "Unexpected descriptor tag: expected={}, actual={tag}",
                Self::TAG
            )));
        }

        let object_type_indication = u8::decode(&mut reader)?;

        let b = u8::decode(&mut reader)?;
        let stream_type = Uint::from_bits(b);
        let up_stream = Uint::from_bits(b);

        let mut buf = [0; 4];
        reader.read_exact(&mut buf[1..])?;
        let buffer_size_db = Uint::from_bits(u32::from_be_bytes(buf));

        let max_bitrate = u32::decode(&mut reader)?;
        let avg_bitrate = u32::decode(&mut reader)?;

        let dec_specific_info = DecoderSpecificInfo::decode(&mut reader)?;
        Ok(Self {
            object_type_indication,
            stream_type,
            up_stream,
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
            dec_specific_info,
        })
    }
}

impl Encode for DecoderConfigDescriptor {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        let mut payload = Vec::new();

        self.object_type_indication.encode(&mut payload)?;
        (self.stream_type.to_bits() | self.up_stream.to_bits() | Uint::<u8, 1>::new(1).to_bits())
            .encode(&mut payload)?;
        payload.write_all(&self.buffer_size_db.to_bits().to_be_bytes()[1..])?;
        self.max_bitrate.encode(&mut payload)?;
        self.avg_bitrate.encode(&mut payload)?;
        self.dec_specific_info.encode(&mut payload)?;

        encode_tag_and_size(&mut writer, Self::TAG, payload.len())?;
        writer.write_all(&payload)?;

        Ok(())
    }
}

impl Encode2 for DecoderConfigDescriptor {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;

        offset += self.object_type_indication.encode2(&mut buf[offset..])?;
        offset += (self.stream_type.to_bits()
            | self.up_stream.to_bits()
            | Uint::<u8, 1>::new(1).to_bits())
        .encode2(&mut buf[offset..])?;
        offset += self.buffer_size_db.to_bits().to_be_bytes()[1..].encode2(&mut buf[offset..])?;
        offset += self.max_bitrate.encode2(&mut buf[offset..])?;
        offset += self.avg_bitrate.encode2(&mut buf[offset..])?;
        offset += self.dec_specific_info.encode2(&mut buf[offset..])?;

        encode_tag_and_payload(buf, Self::TAG, offset)
    }
}

/// [ISO_IEC_14496-1] DecoderSpecificInfo class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct DecoderSpecificInfo {
    pub payload: Vec<u8>,
}

impl DecoderSpecificInfo {
    const TAG: u8 = 5; // DecSpecificInfoTag
}

impl Decode for DecoderSpecificInfo {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let (tag, size) = decode_tag_and_size(&mut reader)?;
        if tag != Self::TAG {
            return Err(Error::invalid_data(&format!(
                "Unexpected descriptor tag: expected={}, actual={tag}",
                Self::TAG
            )));
        }

        let mut payload = vec![0; size];
        reader.read_exact(&mut payload)?;

        Ok(Self { payload })
    }
}

impl Encode for DecoderSpecificInfo {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        encode_tag_and_size(&mut writer, Self::TAG, self.payload.len())?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl Encode2 for DecoderSpecificInfo {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let mut offset = 0;
        offset += encode_tag_and_size2(
            buf.get_mut(offset..)
                .ok_or_else(Error2::insufficient_buffer)?,
            Self::TAG,
            self.payload.len(),
        )?;
        let size = offset + self.payload.len();
        Error2::check_buffer_size(size, buf)?;
        buf[offset..size].copy_from_slice(&self.payload);
        Ok(size)
    }
}

/// [ISO_IEC_14496-1] SLConfigDescriptor class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SlConfigDescriptor;

impl SlConfigDescriptor {
    const TAG: u8 = 6; // SLConfigDescrTag
}

impl Decode for SlConfigDescriptor {
    fn decode<R: Read>(mut reader: R) -> Result<Self> {
        let (tag, _size) = decode_tag_and_size(&mut reader)?;
        if tag != Self::TAG {
            return Err(Error::invalid_data(&format!(
                "Unexpected descriptor tag: expected={}, actual={tag}",
                Self::TAG
            )));
        }

        let predefined = u8::decode(&mut reader)?;
        if predefined != 2 {
            // MP4 では 2 が主に使われていそうなので、いったんそれ以外は未対応にしておいて、
            // 必要に応じて随時対応を追加していく
            return Err(Error::unsupported(&format!(
                "Unsupported `SLConfigDescriptor.predefined` value: {predefined}"
            )));
        }

        // predefined == 2 の場合には、追加の処理は不要

        Ok(Self)
    }
}

impl Encode for SlConfigDescriptor {
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        let predefined = 2;
        let payload = [predefined];

        encode_tag_and_size(&mut writer, Self::TAG, payload.len())?;
        writer.write_all(&payload)?;

        Ok(())
    }
}

impl Encode2 for SlConfigDescriptor {
    fn encode2(&self, buf: &mut [u8]) -> Result2<usize> {
        let predefined = 2;
        let payload = [predefined];
        let mut offset = 0;
        offset += encode_tag_and_size2(
            buf.get_mut(offset..)
                .ok_or_else(Error2::insufficient_buffer)?,
            Self::TAG,
            payload.len(),
        )?;
        let required = offset + payload.len();
        Error2::check_buffer_size(required, buf)?;
        buf[offset..required].copy_from_slice(&payload);
        Ok(required)
    }
}

fn decode_tag_and_size<R: Read>(mut reader: R) -> Result<(u8, usize)> {
    let tag = u8::decode(&mut reader)?;

    let mut size = 0;
    let mut has_next_byte = true;
    while has_next_byte {
        let b = u8::decode(&mut reader)?;
        has_next_byte = Uint::<u8, 1, 7>::from_bits(b).get() == 1;
        size = (size << 7) | Uint::<u8, 7>::from_bits(b).get() as usize
    }

    Ok((tag, size))
}

fn encode_tag_and_size<W: Write>(mut writer: W, tag: u8, mut size: usize) -> Result<()> {
    writer.write_all(&[tag])?;

    let mut buf = Vec::new();
    for i in 0.. {
        let mut b = (size & 0b0111_1111) as u8;
        size >>= 7;

        if i > 0 {
            b |= 0b1000_0000;
        }
        buf.push(b);

        if size == 0 {
            break;
        }
    }
    buf.reverse(); // リトルエンディアンからビッグエンディアンにする
    writer.write_all(&buf)?;

    Ok(())
}

// buf の先頭にペイロードが格納されている前提
fn encode_tag_and_payload(buf: &mut [u8], tag: u8, payload_size: usize) -> Result2<usize> {
    let mut header_buf = [0; 64];
    let header_size = encode_tag_and_size2(&mut header_buf, tag, payload_size)?;
    Error2::check_buffer_size(header_size + payload_size, buf)?;
    buf.copy_within(..payload_size, header_size);
    buf[..header_size].copy_from_slice(&header_buf[..header_size]);
    Ok(header_size + payload_size)
}

fn encode_tag_and_size2(buf: &mut [u8], tag: u8, mut size: usize) -> Result2<usize> {
    let mut offset = 0;
    offset += tag.encode2(&mut buf[offset..])?;

    let mut size_bytes = Vec::new();
    for i in 0.. {
        let mut b = (size & 0b0111_1111) as u8;
        size >>= 7;

        if i > 0 {
            b |= 0b1000_0000;
        }
        size_bytes.push(b);

        if size == 0 {
            break;
        }
    }
    size_bytes.reverse(); // リトルエンディアンからビッグエンディアンにする

    offset += size_bytes.encode2(&mut buf[offset..])?;
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_and_size() {
        let mut buf = Vec::new();
        encode_tag_and_size(&mut buf, 12, 123456).unwrap();

        let (tag, size) = decode_tag_and_size(&buf[..]).unwrap();
        assert_eq!(tag, 12);
        assert_eq!(size, 123456);
    }
}
