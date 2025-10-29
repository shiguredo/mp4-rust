//! ../../../src/boxes.rs の（一部に対応する） C API を定義するためのモジュール

#[repr(C)]
pub enum Mp4SampleEntryKind {
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
    /// Unknown
    Unknown,
}

impl From<&crate::boxes::SampleEntry> for Mp4SampleEntryKind {
    fn from(entry: &crate::boxes::SampleEntry) -> Self {
        match entry {
            crate::boxes::SampleEntry::Avc1(_) => Self::Avc1,
            crate::boxes::SampleEntry::Hev1(_) => Self::Hev1,
            crate::boxes::SampleEntry::Vp08(_) => Self::Vp08,
            crate::boxes::SampleEntry::Vp09(_) => Self::Vp09,
            crate::boxes::SampleEntry::Av01(_) => Self::Av01,
            crate::boxes::SampleEntry::Opus(_) => Self::Opus,
            crate::boxes::SampleEntry::Mp4a(_) => Self::Mp4a,
            crate::boxes::SampleEntry::Unknown(_) => Self::Unknown,
        }
    }
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
