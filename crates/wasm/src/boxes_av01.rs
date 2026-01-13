//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（av01 用）

use c_api::boxes::Mp4SampleEntryAv01;

/// AV01（AV1）サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_av01(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_av01_to_json() {
        static CONFIG_OBUS: &[u8] = &[0x0a, 0x0b, 0x00, 0x00];

        let sample_entry = Mp4SampleEntryAv01 {
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

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_av01(f, &sample_entry)).to_string();
        assert!(json.contains(r#""kind":"av01""#));
        assert!(json.contains(r#""width":3840"#));
        assert!(json.contains(r#""height":2160"#));
        assert!(json.contains(r#""seqProfile":0"#));
        assert!(json.contains(r#""seqLevelIdx0":13"#));
        assert!(json.contains(r#""seqTier0":0"#));
        assert!(json.contains(r#""configObus":"#));
    }
}
