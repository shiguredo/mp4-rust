//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール

use c_api::boxes::{
    Mp4SampleEntry, Mp4SampleEntryAv01, Mp4SampleEntryKind, Mp4SampleEntryMp4a, Mp4SampleEntryOpus,
    Mp4SampleEntryVp08, Mp4SampleEntryVp09,
};

pub fn fmt_json_mp4_sample_entry(
    f: &mut nojson::JsonFormatter<'_, '_>,
    sample_entry: &Mp4SampleEntry,
) -> std::fmt::Result {
    match sample_entry.kind {
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AVC1 => {
            let data = unsafe { &sample_entry.data.avc1 };
            crate::boxes_avc1::fmt_json_mp4_sample_entry_avc1(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_HEV1 => {
            let data = unsafe { &sample_entry.data.hev1 };
            crate::boxes_hev1::fmt_json_mp4_sample_entry_hev1(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_HVC1 => {
            let data = unsafe { &sample_entry.data.hvc1 };
            crate::fmt_json_mp4_sample_entry_hvc1(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP08 => {
            let data = unsafe { &sample_entry.data.vp08 };
            fmt_json_mp4_sample_entry_vp08(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP09 => {
            let data = unsafe { &sample_entry.data.vp09 };
            fmt_json_mp4_sample_entry_vp09(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AV01 => {
            let data = unsafe { &sample_entry.data.av01 };
            fmt_json_mp4_sample_entry_av01(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_OPUS => {
            let data = unsafe { &sample_entry.data.opus };
            fmt_json_mp4_sample_entry_opus(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_MP4A => {
            let data = unsafe { &sample_entry.data.mp4a };
            fmt_json_mp4_sample_entry_mp4a(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_FLAC => {
            let data = unsafe { &sample_entry.data.flac };
            crate::boxes_flac::fmt_json_mp4_sample_entry_flac(f, data)?;
        }
    }
    Ok(())
}

/// VP08（VP8）サンプルエントリーを JSON フォーマットする
fn fmt_json_mp4_sample_entry_vp08(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryVp08,
) -> std::fmt::Result {
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

/// VP09（VP9）サンプルエントリーを JSON フォーマットする
fn fmt_json_mp4_sample_entry_vp09(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryVp09,
) -> std::fmt::Result {
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

/// AV01（AV1）サンプルエントリーを JSON フォーマットする
fn fmt_json_mp4_sample_entry_av01(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryAv01,
) -> std::fmt::Result {
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
        let config_obus =
            unsafe { std::slice::from_raw_parts(data.config_obus, data.config_obus_size as usize) };
        f.member("configObus", config_obus)
    })
}

/// Opus サンプルエントリーを JSON フォーマットする
fn fmt_json_mp4_sample_entry_opus(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryOpus,
) -> std::fmt::Result {
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

/// MP4A（AAC）サンプルエントリーを JSON フォーマットする
fn fmt_json_mp4_sample_entry_mp4a(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryMp4a,
) -> std::fmt::Result {
    f.object(|f| {
        f.member("kind", "mp4a")?;
        f.member("channelCount", data.channel_count)?;
        f.member("sampleRate", data.sample_rate)?;
        f.member("sampleSize", data.sample_size)?;
        f.member("bufferSizeDb", data.buffer_size_db)?;
        f.member("maxBitrate", data.max_bitrate)?;
        f.member("avgBitrate", data.avg_bitrate)?;
        let dec_specific_info = unsafe {
            std::slice::from_raw_parts(data.dec_specific_info, data.dec_specific_info_size as usize)
        };
        f.member("decSpecificInfo", dec_specific_info)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use c_api::boxes::Mp4SampleEntryData;

    #[test]
    fn test_opus_to_json() {
        let opus_data = Mp4SampleEntryOpus {
            channel_count: 2,
            sample_rate: 48000,
            sample_size: 16,
            pre_skip: 312,
            input_sample_rate: 48000,
            output_gain: 0,
        };

        let sample_entry = Mp4SampleEntry {
            kind: Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_OPUS,
            data: Mp4SampleEntryData { opus: opus_data },
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry(f, &sample_entry)).to_string();

        assert!(json.contains(r#""kind":"opus""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":48000"#));
        assert!(json.contains(r#""preSkip":312"#));
        assert!(json.contains(r#""inputSampleRate":48000"#));
        assert!(json.contains(r#""outputGain":0"#));
    }

    #[test]
    fn test_mp4a_to_json() {
        // MP4A（AAC）サンプルエントリーの作成
        // dec_specific_info は AAC-LC の場合の典型的な値
        static DEC_SPECIFIC_INFO: &[u8] = &[0x12, 0x10];

        let mp4a_data = Mp4SampleEntryMp4a {
            channel_count: 2,
            sample_rate: 44100,
            sample_size: 16,
            buffer_size_db: 0,
            max_bitrate: 128000,
            avg_bitrate: 128000,
            dec_specific_info: DEC_SPECIFIC_INFO.as_ptr(),
            dec_specific_info_size: DEC_SPECIFIC_INFO.len() as u32,
        };

        let sample_entry = Mp4SampleEntry {
            kind: Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_MP4A,
            data: Mp4SampleEntryData { mp4a: mp4a_data },
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry(f, &sample_entry)).to_string();

        assert!(json.contains(r#""kind":"mp4a""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":44100"#));
        assert!(json.contains(r#""sampleSize":16"#));
        assert!(json.contains(r#""bufferSizeDb":0"#));
        assert!(json.contains(r#""maxBitrate":128000"#));
        assert!(json.contains(r#""avgBitrate":128000"#));
        assert!(json.contains(r#""decSpecificInfo":"#));
    }

    #[test]
    fn test_vp08_to_json() {
        let vp08_data = Mp4SampleEntryVp08 {
            width: 1920,
            height: 1080,
            bit_depth: 8,
            chroma_subsampling: 1, // 4:2:0
            video_full_range_flag: false,
            colour_primaries: 1,         // BT.709
            transfer_characteristics: 1, // BT.709
            matrix_coefficients: 1,      // BT.709
        };

        let sample_entry = Mp4SampleEntry {
            kind: Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP08,
            data: Mp4SampleEntryData { vp08: vp08_data },
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry(f, &sample_entry)).to_string();

        assert!(json.contains(r#""kind":"vp08""#));
        assert!(json.contains(r#""width":1920"#));
        assert!(json.contains(r#""height":1080"#));
        assert!(json.contains(r#""bitDepth":8"#));
        assert!(json.contains(r#""chromaSubsampling":1"#));
        assert!(json.contains(r#""videoFullRangeFlag":0"#));
        assert!(json.contains(r#""colourPrimaries":1"#));
        assert!(json.contains(r#""transferCharacteristics":1"#));
        assert!(json.contains(r#""matrixCoefficients":1"#));
    }

    #[test]
    fn test_vp09_to_json() {
        let vp09_data = Mp4SampleEntryVp09 {
            width: 1280,
            height: 720,
            profile: 0,
            level: 31,
            bit_depth: 8,
            chroma_subsampling: 1, // 4:2:0
            video_full_range_flag: false,
            colour_primaries: 1,         // BT.709
            transfer_characteristics: 1, // BT.709
            matrix_coefficients: 1,      // BT.709
        };

        let sample_entry = Mp4SampleEntry {
            kind: Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP09,
            data: Mp4SampleEntryData { vp09: vp09_data },
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry(f, &sample_entry)).to_string();

        assert!(json.contains(r#""kind":"vp09""#));
        assert!(json.contains(r#""width":1280"#));
        assert!(json.contains(r#""height":720"#));
        assert!(json.contains(r#""profile":0"#));
        assert!(json.contains(r#""level":31"#));
        assert!(json.contains(r#""bitDepth":8"#));
        assert!(json.contains(r#""chromaSubsampling":1"#));
        assert!(json.contains(r#""videoFullRangeFlag":0"#));
        assert!(json.contains(r#""colourPrimaries":1"#));
        assert!(json.contains(r#""transferCharacteristics":1"#));
        assert!(json.contains(r#""matrixCoefficients":1"#));
    }

    #[test]
    fn test_av01_to_json() {
        static CONFIG_OBUS: &[u8] = &[0x0a, 0x0b, 0x00, 0x00];

        let av01_data = Mp4SampleEntryAv01 {
            width: 3840,
            height: 2160,
            seq_profile: 0,
            seq_level_idx_0: 13,
            seq_tier_0: 0,
            high_bitdepth: 0,
            twelve_bit: 0,
            monochrome: 0,
            chroma_subsampling_x: 1,
            chroma_subsampling_y: 1,
            chroma_sample_position: 0,
            initial_presentation_delay_present: false,
            initial_presentation_delay_minus_one: 0,
            config_obus: CONFIG_OBUS.as_ptr(),
            config_obus_size: CONFIG_OBUS.len() as u32,
        };

        let sample_entry = Mp4SampleEntry {
            kind: Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AV01,
            data: Mp4SampleEntryData { av01: av01_data },
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry(f, &sample_entry)).to_string();

        assert!(json.contains(r#""kind":"av01""#));
        assert!(json.contains(r#""width":3840"#));
        assert!(json.contains(r#""height":2160"#));
        assert!(json.contains(r#""seqProfile":0"#));
        assert!(json.contains(r#""seqLevelIdx0":13"#));
        assert!(json.contains(r#""seqTier0":0"#));
        assert!(json.contains(r#""configObus":"#));
    }
}
