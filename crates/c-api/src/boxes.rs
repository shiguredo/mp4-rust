//! ../../../src/boxes.rs の（一部に対応する） C API を定義するためのモジュール
use shiguredo_mp4::Uint;

use crate::error::Mp4Error;

#[repr(C)]
pub enum Mp4SampleEntryKind {
    /// Unknown
    Unknown = 0,
    /// AVC1 (H.264)
    Avc1,
    /// HEV1 (H.265/HEVC)
    Hev1,
    /// VP08 (VP8)
    Vp08,
    /// VP09 (VP9)
    Vp09,
    /// AV01 (AV1)
    Av01,
    /// Opus
    Opus,
    /// MP4A (AAC)
    Mp4a,
}

pub enum Mp4SampleEntryOwned {
    Avc1 {
        inner: shiguredo_mp4::boxes::Avc1Box,
        sps_data: Vec<*const u8>,
        sps_sizes: Vec<u32>,
        pps_data: Vec<*const u8>,
        pps_sizes: Vec<u32>,
    },
}

impl Mp4SampleEntryOwned {
    pub fn to_mp4_sample_entry(&self) -> Mp4SampleEntry {
        match self {
            Self::Avc1 {
                inner,
                sps_data,
                sps_sizes,
                pps_data,
                pps_sizes,
            } => {
                let avc1 = Mp4SampleEntryAvc1 {
                    width: inner.visual.width,
                    height: inner.visual.height,
                    avc_profile_indication: inner.avcc_box.avc_profile_indication,
                    profile_compatibility: inner.avcc_box.profile_compatibility,
                    avc_level_indication: inner.avcc_box.avc_level_indication,
                    length_size_minus_one: inner.avcc_box.length_size_minus_one.get(),
                    sps_data: sps_data.as_ptr(),
                    sps_sizes: sps_sizes.as_ptr(),
                    sps_count: sps_data.len() as u32,
                    pps_data: pps_data.as_ptr(),
                    pps_sizes: pps_sizes.as_ptr(),
                    pps_count: pps_data.len() as u32,
                    is_chroma_format_present: inner.avcc_box.chroma_format.is_some(),
                    chroma_format: inner.avcc_box.chroma_format.map(|v| v.get()).unwrap_or(0),
                    is_bit_depth_luma_minus8_present: inner
                        .avcc_box
                        .bit_depth_luma_minus8
                        .is_some(),
                    bit_depth_luma_minus8: inner
                        .avcc_box
                        .bit_depth_luma_minus8
                        .map(|v| v.get())
                        .unwrap_or(0),
                    is_bit_depth_chroma_minus8_present: inner
                        .avcc_box
                        .bit_depth_chroma_minus8
                        .is_some(),
                    bit_depth_chroma_minus8: inner
                        .avcc_box
                        .bit_depth_chroma_minus8
                        .map(|v| v.get())
                        .unwrap_or(0),
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Avc1,
                    data: Mp4SampleEntryData { avc1 },
                }
            }
        }
    }
}

#[repr(C)]
pub union Mp4SampleEntryData {
    pub avc1: Mp4SampleEntryAvc1,
    //pub hev1: Mp4SampleEntryHev1,
    //pub vp08: Mp4SampleEntryVp08,
    //pub vp09: Mp4SampleEntryVp09,
    //pub av01: Mp4SampleEntryAv01,
    //pub opus: Mp4SampleEntryOpus,
    //pub mp4a: Mp4SampleEntryMp4a,
}

// TODO: Add a union for Mp4SampleEntryAvc1 and other codecs

#[repr(C)]
pub struct Mp4SampleEntry {
    pub kind: Mp4SampleEntryKind,
    pub data: Mp4SampleEntryData,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryAvc1 {
    pub width: u16,
    pub height: u16,

    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: u8,

    pub sps_data: *const *const u8,
    pub sps_sizes: *const u32,
    pub sps_count: u32,

    pub pps_data: *const *const u8,
    pub pps_sizes: *const u32,
    pub pps_count: u32,

    pub is_chroma_format_present: bool,
    pub chroma_format: u8,

    pub is_bit_depth_luma_minus8_present: bool,
    pub bit_depth_luma_minus8: u8,

    pub is_bit_depth_chroma_minus8_present: bool,
    pub bit_depth_chroma_minus8: u8,
}

impl Mp4SampleEntryAvc1 {
    pub fn to_sample_entry(&self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4Error> {
        // SPS / PPS リストをメモリから読み込む
        let mut sps_list = Vec::new();
        if self.sps_data.is_null() {
            return Err(Mp4Error::NullPointer);
        }
        if self.sps_count > 0 {
            unsafe {
                for i in 0..self.sps_count as usize {
                    let sps_ptr = *self.sps_data.add(i);
                    let sps_size = *self.sps_sizes.add(i) as usize;
                    if sps_ptr.is_null() {
                        return Err(Mp4Error::NullPointer);
                    }
                    sps_list.push(std::slice::from_raw_parts(sps_ptr, sps_size).to_vec());
                }
            }
        }

        let mut pps_list = Vec::new();
        if self.pps_data.is_null() {
            return Err(Mp4Error::NullPointer);
        }
        if self.pps_count > 0 {
            unsafe {
                for i in 0..self.pps_count as usize {
                    let pps_ptr = *self.pps_data.add(i);
                    let pps_size = *self.pps_sizes.add(i) as usize;
                    if pps_ptr.is_null() {
                        return Err(Mp4Error::NullPointer);
                    }
                    pps_list.push(std::slice::from_raw_parts(pps_ptr, pps_size).to_vec());
                }
            }
        }

        // オプショナルフィールドを構築
        let chroma_format = self
            .is_chroma_format_present
            .then_some(Uint::new(self.chroma_format));
        let bit_depth_luma_minus8 = self
            .is_bit_depth_luma_minus8_present
            .then_some(Uint::new(self.bit_depth_luma_minus8));
        let bit_depth_chroma_minus8 = self
            .is_bit_depth_chroma_minus8_present
            .then_some(Uint::new(self.bit_depth_chroma_minus8));

        // ボックスを構築
        let avcc_box = shiguredo_mp4::boxes::AvccBox {
            avc_profile_indication: self.avc_profile_indication,
            profile_compatibility: self.profile_compatibility,
            avc_level_indication: self.avc_level_indication,
            length_size_minus_one: Uint::new(self.length_size_minus_one),
            sps_list,
            pps_list,
            chroma_format,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            sps_ext_list: Vec::new(),
        };
        let visual = shiguredo_mp4::boxes::VisualSampleEntryFields {
            data_reference_index:
                shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
            width: self.width,
            height: self.height,
            horizresolution: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
            vertresolution: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
            frame_count: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
            compressorname: shiguredo_mp4::boxes::VisualSampleEntryFields::NULL_COMPRESSORNAME,
            depth: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_DEPTH,
        };
        let avc1_box = shiguredo_mp4::boxes::Avc1Box {
            visual,
            avcc_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Avc1(avc1_box))
    }
}
