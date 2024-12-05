//! ISO_IEC_14496-1 で定義されているディスクリプター群
use std::io::{Read, Write};

use crate::{Decode, Encode, Error, Result, Uint};

/// [ISO_IEC_14496-1] SLConfigDescriptor class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
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
