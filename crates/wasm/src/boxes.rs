//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール

use std::fmt::Result as FmtResult;

use nojson::{DisplayJson, JsonFormatter, json};

// c-api の型を re-export
pub use c_api::boxes::{
    Mp4SampleEntry, Mp4SampleEntryAv01, Mp4SampleEntryAvc1, Mp4SampleEntryData, Mp4SampleEntryFlac,
    Mp4SampleEntryHev1, Mp4SampleEntryHvc1, Mp4SampleEntryKind, Mp4SampleEntryMp4a,
    Mp4SampleEntryOpus, Mp4SampleEntryOwned, Mp4SampleEntryVp08, Mp4SampleEntryVp09,
};

/// Mp4SampleEntryOwned の JSON シリアライズ機能を提供する trait
pub trait ToJson {
    /// JSON 文字列に変換する
    fn to_json(&self) -> String;
}

impl ToJson for Mp4SampleEntryOwned {
    fn to_json(&self) -> String {
        json(|f| f.value(JsonSampleEntry(self))).to_string()
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
                f.member("sps", JsonBytesArray(&inner.avcc_box.sps_list))?;
                f.member("pps", JsonBytesArray(&inner.avcc_box.pps_list))
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
                f.member("configObus", JsonBytes(config_obus))
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
                f.member("decSpecificInfo", JsonBytes(dec_specific_info))
            }),
            Mp4SampleEntryOwned::Flac {
                inner,
                streaminfo_data,
            } => f.object(|f| {
                f.member("kind", "flac")?;
                f.member("channelCount", inner.audio.channelcount)?;
                f.member("sampleRate", inner.audio.samplerate.integer)?;
                f.member("sampleSize", inner.audio.samplesize)?;
                f.member("streaminfoData", JsonBytes(streaminfo_data))
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
            f.member("nalus", JsonBytesArray(&self.0.nalus))
        })
    }
}

struct JsonBytes<'a>(&'a [u8]);

impl DisplayJson for JsonBytes<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        f.array(|f| {
            for &byte in self.0 {
                f.element(byte)?;
            }
            Ok(())
        })
    }
}

struct JsonBytesArray<'a>(&'a [Vec<u8>]);

impl DisplayJson for JsonBytesArray<'_> {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> FmtResult {
        f.array(|f| {
            for item in self.0 {
                f.element(JsonBytes(item))?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiguredo_mp4::{FixedPointNumber, Uint, boxes::*};

    fn create_visual_sample_entry_fields(width: u16, height: u16) -> VisualSampleEntryFields {
        VisualSampleEntryFields {
            data_reference_index: VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
            width,
            height,
            horizresolution: VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
            vertresolution: VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
            frame_count: VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
            compressorname: VisualSampleEntryFields::NULL_COMPRESSORNAME,
            depth: VisualSampleEntryFields::DEFAULT_DEPTH,
        }
    }

    fn create_audio_sample_entry_fields(
        channel_count: u16,
        sample_rate: u16,
    ) -> AudioSampleEntryFields {
        AudioSampleEntryFields {
            data_reference_index: AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
            channelcount: channel_count,
            samplesize: 16,
            samplerate: FixedPointNumber::new(sample_rate, 0),
        }
    }

    #[test]
    fn test_opus_to_json() {
        let opus_box = OpusBox {
            audio: create_audio_sample_entry_fields(2, 48000),
            dops_box: DopsBox {
                output_channel_count: 2,
                pre_skip: 312,
                input_sample_rate: 48000,
                output_gain: 0,
            },
            unknown_boxes: Vec::new(),
        };

        let entry = SampleEntry::Opus(opus_box);
        let owned = Mp4SampleEntryOwned::new(entry).unwrap();
        let json = owned.to_json();

        assert!(json.contains(r#""kind":"opus""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":48000"#));
        assert!(json.contains(r#""preSkip":312"#));
        assert!(json.contains(r#""inputSampleRate":48000"#));
        assert!(json.contains(r#""outputGain":0"#));
    }

    #[test]
    fn test_avc1_to_json() {
        let avc1_box = Avc1Box {
            visual: create_visual_sample_entry_fields(1920, 1080),
            avcc_box: AvccBox {
                avc_profile_indication: 100,
                profile_compatibility: 0,
                avc_level_indication: 40,
                length_size_minus_one: Uint::new(3),
                sps_list: vec![vec![0x67, 0x64, 0x00, 0x28]],
                pps_list: vec![vec![0x68, 0xee, 0x3c, 0x80]],
                chroma_format: None,
                bit_depth_luma_minus8: None,
                bit_depth_chroma_minus8: None,
                sps_ext_list: Vec::new(),
            },
            unknown_boxes: Vec::new(),
        };

        let entry = SampleEntry::Avc1(avc1_box);
        let owned = Mp4SampleEntryOwned::new(entry).unwrap();
        let json = owned.to_json();

        assert!(json.contains(r#""kind":"avc1""#));
        assert!(json.contains(r#""width":1920"#));
        assert!(json.contains(r#""height":1080"#));
        assert!(json.contains(r#""avcProfileIndication":100"#));
        assert!(json.contains(r#""avcLevelIndication":40"#));
        assert!(json.contains(r#""lengthSizeMinusOne":3"#));
        assert!(json.contains(r#""sps":[[103,100,0,40]]"#));
        assert!(json.contains(r#""pps":[[104,238,60,128]]"#));
    }

    #[test]
    fn test_vp09_to_json() {
        let vp09_box = Vp09Box {
            visual: create_visual_sample_entry_fields(1280, 720),
            vpcc_box: VpccBox {
                profile: 0,
                level: 31,
                bit_depth: Uint::new(8),
                chroma_subsampling: Uint::new(1),
                video_full_range_flag: Uint::new(0),
                colour_primaries: 1,
                transfer_characteristics: 1,
                matrix_coefficients: 1,
                codec_initialization_data: Vec::new(),
            },
            unknown_boxes: Vec::new(),
        };

        let entry = SampleEntry::Vp09(vp09_box);
        let owned = Mp4SampleEntryOwned::new(entry).unwrap();
        let json = owned.to_json();

        assert!(json.contains(r#""kind":"vp09""#));
        assert!(json.contains(r#""width":1280"#));
        assert!(json.contains(r#""height":720"#));
        assert!(json.contains(r#""profile":0"#));
        assert!(json.contains(r#""level":31"#));
        assert!(json.contains(r#""bitDepth":8"#));
    }

    #[test]
    fn test_mp4a_to_json() {
        use shiguredo_mp4::descriptors::*;

        let mp4a_box = Mp4aBox {
            audio: create_audio_sample_entry_fields(2, 44100),
            esds_box: EsdsBox {
                es: EsDescriptor {
                    es_id: EsDescriptor::MIN_ES_ID,
                    stream_priority: EsDescriptor::LOWEST_STREAM_PRIORITY,
                    depends_on_es_id: None,
                    url_string: None,
                    ocr_es_id: None,
                    dec_config_descr: DecoderConfigDescriptor {
                        object_type_indication:
                            DecoderConfigDescriptor::OBJECT_TYPE_INDICATION_AUDIO_ISO_IEC_14496_3,
                        stream_type: DecoderConfigDescriptor::STREAM_TYPE_AUDIO,
                        up_stream: DecoderConfigDescriptor::UP_STREAM_FALSE,
                        buffer_size_db: Uint::new(0),
                        max_bitrate: 128000,
                        avg_bitrate: 128000,
                        dec_specific_info: Some(DecoderSpecificInfo {
                            payload: vec![0x12, 0x10],
                        }),
                    },
                    sl_config_descr: SlConfigDescriptor,
                },
            },
            unknown_boxes: Vec::new(),
        };

        let entry = SampleEntry::Mp4a(mp4a_box);
        let owned = Mp4SampleEntryOwned::new(entry).unwrap();
        let json = owned.to_json();

        assert!(json.contains(r#""kind":"mp4a""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":44100"#));
        assert!(json.contains(r#""maxBitrate":128000"#));
        assert!(json.contains(r#""avgBitrate":128000"#));
        assert!(json.contains(r#""decSpecificInfo":[18,16]"#));
    }

    #[test]
    fn test_av01_to_json() {
        let av01_box = Av01Box {
            visual: create_visual_sample_entry_fields(3840, 2160),
            av1c_box: Av1cBox {
                seq_profile: Uint::new(0),
                seq_level_idx_0: Uint::new(13),
                seq_tier_0: Uint::new(0),
                high_bitdepth: Uint::new(0),
                twelve_bit: Uint::new(0),
                monochrome: Uint::new(0),
                chroma_subsampling_x: Uint::new(1),
                chroma_subsampling_y: Uint::new(1),
                chroma_sample_position: Uint::new(0),
                initial_presentation_delay_minus_one: None,
                config_obus: vec![0x0a, 0x0b, 0x00, 0x00],
            },
            unknown_boxes: Vec::new(),
        };

        let entry = SampleEntry::Av01(av01_box);
        let owned = Mp4SampleEntryOwned::new(entry).unwrap();
        let json = owned.to_json();

        assert!(json.contains(r#""kind":"av01""#));
        assert!(json.contains(r#""width":3840"#));
        assert!(json.contains(r#""height":2160"#));
        assert!(json.contains(r#""seqProfile":0"#));
        assert!(json.contains(r#""seqLevelIdx0":13"#));
        assert!(json.contains(r#""configObus":[10,11,0,0]"#));
    }

    #[test]
    fn test_flac_to_json() {
        let flac_box = FlacBox {
            audio: create_audio_sample_entry_fields(2, 44100),
            dfla_box: DflaBox {
                metadata_blocks: vec![FlacMetadataBlock {
                    last_metadata_block_flag: Uint::from(true),
                    block_type: FlacMetadataBlock::BLOCK_TYPE_STREAMINFO,
                    block_data: vec![0x00, 0x10, 0x00, 0x10],
                }],
            },
            unknown_boxes: Vec::new(),
        };

        let entry = SampleEntry::Flac(flac_box);
        let owned = Mp4SampleEntryOwned::new(entry).unwrap();
        let json = owned.to_json();

        assert!(json.contains(r#""kind":"flac""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":44100"#));
        assert!(json.contains(r#""streaminfoData":[0,16,0,16]"#));
    }
}
