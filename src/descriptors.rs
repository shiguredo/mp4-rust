//! ISO_IEC_14496-1 で定義されているディスクリプター群
use std::io::{Read, Write};

use crate::{Decode, Encode, Error, Result, Uint};

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
        (self.stream_type.to_bits() | self.up_stream.to_bits()).encode(&mut payload)?;
        writer.write_all(&self.buffer_size_db.to_bits().to_be_bytes()[1..])?;
        self.max_bitrate.encode(&mut writer)?;
        self.avg_bitrate.encode(&mut writer)?;
        self.dec_specific_info.encode(&mut writer)?;

        encode_tag_and_size(&mut writer, Self::TAG, payload.len())?;
        writer.write_all(&payload)?;

        Ok(())
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

    loop {
        let mut b = (size & 0b0111_1111) as u8;
        size >>= 7;

        if size != 0 {
            b |= 0b1000_0000;
        }
        writer.write_all(&[b])?;

        if size == 0 {
            break;
        }
    }

    Ok(())
}
