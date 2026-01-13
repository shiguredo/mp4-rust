//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（mp4a 用）

use c_api::boxes::Mp4SampleEntryMp4a;

/// MP4A（AAC）サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_mp4a(
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

    #[test]
    fn test_mp4a_to_json() {
        // MP4A（AAC）サンプルエントリーの作成
        // dec_specific_info は AAC-LC の場合の典型的な値
        static DEC_SPECIFIC_INFO: &[u8] = &[0x12, 0x10];

        let sample_entry = Mp4SampleEntryMp4a {
            channel_count: 2,
            sample_rate: 44100,
            sample_size: 16,
            buffer_size_db: 0,
            max_bitrate: 128000,
            avg_bitrate: 128000,
            dec_specific_info: DEC_SPECIFIC_INFO.as_ptr(),
            dec_specific_info_size: DEC_SPECIFIC_INFO.len() as u32,
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_mp4a(f, &sample_entry)).to_string();
        assert!(json.contains(r#""kind":"mp4a""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":44100"#));
        assert!(json.contains(r#""sampleSize":16"#));
        assert!(json.contains(r#""bufferSizeDb":0"#));
        assert!(json.contains(r#""maxBitrate":128000"#));
        assert!(json.contains(r#""avgBitrate":128000"#));
        assert!(json.contains(r#""decSpecificInfo":"#));
    }
}
