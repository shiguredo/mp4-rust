//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（avc1 ボックス用）
use c_api::boxes::Mp4SampleEntryAvc1;

/// AVC1（H.264）サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_avc1(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryAvc1,
) -> std::fmt::Result {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avc1_to_json() {
        static SPS: &[u8] = &[0x67, 0x64, 0x00, 0x28];
        static PPS: &[u8] = &[0x68, 0xee, 0x3c, 0x80];

        let sps_data = [SPS.as_ptr()];
        let sps_sizes = [SPS.len() as u32];
        let pps_data = [PPS.as_ptr()];
        let pps_sizes = [PPS.len() as u32];

        let avc1 = Mp4SampleEntryAvc1 {
            width: 1920,
            height: 1080,
            avc_profile_indication: 100,
            profile_compatibility: 0,
            avc_level_indication: 40,
            length_size_minus_one: 3,
            sps_data: sps_data.as_ptr(),
            sps_sizes: sps_sizes.as_ptr(),
            sps_count: 1,
            pps_data: pps_data.as_ptr(),
            pps_sizes: pps_sizes.as_ptr(),
            pps_count: 1,
            is_chroma_format_present: false,
            chroma_format: 0,
            is_bit_depth_luma_minus8_present: false,
            bit_depth_luma_minus8: 0,
            is_bit_depth_chroma_minus8_present: false,
            bit_depth_chroma_minus8: 0,
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_avc1(f, &avc1)).to_string();
        assert!(json.contains(r#""kind":"avc1""#));
        assert!(json.contains(r#""width":1920"#));
        assert!(json.contains(r#""height":1080"#));
        assert!(json.contains(r#""avcProfileIndication":100"#));
        assert!(json.contains(r#""profileCompatibility":0"#));
        assert!(json.contains(r#""avcLevelIndication":40"#));
        assert!(json.contains(r#""lengthSizeMinusOne":3"#));
        assert!(json.contains(r#""sps":"#));
        assert!(json.contains(r#""pps":"#));
    }
}
