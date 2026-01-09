//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール

use c_api::boxes::{
    Mp4SampleEntry, Mp4SampleEntryAv01, Mp4SampleEntryAvc1, Mp4SampleEntryData, Mp4SampleEntryFlac,
    Mp4SampleEntryHev1, Mp4SampleEntryHvc1, Mp4SampleEntryKind, Mp4SampleEntryMp4a,
    Mp4SampleEntryOpus, Mp4SampleEntryOwned, Mp4SampleEntryVp08, Mp4SampleEntryVp09,
};

pub fn fmt_json_mp4_sample_entry(
    f: &mut nojson::JsonFormatter<'_, '_>,
    sample_entry: &Mp4SampleEntry,
) -> std::fmt::Result {
    match sample_entry.kind {
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AVC1 => {
            let data = unsafe { &sample_entry.data.avc1 };
            f.object(|f| {
                f.member("kind", "avc1")?;
                f.member("width", data.width)?;
                f.member("height", data.height)?;
                f.member("avcProfileIndication", data.avc_profile_indication)?;
                f.member("profileCompatibility", data.profile_compatibility)?;
                f.member("avcLevelIndication", data.avc_level_indication)?;
                f.member("lengthSizeMinusOne", data.length_size_minus_one)?;
                if data.is_chroma_format_present {
                    f.member("chromaFormat", data.chroma_format)?;
                }
                if data.is_bit_depth_luma_minus8_present {
                    f.member("bitDepthLumaMinus8", data.bit_depth_luma_minus8)?;
                }
                if data.is_bit_depth_chroma_minus8_present {
                    f.member("bitDepthChromaMinus8", data.bit_depth_chroma_minus8)?;
                }
                f.member(
                    "sps",
                    JsonAvcNaluList {
                        data_ptr: data.sps_data,
                        sizes_ptr: data.sps_sizes,
                        count: data.sps_count,
                    },
                )?;
                f.member(
                    "pps",
                    JsonAvcNaluList {
                        data_ptr: data.pps_data,
                        sizes_ptr: data.pps_sizes,
                        count: data.pps_count,
                    },
                )
            })
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_HEV1 => {
            // let data = unsafe { &sample_entry.data.hev1 };
            // format_hevc_ref(f, "hev1", data)
            todo!()
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_HVC1 => {
            // let data = unsafe { &sample_entry.data.hvc1 };
            // format_hvc1_ref(f, "hvc1", data)
            todo!()
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP08 => {
            let data = unsafe { &sample_entry.data.vp08 };
            f.object(|f| {
                f.member("kind", "vp08")?;
                f.member("width", data.width)?;
                f.member("height", data.height)?;
                f.member("bitDepth", data.bit_depth)?;
                f.member("chromaSubsampling", data.chroma_subsampling)?;
                f.member("videoFullRangeFlag", u8::from(data.video_full_range_flag))?;
                f.member("colourPrimaries", data.colour_primaries)?;
                f.member("transferCharacteristics", data.transfer_characteristics)?;
                f.member("matrixCoefficients", data.matrix_coefficients)
            })
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP09 => {
            let data = unsafe { &sample_entry.data.vp09 };
            f.object(|f| {
                f.member("kind", "vp09")?;
                f.member("width", data.width)?;
                f.member("height", data.height)?;
                f.member("profile", data.profile)?;
                f.member("level", data.level)?;
                f.member("bitDepth", data.bit_depth)?;
                f.member("chromaSubsampling", data.chroma_subsampling)?;
                f.member("videoFullRangeFlag", u8::from(data.video_full_range_flag))?;
                f.member("colourPrimaries", data.colour_primaries)?;
                f.member("transferCharacteristics", data.transfer_characteristics)?;
                f.member("matrixCoefficients", data.matrix_coefficients)
            })
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AV01 => {
            let data = unsafe { &sample_entry.data.av01 };
            f.object(|f| {
                f.member("kind", "av01")?;
                f.member("width", data.width)?;
                f.member("height", data.height)?;
                f.member("seqProfile", data.seq_profile)?;
                f.member("seqLevelIdx0", data.seq_level_idx_0)?;
                f.member("seqTier0", data.seq_tier_0)?;
                f.member("highBitdepth", data.high_bitdepth)?;
                f.member("twelveBit", data.twelve_bit)?;
                f.member("monochrome", data.monochrome)?;
                f.member("chromaSubsamplingX", data.chroma_subsampling_x)?;
                f.member("chromaSubsamplingY", data.chroma_subsampling_y)?;
                f.member("chromaSamplePosition", data.chroma_sample_position)?;
                if data.initial_presentation_delay_present {
                    f.member(
                        "initialPresentationDelayMinusOne",
                        data.initial_presentation_delay_minus_one,
                    )?;
                }
                let config_obus = unsafe {
                    std::slice::from_raw_parts(data.config_obus, data.config_obus_size as usize)
                };
                f.member("configObus", config_obus)
            })
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_OPUS => {
            let data = unsafe { &sample_entry.data.opus };
            f.object(|f| {
                f.member("kind", "opus")?;
                f.member("channelCount", data.channel_count)?;
                f.member("sampleRate", data.sample_rate)?;
                f.member("sampleSize", data.sample_size)?;
                f.member("preSkip", data.pre_skip)?;
                f.member("inputSampleRate", data.input_sample_rate)?;
                f.member("outputGain", data.output_gain)
            })
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_MP4A => {
            let data = unsafe { &sample_entry.data.mp4a };
            f.object(|f| {
                f.member("kind", "mp4a")?;
                f.member("channelCount", data.channel_count)?;
                f.member("sampleRate", data.sample_rate)?;
                f.member("sampleSize", data.sample_size)?;
                f.member("bufferSizeDb", data.buffer_size_db)?;
                f.member("maxBitrate", data.max_bitrate)?;
                f.member("avgBitrate", data.avg_bitrate)?;
                let dec_specific_info = unsafe {
                    std::slice::from_raw_parts(
                        data.dec_specific_info,
                        data.dec_specific_info_size as usize,
                    )
                };
                f.member("decSpecificInfo", dec_specific_info)
            })
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_FLAC => {
            let data = unsafe { &sample_entry.data.flac };
            f.object(|f| {
                f.member("kind", "flac")?;
                f.member("channelCount", data.channel_count)?;
                f.member("sampleRate", data.sample_rate)?;
                f.member("sampleSize", data.sample_size)?;
                let streaminfo = unsafe {
                    std::slice::from_raw_parts(data.streaminfo_data, data.streaminfo_size as usize)
                };
                f.member("streaminfoData", streaminfo)
            })
        }
    }
}

/// AVC SPS/PPS リストの JSON シリアライズ用構造体
struct JsonAvcNaluList {
    data_ptr: *const *const u8,
    sizes_ptr: *const u32,
    count: u32,
}

impl nojson::DisplayJson for JsonAvcNaluList {
    fn fmt(&self, f: &mut nojson::JsonFormatter<'_, '_>) -> std::fmt::Result {
        f.array(|f| {
            for i in 0..self.count as usize {
                let nalu_ptr = unsafe { *self.data_ptr.add(i) };
                let nalu_size = unsafe { *self.sizes_ptr.add(i) } as usize;
                let nalu = unsafe { std::slice::from_raw_parts(nalu_ptr, nalu_size) };
                f.element(nalu)?;
            }
            Ok(())
        })
    }
}

/*
    // TODO: テストは shiguredo_mp4 に依存しない形に書き換える
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
*/
