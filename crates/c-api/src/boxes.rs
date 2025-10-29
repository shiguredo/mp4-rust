//! ../../../src/boxes.rs の（一部に対応する） C API を定義するためのモジュール
use shiguredo_mp4::Uint;

use crate::error::Mp4Error;

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
}

pub enum Mp4SampleEntryOwned {
    Avc1 {
        inner: shiguredo_mp4::boxes::Avc1Box,
        sps_data: Vec<*const u8>,
        sps_sizes: Vec<u32>,
        pps_data: Vec<*const u8>,
        pps_sizes: Vec<u32>,
    },
    Hev1 {
        inner: shiguredo_mp4::boxes::Hev1Box,
        nalu_types: Vec<u8>,
        nalu_counts: Vec<u32>,
        nalu_data: Vec<*const u8>,
        nalu_sizes: Vec<u32>,
    },
}

impl Mp4SampleEntryOwned {
    pub fn new(entry: shiguredo_mp4::boxes::SampleEntry) -> Option<Self> {
        match entry {
            shiguredo_mp4::boxes::SampleEntry::Avc1(inner) => {
                let mut sps_data = Vec::new();
                let mut sps_sizes = Vec::new();
                for sps in &inner.avcc_box.sps_list {
                    sps_data.push(sps.as_ptr());
                    sps_sizes.push(sps.len() as u32);
                }

                let mut pps_data = Vec::new();
                let mut pps_sizes = Vec::new();
                for pps in &inner.avcc_box.pps_list {
                    pps_data.push(pps.as_ptr());
                    pps_sizes.push(pps.len() as u32);
                }

                Some(Self::Avc1 {
                    inner: inner,
                    sps_data,
                    sps_sizes,
                    pps_data,
                    pps_sizes,
                })
            }
            shiguredo_mp4::boxes::SampleEntry::Hev1(inner) => {
                let mut nalu_types = Vec::new();
                let mut nalu_counts = Vec::new();
                let mut nalu_data = Vec::new();
                let mut nalu_sizes = Vec::new();

                for array in &inner.hvcc_box.nalu_arrays {
                    nalu_types.push(array.nal_unit_type.get());
                    nalu_counts.push(array.nalus.len() as u32);

                    for nalu in &array.nalus {
                        nalu_data.push(nalu.as_ptr());
                        nalu_sizes.push(nalu.len() as u32);
                    }
                }

                Some(Self::Hev1 {
                    inner,
                    nalu_types,
                    nalu_counts,
                    nalu_data,
                    nalu_sizes,
                })
            }

            _ => None,
        }
    }

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
            Self::Hev1 {
                inner,
                nalu_types,
                nalu_counts,
                nalu_data,
                nalu_sizes,
            } => {
                let hev1 = Mp4SampleEntryHev1 {
                    width: inner.visual.width,
                    height: inner.visual.height,
                    general_profile_space: inner.hvcc_box.general_profile_space.get(),
                    general_tier_flag: inner.hvcc_box.general_tier_flag.get(),
                    general_profile_idc: inner.hvcc_box.general_profile_idc.get(),
                    general_profile_compatibility_flags: inner
                        .hvcc_box
                        .general_profile_compatibility_flags,
                    general_constraint_indicator_flags: inner
                        .hvcc_box
                        .general_constraint_indicator_flags
                        .get(),
                    general_level_idc: inner.hvcc_box.general_level_idc,
                    chroma_format_idc: inner.hvcc_box.chroma_format_idc.get(),
                    bit_depth_luma_minus8: inner.hvcc_box.bit_depth_luma_minus8.get(),
                    bit_depth_chroma_minus8: inner.hvcc_box.bit_depth_chroma_minus8.get(),
                    min_spatial_segmentation_idc: inner.hvcc_box.min_spatial_segmentation_idc.get(),
                    parallelism_type: inner.hvcc_box.parallelism_type.get(),
                    avg_frame_rate: inner.hvcc_box.avg_frame_rate,
                    constant_frame_rate: inner.hvcc_box.constant_frame_rate.get(),
                    num_temporal_layers: inner.hvcc_box.num_temporal_layers.get(),
                    temporal_id_nested: inner.hvcc_box.temporal_id_nested.get(),
                    length_size_minus_one: inner.hvcc_box.length_size_minus_one.get(),
                    nalu_array_count: nalu_types.len() as u32,
                    nalu_types: nalu_types.as_ptr(),
                    nalu_counts: nalu_counts.as_ptr(),
                    nalu_data: nalu_data.as_ptr(),
                    nalu_sizes: nalu_sizes.as_ptr(),
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Hev1,
                    data: Mp4SampleEntryData { hev1 },
                }
            }
        }
    }
}

#[repr(C)]
pub union Mp4SampleEntryData {
    pub avc1: Mp4SampleEntryAvc1,
    pub hev1: Mp4SampleEntryHev1,
    //pub vp08: Mp4SampleEntryVp08,
    //pub vp09: Mp4SampleEntryVp09,
    //pub av01: Mp4SampleEntryAv01,
    //pub opus: Mp4SampleEntryOpus,
    //pub mp4a: Mp4SampleEntryMp4a,
}

#[repr(C)]
pub struct Mp4SampleEntry {
    pub kind: Mp4SampleEntryKind,
    pub data: Mp4SampleEntryData,
}

impl Mp4SampleEntry {
    pub fn to_sample_entry(&self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4Error> {
        match self.kind {
            Mp4SampleEntryKind::Avc1 => unsafe { self.data.avc1.to_sample_entry() },
            Mp4SampleEntryKind::Hev1 => unsafe { self.data.hev1.to_sample_entry() },
            _ => Err(Mp4Error::InvalidInput),
        }
    }
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
    fn to_sample_entry(&self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4Error> {
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

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryHev1 {
    pub width: u16,
    pub height: u16,

    pub general_profile_space: u8,
    pub general_tier_flag: u8,
    pub general_profile_idc: u8,
    pub general_profile_compatibility_flags: u32,
    pub general_constraint_indicator_flags: u64,
    pub general_level_idc: u8,
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub min_spatial_segmentation_idc: u16,
    pub parallelism_type: u8,
    pub avg_frame_rate: u16,
    pub constant_frame_rate: u8,
    pub num_temporal_layers: u8,
    pub temporal_id_nested: u8,
    pub length_size_minus_one: u8,

    pub nalu_array_count: u32,
    pub nalu_types: *const u8,
    pub nalu_counts: *const u32,
    pub nalu_data: *const *const u8,
    pub nalu_sizes: *const u32,
}

impl Mp4SampleEntryHev1 {
    fn to_sample_entry(&self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4Error> {
        // NALU 配列を構築
        let mut nalu_arrays = Vec::new();
        if self.nalu_array_count > 0 {
            unsafe {
                for i in 0..self.nalu_array_count as usize {
                    let nalu_type = *self.nalu_types.add(i);
                    let nalu_count = *self.nalu_counts.add(i);

                    let mut nalus = Vec::new();
                    for j in 0..nalu_count as usize {
                        let nalu_index = self.nalu_data_index(i, j);
                        let nalu_ptr = *self.nalu_data.add(nalu_index);
                        let nalu_size = *self.nalu_sizes.add(nalu_index) as usize;

                        if nalu_ptr.is_null() {
                            return Err(Mp4Error::NullPointer);
                        }
                        nalus.push(std::slice::from_raw_parts(nalu_ptr, nalu_size).to_vec());
                    }

                    nalu_arrays.push(shiguredo_mp4::boxes::HvccNalUintArray {
                        // 保守的な固定値: この NALU 型のすべてのインスタンスが配列に含まれていない可能性を示す
                        array_completeness: shiguredo_mp4::Uint::new(0),

                        nal_unit_type: shiguredo_mp4::Uint::new(nalu_type),
                        nalus,
                    });
                }
            }
        }

        // ボックスを構築
        let hvcc_box = shiguredo_mp4::boxes::HvccBox {
            general_profile_space: shiguredo_mp4::Uint::new(self.general_profile_space),
            general_tier_flag: shiguredo_mp4::Uint::new(self.general_tier_flag),
            general_profile_idc: shiguredo_mp4::Uint::new(self.general_profile_idc),
            general_profile_compatibility_flags: self.general_profile_compatibility_flags,
            general_constraint_indicator_flags: shiguredo_mp4::Uint::new(
                self.general_constraint_indicator_flags,
            ),
            general_level_idc: self.general_level_idc,
            min_spatial_segmentation_idc: shiguredo_mp4::Uint::new(
                self.min_spatial_segmentation_idc,
            ),
            parallelism_type: shiguredo_mp4::Uint::new(self.parallelism_type),
            chroma_format_idc: shiguredo_mp4::Uint::new(self.chroma_format_idc),
            bit_depth_luma_minus8: shiguredo_mp4::Uint::new(self.bit_depth_luma_minus8),
            bit_depth_chroma_minus8: shiguredo_mp4::Uint::new(self.bit_depth_chroma_minus8),
            avg_frame_rate: self.avg_frame_rate,
            constant_frame_rate: shiguredo_mp4::Uint::new(self.constant_frame_rate),
            num_temporal_layers: shiguredo_mp4::Uint::new(self.num_temporal_layers),
            temporal_id_nested: shiguredo_mp4::Uint::new(self.temporal_id_nested),
            length_size_minus_one: shiguredo_mp4::Uint::new(self.length_size_minus_one),
            nalu_arrays,
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
        let hev1_box = shiguredo_mp4::boxes::Hev1Box {
            visual,
            hvcc_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Hev1(hev1_box))
    }

    fn nalu_data_index(&self, array_index: usize, nalu_index: usize) -> usize {
        unsafe {
            let mut index = 0;
            // 指定された配列インデックスまでの NALU 数を合計する
            for i in 0..array_index {
                let count = *self.nalu_counts.add(i) as usize;
                index += count;
            }
            // 現在の配列内でのインデックスを加算
            index += nalu_index;
            index
        }
    }
}
