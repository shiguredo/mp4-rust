//! ../../../src/boxes.rs の（一部に対応する） C API を定義するためのモジュール

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
