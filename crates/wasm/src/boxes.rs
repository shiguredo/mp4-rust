//! shiguredo_mp4::boxes の wasm FFI

use std::fmt::Result as FmtResult;

use nojson::{DisplayJson, JsonFormatter, json};
use shiguredo_mp4::Uint;

use crate::demux::Mp4WasmError;

/// サンプルエントリーの種類
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Mp4SampleEntryKind {
    Avc1 = 0,
    Hev1 = 1,
    Hvc1 = 2,
    Vp08 = 3,
    Vp09 = 4,
    Av01 = 5,
    Opus = 6,
    Mp4a = 7,
    Flac = 8,
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
    Hvc1 {
        inner: shiguredo_mp4::boxes::Hvc1Box,
        nalu_types: Vec<u8>,
        nalu_counts: Vec<u32>,
        nalu_data: Vec<*const u8>,
        nalu_sizes: Vec<u32>,
    },
    Vp08 {
        inner: shiguredo_mp4::boxes::Vp08Box,
    },
    Vp09 {
        inner: shiguredo_mp4::boxes::Vp09Box,
    },
    Av01 {
        inner: shiguredo_mp4::boxes::Av01Box,
        config_obus: Vec<u8>,
    },
    Opus {
        inner: shiguredo_mp4::boxes::OpusBox,
    },
    Mp4a {
        inner: shiguredo_mp4::boxes::Mp4aBox,
        dec_specific_info: Vec<u8>,
    },
    Flac {
        inner: shiguredo_mp4::boxes::FlacBox,
        streaminfo_data: Vec<u8>,
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
                    inner,
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
            shiguredo_mp4::boxes::SampleEntry::Hvc1(inner) => {
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

                Some(Self::Hvc1 {
                    inner,
                    nalu_types,
                    nalu_counts,
                    nalu_data,
                    nalu_sizes,
                })
            }
            shiguredo_mp4::boxes::SampleEntry::Vp08(inner) => Some(Self::Vp08 { inner }),
            shiguredo_mp4::boxes::SampleEntry::Vp09(inner) => Some(Self::Vp09 { inner }),
            shiguredo_mp4::boxes::SampleEntry::Av01(inner) => {
                let config_obus = inner.av1c_box.config_obus.clone();
                Some(Self::Av01 { inner, config_obus })
            }
            shiguredo_mp4::boxes::SampleEntry::Opus(inner) => Some(Self::Opus { inner }),
            shiguredo_mp4::boxes::SampleEntry::Mp4a(inner) => {
                let dec_specific_info = inner
                    .esds_box
                    .es
                    .dec_config_descr
                    .dec_specific_info
                    .as_ref()
                    .map_or(Vec::new(), |info| info.payload.clone());
                Some(Self::Mp4a {
                    inner,
                    dec_specific_info,
                })
            }
            shiguredo_mp4::boxes::SampleEntry::Flac(inner) => {
                let streaminfo_data = if let Some(block) = inner.dfla_box.metadata_blocks.first() {
                    block.block_data.clone()
                } else {
                    Vec::new()
                };
                Some(Self::Flac {
                    inner,
                    streaminfo_data,
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
            Self::Hvc1 {
                inner,
                nalu_types,
                nalu_counts,
                nalu_data,
                nalu_sizes,
            } => {
                let hvc1 = Mp4SampleEntryHvc1 {
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
                    kind: Mp4SampleEntryKind::Hvc1,
                    data: Mp4SampleEntryData { hvc1 },
                }
            }
            Self::Vp08 { inner } => {
                let vp08 = Mp4SampleEntryVp08 {
                    width: inner.visual.width,
                    height: inner.visual.height,
                    bit_depth: inner.vpcc_box.bit_depth.get(),
                    chroma_subsampling: inner.vpcc_box.chroma_subsampling.get(),
                    video_full_range_flag: if inner.vpcc_box.video_full_range_flag.get() != 0 {
                        1
                    } else {
                        0
                    },
                    colour_primaries: inner.vpcc_box.colour_primaries,
                    transfer_characteristics: inner.vpcc_box.transfer_characteristics,
                    matrix_coefficients: inner.vpcc_box.matrix_coefficients,
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Vp08,
                    data: Mp4SampleEntryData { vp08 },
                }
            }
            Self::Vp09 { inner } => {
                let vp09 = Mp4SampleEntryVp09 {
                    width: inner.visual.width,
                    height: inner.visual.height,
                    profile: inner.vpcc_box.profile,
                    level: inner.vpcc_box.level,
                    bit_depth: inner.vpcc_box.bit_depth.get(),
                    chroma_subsampling: inner.vpcc_box.chroma_subsampling.get(),
                    video_full_range_flag: if inner.vpcc_box.video_full_range_flag.get() != 0 {
                        1
                    } else {
                        0
                    },
                    colour_primaries: inner.vpcc_box.colour_primaries,
                    transfer_characteristics: inner.vpcc_box.transfer_characteristics,
                    matrix_coefficients: inner.vpcc_box.matrix_coefficients,
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Vp09,
                    data: Mp4SampleEntryData { vp09 },
                }
            }
            Self::Av01 { inner, config_obus } => {
                let av01 = Mp4SampleEntryAv01 {
                    width: inner.visual.width,
                    height: inner.visual.height,
                    seq_profile: inner.av1c_box.seq_profile.get(),
                    seq_level_idx_0: inner.av1c_box.seq_level_idx_0.get(),
                    seq_tier_0: inner.av1c_box.seq_tier_0.get(),
                    high_bitdepth: inner.av1c_box.high_bitdepth.get(),
                    twelve_bit: inner.av1c_box.twelve_bit.get(),
                    monochrome: inner.av1c_box.monochrome.get(),
                    chroma_subsampling_x: inner.av1c_box.chroma_subsampling_x.get(),
                    chroma_subsampling_y: inner.av1c_box.chroma_subsampling_y.get(),
                    chroma_sample_position: inner.av1c_box.chroma_sample_position.get(),
                    initial_presentation_delay_present: if inner
                        .av1c_box
                        .initial_presentation_delay_minus_one
                        .is_some()
                    {
                        1
                    } else {
                        0
                    },
                    initial_presentation_delay_minus_one: inner
                        .av1c_box
                        .initial_presentation_delay_minus_one
                        .map(|v| v.get())
                        .unwrap_or(0),
                    config_obus: config_obus.as_ptr(),
                    config_obus_size: config_obus.len() as u32,
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Av01,
                    data: Mp4SampleEntryData { av01 },
                }
            }
            Self::Opus { inner } => {
                let opus = Mp4SampleEntryOpus {
                    channel_count: inner.audio.channelcount as u8,
                    sample_rate: inner.audio.samplerate.integer,
                    sample_size: inner.audio.samplesize,
                    pre_skip: inner.dops_box.pre_skip,
                    input_sample_rate: inner.dops_box.input_sample_rate,
                    output_gain: inner.dops_box.output_gain,
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Opus,
                    data: Mp4SampleEntryData { opus },
                }
            }
            Self::Mp4a {
                inner,
                dec_specific_info,
            } => {
                let mp4a = Mp4SampleEntryMp4a {
                    channel_count: inner.audio.channelcount as u8,
                    sample_rate: inner.audio.samplerate.integer,
                    sample_size: inner.audio.samplesize,
                    buffer_size_db: inner.esds_box.es.dec_config_descr.buffer_size_db.get(),
                    max_bitrate: inner.esds_box.es.dec_config_descr.max_bitrate,
                    avg_bitrate: inner.esds_box.es.dec_config_descr.avg_bitrate,
                    dec_specific_info: dec_specific_info.as_ptr(),
                    dec_specific_info_size: dec_specific_info.len() as u32,
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Mp4a,
                    data: Mp4SampleEntryData { mp4a },
                }
            }
            Self::Flac {
                inner,
                streaminfo_data,
            } => {
                let flac = Mp4SampleEntryFlac {
                    channel_count: inner.audio.channelcount as u8,
                    sample_rate: inner.audio.samplerate.integer,
                    sample_size: inner.audio.samplesize,
                    streaminfo_data: streaminfo_data.as_ptr(),
                    streaminfo_size: streaminfo_data.len() as u32,
                };
                Mp4SampleEntry {
                    kind: Mp4SampleEntryKind::Flac,
                    data: Mp4SampleEntryData { flac },
                }
            }
        }
    }
}

#[repr(C)]
pub union Mp4SampleEntryData {
    pub avc1: Mp4SampleEntryAvc1,
    pub hev1: Mp4SampleEntryHev1,
    pub hvc1: Mp4SampleEntryHvc1,
    pub vp08: Mp4SampleEntryVp08,
    pub vp09: Mp4SampleEntryVp09,
    pub av01: Mp4SampleEntryAv01,
    pub opus: Mp4SampleEntryOpus,
    pub mp4a: Mp4SampleEntryMp4a,
    pub flac: Mp4SampleEntryFlac,
}

#[repr(C)]
pub struct Mp4SampleEntry {
    pub kind: Mp4SampleEntryKind,
    pub data: Mp4SampleEntryData,
}

impl Mp4SampleEntry {
    pub fn to_sample_entry(&self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        match self.kind {
            Mp4SampleEntryKind::Avc1 => unsafe { self.data.avc1.to_sample_entry() },
            Mp4SampleEntryKind::Hev1 => unsafe { self.data.hev1.to_sample_entry() },
            Mp4SampleEntryKind::Hvc1 => unsafe { self.data.hvc1.to_sample_entry() },
            Mp4SampleEntryKind::Vp08 => unsafe { self.data.vp08.to_sample_entry() },
            Mp4SampleEntryKind::Vp09 => unsafe { self.data.vp09.to_sample_entry() },
            Mp4SampleEntryKind::Av01 => unsafe { self.data.av01.to_sample_entry() },
            Mp4SampleEntryKind::Opus => unsafe { self.data.opus.to_sample_entry() },
            Mp4SampleEntryKind::Mp4a => unsafe { self.data.mp4a.to_sample_entry() },
            Mp4SampleEntryKind::Flac => unsafe { self.data.flac.to_sample_entry() },
        }
    }
}

/// AVC1 (H.264) サンプルエントリー
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
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let mut sps_list = Vec::new();
        if self.sps_count > 0 {
            if self.sps_data.is_null() {
                return Err(Mp4WasmError::NullPointer);
            }
            unsafe {
                for i in 0..self.sps_count as usize {
                    let sps_ptr = *self.sps_data.add(i);
                    let sps_size = *self.sps_sizes.add(i) as usize;
                    if sps_ptr.is_null() {
                        return Err(Mp4WasmError::NullPointer);
                    }
                    sps_list.push(std::slice::from_raw_parts(sps_ptr, sps_size).to_vec());
                }
            }
        }

        let mut pps_list = Vec::new();
        if self.pps_count > 0 {
            if self.pps_data.is_null() {
                return Err(Mp4WasmError::NullPointer);
            }
            unsafe {
                for i in 0..self.pps_count as usize {
                    let pps_ptr = *self.pps_data.add(i);
                    let pps_size = *self.pps_sizes.add(i) as usize;
                    if pps_ptr.is_null() {
                        return Err(Mp4WasmError::NullPointer);
                    }
                    pps_list.push(std::slice::from_raw_parts(pps_ptr, pps_size).to_vec());
                }
            }
        }

        let chroma_format = self
            .is_chroma_format_present
            .then_some(Uint::new(self.chroma_format));
        let bit_depth_luma_minus8 = self
            .is_bit_depth_luma_minus8_present
            .then_some(Uint::new(self.bit_depth_luma_minus8));
        let bit_depth_chroma_minus8 = self
            .is_bit_depth_chroma_minus8_present
            .then_some(Uint::new(self.bit_depth_chroma_minus8));

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
        let avc1_box = shiguredo_mp4::boxes::Avc1Box {
            visual: create_visual_sample_entry_fields(self.width, self.height),
            avcc_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Avc1(avc1_box))
    }
}

/// HEV1 (H.265/HEVC) サンプルエントリー
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
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let mut nalu_arrays = Vec::new();
        if self.nalu_array_count > 0 {
            unsafe {
                let mut nalu_index = 0usize;
                for i in 0..self.nalu_array_count as usize {
                    let nalu_type = *self.nalu_types.add(i);
                    let nalu_count = *self.nalu_counts.add(i);

                    let mut nalus = Vec::new();
                    for _ in 0..nalu_count as usize {
                        let nalu_ptr = *self.nalu_data.add(nalu_index);
                        let nalu_size = *self.nalu_sizes.add(nalu_index) as usize;

                        if nalu_ptr.is_null() {
                            return Err(Mp4WasmError::NullPointer);
                        }
                        nalus.push(std::slice::from_raw_parts(nalu_ptr, nalu_size).to_vec());
                        nalu_index += 1;
                    }

                    nalu_arrays.push(shiguredo_mp4::boxes::HvccNalUintArray {
                        array_completeness: shiguredo_mp4::Uint::new(0),
                        nal_unit_type: shiguredo_mp4::Uint::new(nalu_type),
                        nalus,
                    });
                }
            }
        }

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
        let hev1_box = shiguredo_mp4::boxes::Hev1Box {
            visual: create_visual_sample_entry_fields(self.width, self.height),
            hvcc_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Hev1(hev1_box))
    }
}

/// HVC1 (H.265/HEVC) サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryHvc1 {
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

impl Mp4SampleEntryHvc1 {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let mut nalu_arrays = Vec::new();
        if self.nalu_array_count > 0 {
            unsafe {
                let mut nalu_index = 0usize;
                for i in 0..self.nalu_array_count as usize {
                    let nalu_type = *self.nalu_types.add(i);
                    let nalu_count = *self.nalu_counts.add(i);

                    let mut nalus = Vec::new();
                    for _ in 0..nalu_count as usize {
                        let nalu_ptr = *self.nalu_data.add(nalu_index);
                        let nalu_size = *self.nalu_sizes.add(nalu_index) as usize;

                        if nalu_ptr.is_null() {
                            return Err(Mp4WasmError::NullPointer);
                        }
                        nalus.push(std::slice::from_raw_parts(nalu_ptr, nalu_size).to_vec());
                        nalu_index += 1;
                    }

                    nalu_arrays.push(shiguredo_mp4::boxes::HvccNalUintArray {
                        array_completeness: shiguredo_mp4::Uint::new(0),
                        nal_unit_type: shiguredo_mp4::Uint::new(nalu_type),
                        nalus,
                    });
                }
            }
        }

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
        let hvc1_box = shiguredo_mp4::boxes::Hvc1Box {
            visual: create_visual_sample_entry_fields(self.width, self.height),
            hvcc_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Hvc1(hvc1_box))
    }
}

/// VP08 (VP8) サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryVp08 {
    pub width: u16,
    pub height: u16,
    pub bit_depth: u8,
    pub chroma_subsampling: u8,
    pub video_full_range_flag: u8,
    pub colour_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
}

impl Mp4SampleEntryVp08 {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let vpcc_box = shiguredo_mp4::boxes::VpccBox {
            bit_depth: shiguredo_mp4::Uint::new(self.bit_depth),
            chroma_subsampling: shiguredo_mp4::Uint::new(self.chroma_subsampling),
            video_full_range_flag: shiguredo_mp4::Uint::new(self.video_full_range_flag),
            colour_primaries: self.colour_primaries,
            transfer_characteristics: self.transfer_characteristics,
            matrix_coefficients: self.matrix_coefficients,
            profile: 0,
            level: 0,
            codec_initialization_data: Vec::new(),
        };
        let vp08_box = shiguredo_mp4::boxes::Vp08Box {
            visual: create_visual_sample_entry_fields(self.width, self.height),
            vpcc_box,
            unknown_boxes: Vec::new(),
        };
        Ok(shiguredo_mp4::boxes::SampleEntry::Vp08(vp08_box))
    }
}

/// VP09 (VP9) サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryVp09 {
    pub width: u16,
    pub height: u16,
    pub profile: u8,
    pub level: u8,
    pub bit_depth: u8,
    pub chroma_subsampling: u8,
    pub video_full_range_flag: u8,
    pub colour_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
}

impl Mp4SampleEntryVp09 {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let vpcc_box = shiguredo_mp4::boxes::VpccBox {
            profile: self.profile,
            level: self.level,
            bit_depth: shiguredo_mp4::Uint::new(self.bit_depth),
            chroma_subsampling: shiguredo_mp4::Uint::new(self.chroma_subsampling),
            video_full_range_flag: shiguredo_mp4::Uint::new(self.video_full_range_flag),
            colour_primaries: self.colour_primaries,
            transfer_characteristics: self.transfer_characteristics,
            matrix_coefficients: self.matrix_coefficients,
            codec_initialization_data: Vec::new(),
        };
        let vp09_box = shiguredo_mp4::boxes::Vp09Box {
            visual: create_visual_sample_entry_fields(self.width, self.height),
            vpcc_box,
            unknown_boxes: Vec::new(),
        };
        Ok(shiguredo_mp4::boxes::SampleEntry::Vp09(vp09_box))
    }
}

/// AV01 (AV1) サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryAv01 {
    pub width: u16,
    pub height: u16,
    pub seq_profile: u8,
    pub seq_level_idx_0: u8,
    pub seq_tier_0: u8,
    pub high_bitdepth: u8,
    pub twelve_bit: u8,
    pub monochrome: u8,
    pub chroma_subsampling_x: u8,
    pub chroma_subsampling_y: u8,
    pub chroma_sample_position: u8,
    pub initial_presentation_delay_present: u8,
    pub initial_presentation_delay_minus_one: u8,
    pub config_obus: *const u8,
    pub config_obus_size: u32,
}

impl Mp4SampleEntryAv01 {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let config_obus = if self.config_obus_size > 0 {
            if self.config_obus.is_null() {
                return Err(Mp4WasmError::NullPointer);
            }
            unsafe {
                std::slice::from_raw_parts(self.config_obus, self.config_obus_size as usize)
                    .to_vec()
            }
        } else {
            Vec::new()
        };

        let initial_presentation_delay_minus_one = (self.initial_presentation_delay_present != 0)
            .then_some(shiguredo_mp4::Uint::new(
                self.initial_presentation_delay_minus_one,
            ));

        let av1c_box = shiguredo_mp4::boxes::Av1cBox {
            seq_profile: shiguredo_mp4::Uint::new(self.seq_profile),
            seq_level_idx_0: shiguredo_mp4::Uint::new(self.seq_level_idx_0),
            seq_tier_0: shiguredo_mp4::Uint::new(self.seq_tier_0),
            high_bitdepth: shiguredo_mp4::Uint::new(self.high_bitdepth),
            twelve_bit: shiguredo_mp4::Uint::new(self.twelve_bit),
            monochrome: shiguredo_mp4::Uint::new(self.monochrome),
            chroma_subsampling_x: shiguredo_mp4::Uint::new(self.chroma_subsampling_x),
            chroma_subsampling_y: shiguredo_mp4::Uint::new(self.chroma_subsampling_y),
            chroma_sample_position: shiguredo_mp4::Uint::new(self.chroma_sample_position),
            initial_presentation_delay_minus_one,
            config_obus,
        };
        let av01_box = shiguredo_mp4::boxes::Av01Box {
            visual: create_visual_sample_entry_fields(self.width, self.height),
            av1c_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Av01(av01_box))
    }
}

fn create_visual_sample_entry_fields(
    width: u16,
    height: u16,
) -> shiguredo_mp4::boxes::VisualSampleEntryFields {
    shiguredo_mp4::boxes::VisualSampleEntryFields {
        data_reference_index:
            shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
        width,
        height,
        horizresolution: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
        vertresolution: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
        frame_count: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
        compressorname: shiguredo_mp4::boxes::VisualSampleEntryFields::NULL_COMPRESSORNAME,
        depth: shiguredo_mp4::boxes::VisualSampleEntryFields::DEFAULT_DEPTH,
    }
}

/// Opus サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryOpus {
    pub channel_count: u8,
    pub sample_rate: u16,
    pub sample_size: u16,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
    pub output_gain: i16,
}

impl Mp4SampleEntryOpus {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let dops_box = shiguredo_mp4::boxes::DopsBox {
            output_channel_count: self.channel_count,
            pre_skip: self.pre_skip,
            input_sample_rate: self.input_sample_rate,
            output_gain: self.output_gain,
        };
        let opus_box = shiguredo_mp4::boxes::OpusBox {
            audio: shiguredo_mp4::boxes::AudioSampleEntryFields {
                data_reference_index:
                    shiguredo_mp4::boxes::AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
                channelcount: self.channel_count as u16,
                samplesize: self.sample_size,
                samplerate: shiguredo_mp4::FixedPointNumber::new(self.sample_rate, 0),
            },
            dops_box,
            unknown_boxes: Vec::new(),
        };
        Ok(shiguredo_mp4::boxes::SampleEntry::Opus(opus_box))
    }
}

/// MP4A (AAC) サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryMp4a {
    pub channel_count: u8,
    pub sample_rate: u16,
    pub sample_size: u16,
    pub buffer_size_db: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,
    pub dec_specific_info: *const u8,
    pub dec_specific_info_size: u32,
}

impl Mp4SampleEntryMp4a {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let dec_specific_info = if self.dec_specific_info_size > 0 {
            if self.dec_specific_info.is_null() {
                return Err(Mp4WasmError::NullPointer);
            }
            unsafe {
                std::slice::from_raw_parts(
                    self.dec_specific_info,
                    self.dec_specific_info_size as usize,
                )
                .to_vec()
            }
        } else {
            Vec::new()
        };

        let object_type_indication = shiguredo_mp4::descriptors::DecoderConfigDescriptor::OBJECT_TYPE_INDICATION_AUDIO_ISO_IEC_14496_3;
        let dec_config_descr = shiguredo_mp4::descriptors::DecoderConfigDescriptor {
            object_type_indication,
            stream_type: shiguredo_mp4::descriptors::DecoderConfigDescriptor::STREAM_TYPE_AUDIO,
            up_stream: shiguredo_mp4::descriptors::DecoderConfigDescriptor::UP_STREAM_FALSE,
            buffer_size_db: Uint::new(self.buffer_size_db),
            max_bitrate: self.max_bitrate,
            avg_bitrate: self.avg_bitrate,
            dec_specific_info: Some(shiguredo_mp4::descriptors::DecoderSpecificInfo {
                payload: dec_specific_info,
            }),
        };
        let esds_box = shiguredo_mp4::boxes::EsdsBox {
            es: shiguredo_mp4::descriptors::EsDescriptor {
                es_id: shiguredo_mp4::descriptors::EsDescriptor::MIN_ES_ID,
                stream_priority: shiguredo_mp4::descriptors::EsDescriptor::LOWEST_STREAM_PRIORITY,
                depends_on_es_id: None,
                url_string: None,
                ocr_es_id: None,
                dec_config_descr,
                sl_config_descr: shiguredo_mp4::descriptors::SlConfigDescriptor,
            },
        };
        let mp4a_box = shiguredo_mp4::boxes::Mp4aBox {
            audio: shiguredo_mp4::boxes::AudioSampleEntryFields {
                data_reference_index:
                    shiguredo_mp4::boxes::AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
                channelcount: self.channel_count as u16,
                samplesize: self.sample_size,
                samplerate: shiguredo_mp4::FixedPointNumber::new(self.sample_rate, 0),
            },
            esds_box,
            unknown_boxes: Vec::new(),
        };
        Ok(shiguredo_mp4::boxes::SampleEntry::Mp4a(mp4a_box))
    }
}

/// FLAC サンプルエントリー
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Mp4SampleEntryFlac {
    pub channel_count: u8,
    pub sample_rate: u16,
    pub sample_size: u16,
    pub streaminfo_data: *const u8,
    pub streaminfo_size: u32,
}

impl Mp4SampleEntryFlac {
    fn to_sample_entry(self) -> Result<shiguredo_mp4::boxes::SampleEntry, Mp4WasmError> {
        let streaminfo = if self.streaminfo_size > 0 {
            if self.streaminfo_data.is_null() {
                return Err(Mp4WasmError::NullPointer);
            }
            unsafe {
                std::slice::from_raw_parts(self.streaminfo_data, self.streaminfo_size as usize)
                    .to_vec()
            }
        } else {
            Vec::new()
        };

        let dfla_box = shiguredo_mp4::boxes::DflaBox {
            metadata_blocks: vec![shiguredo_mp4::boxes::FlacMetadataBlock {
                last_metadata_block_flag: Uint::from(true),
                block_type: shiguredo_mp4::boxes::FlacMetadataBlock::BLOCK_TYPE_STREAMINFO,
                block_data: streaminfo,
            }],
        };

        let flac_box = shiguredo_mp4::boxes::FlacBox {
            audio: shiguredo_mp4::boxes::AudioSampleEntryFields {
                data_reference_index:
                    shiguredo_mp4::boxes::AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
                channelcount: self.channel_count as u16,
                samplesize: self.sample_size,
                samplerate: shiguredo_mp4::FixedPointNumber::new(self.sample_rate, 0),
            },
            dfla_box,
            unknown_boxes: Vec::new(),
        };

        Ok(shiguredo_mp4::boxes::SampleEntry::Flac(flac_box))
    }
}

// JSON シリアライズ用のヘルパー構造体
struct JsonSampleEntry<'a>(&'a Mp4SampleEntryOwned);

impl DisplayJson for JsonSampleEntry<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        match self.0 {
            Mp4SampleEntryOwned::Avc1 { inner, .. } => f.object(|f| {
                f.member("kind", "avc1")?;
                f.member("width", inner.visual.width)?;
                f.member("height", inner.visual.height)?;
                f.member(
                    "avcProfileIndication",
                    inner.avcc_box.avc_profile_indication,
                )?;
                f.member("profileCompatibility", inner.avcc_box.profile_compatibility)?;
                f.member("avcLevelIndication", inner.avcc_box.avc_level_indication)?;
                f.member(
                    "lengthSizeMinusOne",
                    inner.avcc_box.length_size_minus_one.get(),
                )?;
                if let Some(v) = inner.avcc_box.chroma_format {
                    f.member("chromaFormat", v.get())?;
                }
                if let Some(v) = inner.avcc_box.bit_depth_luma_minus8 {
                    f.member("bitDepthLumaMinus8", v.get())?;
                }
                if let Some(v) = inner.avcc_box.bit_depth_chroma_minus8 {
                    f.member("bitDepthChromaMinus8", v.get())?;
                }
                f.member("sps", JsonBase64Array(&inner.avcc_box.sps_list))?;
                f.member("pps", JsonBase64Array(&inner.avcc_box.pps_list))
            }),
            Mp4SampleEntryOwned::Hev1 { inner, .. } => {
                format_hevc(f, "hev1", &inner.visual, &inner.hvcc_box)
            }
            Mp4SampleEntryOwned::Hvc1 { inner, .. } => {
                format_hevc(f, "hvc1", &inner.visual, &inner.hvcc_box)
            }
            Mp4SampleEntryOwned::Vp08 { inner } => f.object(|f| {
                f.member("kind", "vp08")?;
                f.member("width", inner.visual.width)?;
                f.member("height", inner.visual.height)?;
                f.member("bitDepth", inner.vpcc_box.bit_depth.get())?;
                f.member("chromaSubsampling", inner.vpcc_box.chroma_subsampling.get())?;
                f.member(
                    "videoFullRangeFlag",
                    inner.vpcc_box.video_full_range_flag.get(),
                )?;
                f.member("colourPrimaries", inner.vpcc_box.colour_primaries)?;
                f.member(
                    "transferCharacteristics",
                    inner.vpcc_box.transfer_characteristics,
                )?;
                f.member("matrixCoefficients", inner.vpcc_box.matrix_coefficients)
            }),
            Mp4SampleEntryOwned::Vp09 { inner } => f.object(|f| {
                f.member("kind", "vp09")?;
                f.member("width", inner.visual.width)?;
                f.member("height", inner.visual.height)?;
                f.member("profile", inner.vpcc_box.profile)?;
                f.member("level", inner.vpcc_box.level)?;
                f.member("bitDepth", inner.vpcc_box.bit_depth.get())?;
                f.member("chromaSubsampling", inner.vpcc_box.chroma_subsampling.get())?;
                f.member(
                    "videoFullRangeFlag",
                    inner.vpcc_box.video_full_range_flag.get(),
                )?;
                f.member("colourPrimaries", inner.vpcc_box.colour_primaries)?;
                f.member(
                    "transferCharacteristics",
                    inner.vpcc_box.transfer_characteristics,
                )?;
                f.member("matrixCoefficients", inner.vpcc_box.matrix_coefficients)
            }),
            Mp4SampleEntryOwned::Av01 { inner, config_obus } => f.object(|f| {
                f.member("kind", "av01")?;
                f.member("width", inner.visual.width)?;
                f.member("height", inner.visual.height)?;
                f.member("seqProfile", inner.av1c_box.seq_profile.get())?;
                f.member("seqLevelIdx0", inner.av1c_box.seq_level_idx_0.get())?;
                f.member("seqTier0", inner.av1c_box.seq_tier_0.get())?;
                f.member("highBitdepth", inner.av1c_box.high_bitdepth.get())?;
                f.member("twelveBit", inner.av1c_box.twelve_bit.get())?;
                f.member("monochrome", inner.av1c_box.monochrome.get())?;
                f.member(
                    "chromaSubsamplingX",
                    inner.av1c_box.chroma_subsampling_x.get(),
                )?;
                f.member(
                    "chromaSubsamplingY",
                    inner.av1c_box.chroma_subsampling_y.get(),
                )?;
                f.member(
                    "chromaSamplePosition",
                    inner.av1c_box.chroma_sample_position.get(),
                )?;
                if let Some(v) = inner.av1c_box.initial_presentation_delay_minus_one {
                    f.member("initialPresentationDelayMinusOne", v.get())?;
                }
                f.member("configObus", JsonBase64(config_obus))
            }),
            Mp4SampleEntryOwned::Opus { inner } => f.object(|f| {
                f.member("kind", "opus")?;
                f.member("channelCount", inner.audio.channelcount)?;
                f.member("sampleRate", inner.audio.samplerate.integer)?;
                f.member("sampleSize", inner.audio.samplesize)?;
                f.member("preSkip", inner.dops_box.pre_skip)?;
                f.member("inputSampleRate", inner.dops_box.input_sample_rate)?;
                f.member("outputGain", inner.dops_box.output_gain)
            }),
            Mp4SampleEntryOwned::Mp4a {
                inner,
                dec_specific_info,
            } => f.object(|f| {
                f.member("kind", "mp4a")?;
                f.member("channelCount", inner.audio.channelcount)?;
                f.member("sampleRate", inner.audio.samplerate.integer)?;
                f.member("sampleSize", inner.audio.samplesize)?;
                f.member(
                    "bufferSizeDb",
                    inner.esds_box.es.dec_config_descr.buffer_size_db.get(),
                )?;
                f.member("maxBitrate", inner.esds_box.es.dec_config_descr.max_bitrate)?;
                f.member("avgBitrate", inner.esds_box.es.dec_config_descr.avg_bitrate)?;
                f.member("decSpecificInfo", JsonBase64(dec_specific_info))
            }),
            Mp4SampleEntryOwned::Flac {
                inner,
                streaminfo_data,
            } => f.object(|f| {
                f.member("kind", "flac")?;
                f.member("channelCount", inner.audio.channelcount)?;
                f.member("sampleRate", inner.audio.samplerate.integer)?;
                f.member("sampleSize", inner.audio.samplesize)?;
                f.member("streaminfoData", JsonBase64(streaminfo_data))
            }),
        }
    }
}

fn format_hevc(
    f: &mut JsonFormatter<'_, '_>,
    kind: &str,
    visual: &shiguredo_mp4::boxes::VisualSampleEntryFields,
    hvcc: &shiguredo_mp4::boxes::HvccBox,
) -> FmtResult {
    f.object(|f| {
        f.member("kind", kind)?;
        f.member("width", visual.width)?;
        f.member("height", visual.height)?;
        f.member("generalProfileSpace", hvcc.general_profile_space.get())?;
        f.member("generalTierFlag", hvcc.general_tier_flag.get())?;
        f.member("generalProfileIdc", hvcc.general_profile_idc.get())?;
        f.member(
            "generalProfileCompatibilityFlags",
            hvcc.general_profile_compatibility_flags,
        )?;
        f.member(
            "generalConstraintIndicatorFlags",
            hvcc.general_constraint_indicator_flags.get(),
        )?;
        f.member("generalLevelIdc", hvcc.general_level_idc)?;
        f.member("chromaFormatIdc", hvcc.chroma_format_idc.get())?;
        f.member("bitDepthLumaMinus8", hvcc.bit_depth_luma_minus8.get())?;
        f.member("bitDepthChromaMinus8", hvcc.bit_depth_chroma_minus8.get())?;
        f.member(
            "minSpatialSegmentationIdc",
            hvcc.min_spatial_segmentation_idc.get(),
        )?;
        f.member("parallelismType", hvcc.parallelism_type.get())?;
        f.member("avgFrameRate", hvcc.avg_frame_rate)?;
        f.member("constantFrameRate", hvcc.constant_frame_rate.get())?;
        f.member("numTemporalLayers", hvcc.num_temporal_layers.get())?;
        f.member("temporalIdNested", hvcc.temporal_id_nested.get())?;
        f.member("lengthSizeMinusOne", hvcc.length_size_minus_one.get())?;
        f.member("naluArrays", JsonNaluArrays(&hvcc.nalu_arrays))
    })
}

struct JsonNaluArrays<'a>(&'a [shiguredo_mp4::boxes::HvccNalUintArray]);

impl DisplayJson for JsonNaluArrays<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        f.array(|f| {
            for array in self.0 {
                f.element(JsonNaluArray(array))?;
            }
            Ok(())
        })
    }
}

struct JsonNaluArray<'a>(&'a shiguredo_mp4::boxes::HvccNalUintArray);

impl DisplayJson for JsonNaluArray<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        f.object(|f| {
            f.member("type", self.0.nal_unit_type.get())?;
            f.member("nalus", JsonBase64Array(&self.0.nalus))
        })
    }
}

struct JsonBase64<'a>(&'a [u8]);

impl DisplayJson for JsonBase64<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        f.string(base64_encode(self.0))
    }
}

struct JsonBase64Array<'a>(&'a [Vec<u8>]);

impl DisplayJson for JsonBase64Array<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        f.array(|f| {
            for item in self.0 {
                f.element(JsonBase64(item))?;
            }
            Ok(())
        })
    }
}

impl Mp4SampleEntryOwned {
    /// JSON 文字列に変換する
    pub fn to_json(&self) -> String {
        json(|f| f.value(JsonSampleEntry(self))).to_string()
    }
}

/// Base64 エンコード (RFC 4648 標準)
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let chunks = data.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}
