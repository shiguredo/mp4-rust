//! ../../../src/boxes.rs の（一部に対応する） C API を定義するためのモジュール

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

impl From<&shiguredo_mp4::boxes::SampleEntry> for Mp4SampleEntryKind {
    fn from(entry: &shiguredo_mp4::boxes::SampleEntry) -> Self {
        match entry {
            shiguredo_mp4::boxes::SampleEntry::Avc1(_) => Self::Avc1,
            shiguredo_mp4::boxes::SampleEntry::Hev1(_) => Self::Hev1,
            shiguredo_mp4::boxes::SampleEntry::Vp08(_) => Self::Vp08,
            shiguredo_mp4::boxes::SampleEntry::Vp09(_) => Self::Vp09,
            shiguredo_mp4::boxes::SampleEntry::Av01(_) => Self::Av01,
            shiguredo_mp4::boxes::SampleEntry::Opus(_) => Self::Opus,
            shiguredo_mp4::boxes::SampleEntry::Mp4a(_) => Self::Mp4a,
            shiguredo_mp4::boxes::SampleEntry::Unknown(_) => Self::Unknown,
        }
    }
}

enum CodecSpecificData {
    Avc1 {
        sps_data: Vec<*const u8>,
        sps_sizes: Vec<u32>,
        pps_data: Vec<*const u8>,
        pps_sizes: Vec<u32>,
    },
}

#[repr(C)]
pub struct Mp4SampleEntry {
    inner: shiguredo_mp4::boxes::SampleEntry,
    data: CodecSpecificData,
}

impl From<shiguredo_mp4::boxes::SampleEntry> for Mp4SampleEntry {
    fn from(inner: shiguredo_mp4::boxes::SampleEntry) -> Self {
        let data = match &inner {
            shiguredo_mp4::boxes::SampleEntry::Avc1(avc1_box) => {
                let sps_data: Vec<*const u8> = avc1_box
                    .avcc_box
                    .sps_list
                    .iter()
                    .map(|sps| sps.as_ptr())
                    .collect();
                let sps_sizes: Vec<u32> = avc1_box
                    .avcc_box
                    .sps_list
                    .iter()
                    .map(|sps| sps.len() as u32)
                    .collect();

                let pps_data: Vec<*const u8> = avc1_box
                    .avcc_box
                    .pps_list
                    .iter()
                    .map(|pps| pps.as_ptr())
                    .collect();
                let pps_sizes: Vec<u32> = avc1_box
                    .avcc_box
                    .pps_list
                    .iter()
                    .map(|pps| pps.len() as u32)
                    .collect();

                CodecSpecificData::Avc1 {
                    sps_data,
                    sps_sizes,
                    pps_data,
                    pps_sizes,
                }
            }
            _ => todo!(),
        };

        Mp4SampleEntry { inner, data }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_sample_entry_get_kind(entry: *const Mp4SampleEntry) -> Mp4SampleEntryKind {
    if entry.is_null() {
        return Mp4SampleEntryKind::Unknown;
    }

    unsafe { Mp4SampleEntryKind::from(&(*entry).inner) }
}

#[unsafe(no_mangle)]
pub extern "C" fn mp4_sample_entry_get_avc1(
    entry: *const Mp4SampleEntry,
    out_entry: *mut Mp4SampleEntryAvc1,
) -> Mp4Error {
    if entry.is_null() {
        return Mp4Error::NullPointer;
    }

    let (
        shiguredo_mp4::boxes::SampleEntry::Avc1(inner),
        CodecSpecificData::Avc1 {
            sps_data,
            sps_sizes,
            pps_data,
            pps_sizes,
        },
    ) = (unsafe { &(*entry).inner }, unsafe { &(*entry).data })
    else {
        return Mp4Error::InvalidInput;
    };

    unsafe {
        (*out_entry).width = inner.visual.width as u32;
        (*out_entry).height = inner.visual.height as u32;

        (*out_entry).avc_profile_indication = inner.avcc_box.avc_profile_indication;
        (*out_entry).profile_compatibility = inner.avcc_box.profile_compatibility;
        (*out_entry).avc_level_indication = inner.avcc_box.avc_level_indication;
        (*out_entry).length_size_minus_one = inner.avcc_box.length_size_minus_one.get();

        (*out_entry).sps_data = sps_data.as_ptr();
        (*out_entry).sps_sizes = sps_sizes.as_ptr();
        (*out_entry).sps_count = sps_data.len() as u32;

        (*out_entry).pps_data = pps_data.as_ptr();
        (*out_entry).pps_sizes = pps_sizes.as_ptr();
        (*out_entry).pps_count = pps_data.len() as u32;

        (*out_entry).is_chroma_format_present = inner.avcc_box.chroma_format.is_some();
        (*out_entry).chroma_format = inner.avcc_box.chroma_format.map(|v| v.get()).unwrap_or(0);

        (*out_entry).is_bit_depth_luma_minus8_present =
            inner.avcc_box.bit_depth_luma_minus8.is_some();
        (*out_entry).bit_depth_luma_minus8 = inner
            .avcc_box
            .bit_depth_luma_minus8
            .map(|v| v.get())
            .unwrap_or(0);

        (*out_entry).is_bit_depth_chroma_minus8_present =
            inner.avcc_box.bit_depth_chroma_minus8.is_some();
        (*out_entry).bit_depth_chroma_minus8 = inner
            .avcc_box
            .bit_depth_chroma_minus8
            .map(|v| v.get())
            .unwrap_or(0);
    }

    Mp4Error::Ok
}

#[repr(C)]
pub struct Mp4SampleEntryAvc1 {
    pub width: u32,
    pub height: u32,

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
