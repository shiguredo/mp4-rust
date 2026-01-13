//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（vp08 用）

use c_api::boxes::Mp4SampleEntryVp08;

/// VP08（VP8）サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_vp08(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vp08_to_json() {
        let sample_entry = Mp4SampleEntryVp08 {
            width: 1920,
            height: 1080,
            bit_depth: 8,
            chroma_subsampling: 1, // 4:2:0
            video_full_range_flag: false,
            colour_primaries: 1,         // BT.709
            transfer_characteristics: 1, // BT.709
            matrix_coefficients: 1,      // BT.709
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_vp08(f, &sample_entry)).to_string();
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
}
