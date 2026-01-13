//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（flac 用）

use c_api::boxes::Mp4SampleEntryFlac;

/// FLAC サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_flac(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryFlac,
) -> std::fmt::Result {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flac_to_json() {
        static STREAMINFO: &[u8] = &[0x00, 0x10, 0x00, 0x10];

        let sample_entry = Mp4SampleEntryFlac {
            channel_count: 2,
            sample_rate: 44100,
            sample_size: 16,
            streaminfo_data: STREAMINFO.as_ptr(),
            streaminfo_size: STREAMINFO.len() as u32,
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_flac(f, &sample_entry)).to_string();
        assert!(json.contains(r#""kind":"flac""#));
        assert!(json.contains(r#""channelCount":2"#));
        assert!(json.contains(r#""sampleRate":44100"#));
        assert!(json.contains(r#""sampleSize":16"#));
        assert!(json.contains(r#""streaminfoData":"#));
    }
}
