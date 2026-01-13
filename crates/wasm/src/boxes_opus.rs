//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（opus 用）

use c_api::boxes::Mp4SampleEntryOpus;

/// Opus サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_opus(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opus_to_json() {
        let sample_entry = Mp4SampleEntryOpus {
            channel_count: 2,
            sample_rate: 48000,
            sample_size: 16,
            pre_skip: 312,
            input_sample_rate: 48000,
            output_gain: 0,
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_opus(f, &sample_entry)).to_string();
        assert!(json.contains(r#""kind":"opus""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":48000"#));
        assert!(json.contains(r#""preSkip":312"#));
        assert!(json.contains(r#""inputSampleRate":48000"#));
        assert!(json.contains(r#""outputGain":0"#));
    }
}
