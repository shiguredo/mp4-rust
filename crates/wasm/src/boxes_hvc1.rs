//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール（hvc1 用）

use c_api::boxes::Mp4SampleEntryHvc1;

/// HVC1（H.265/HEVC）サンプルエントリーを JSON フォーマットする
pub fn fmt_json_mp4_sample_entry_hvc1(
    f: &mut nojson::JsonFormatter<'_, '_>,
    data: &Mp4SampleEntryHvc1,
) -> std::fmt::Result {
    f.object(|f| {
        f.member("kind", "hvc1")?;
        f.member("width", data.width)?;
        f.member("height", data.height)?;
        f.member("generalProfileSpace", data.general_profile_space)?;
        f.member("generalTierFlag", data.general_tier_flag)?;
        f.member("generalProfileIdc", data.general_profile_idc)?;
        f.member(
            "generalProfileCompatibilityFlags",
            data.general_profile_compatibility_flags,
        )?;
        f.member(
            "generalConstraintIndicatorFlags",
            data.general_constraint_indicator_flags,
        )?;
        f.member("generalLevelIdc", data.general_level_idc)?;
        f.member("chromaFormatIdc", data.chroma_format_idc)?;
        f.member("bitDepthLumaMinus8", data.bit_depth_luma_minus8)?;
        f.member("bitDepthChromaMinus8", data.bit_depth_chroma_minus8)?;
        f.member(
            "minSpatialSegmentationIdc",
            data.min_spatial_segmentation_idc,
        )?;
        f.member("parallelismType", data.parallelism_type)?;
        f.member("avgFrameRate", data.avg_frame_rate)?;
        f.member("constantFrameRate", data.constant_frame_rate)?;
        f.member("numTemporalLayers", data.num_temporal_layers)?;
        f.member("temporalIdNested", data.temporal_id_nested)?;
        f.member("lengthSizeMinusOne", data.length_size_minus_one)?;
        f.member(
            "naluArrays",
            NaluArrays {
                nalu_types: data.nalu_types,
                nalu_counts: data.nalu_counts,
                nalu_data: data.nalu_data,
                nalu_sizes: data.nalu_sizes,
                nalu_array_count: data.nalu_array_count,
            },
        )
    })
}

/// NALU 配列の JSON シリアライズ用構造体
struct NaluArrays {
    nalu_types: *const u8,
    nalu_counts: *const u32,
    nalu_data: *const *const u8,
    nalu_sizes: *const u32,
    nalu_array_count: u32,
}

impl nojson::DisplayJson for NaluArrays {
    fn fmt(&self, f: &mut nojson::JsonFormatter<'_, '_>) -> std::fmt::Result {
        f.array(|f| {
            let mut nalu_index_base = 0u32;
            for i in 0..self.nalu_array_count as usize {
                let nalu_type = unsafe { *self.nalu_types.add(i) };
                let nalu_count = unsafe { *self.nalu_counts.add(i) };

                f.element(nojson::object(|f| {
                    f.member("naluType", nalu_type)?;
                    f.member(
                        "units",
                        nojson::array(|f| {
                            for j in 0..nalu_count {
                                let nalu_index = nalu_index_base + j;
                                let nalu_ptr = unsafe { *self.nalu_data.add(nalu_index as usize) };
                                let nalu_size =
                                    unsafe { *self.nalu_sizes.add(nalu_index as usize) } as usize;
                                let nalu =
                                    unsafe { std::slice::from_raw_parts(nalu_ptr, nalu_size) };
                                f.element(nalu)?;
                            }
                            Ok(())
                        }),
                    )
                }))?;

                nalu_index_base += nalu_count;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hvc1_to_json() {
        static VPS: &[u8] = &[0x40, 0x01, 0x0c, 0x01];
        static SPS: &[u8] = &[0x42, 0x01, 0x01, 0x01];
        static PPS: &[u8] = &[0x44, 0x01, 0x00];

        // NALU 配列を構築: VPS, SPS, PPS の順序で格納
        let nalu_types = [32u8, 33u8, 34u8]; // VPS=32, SPS=33, PPS=34
        let nalu_counts = [1u32, 1u32, 1u32];
        let mut nalu_data = Vec::new();
        let mut nalu_sizes_vec = Vec::new();

        nalu_data.push(VPS.as_ptr());
        nalu_sizes_vec.push(VPS.len() as u32);
        nalu_data.push(SPS.as_ptr());
        nalu_sizes_vec.push(SPS.len() as u32);
        nalu_data.push(PPS.as_ptr());
        nalu_sizes_vec.push(PPS.len() as u32);

        let sample_entry = Mp4SampleEntryHvc1 {
            width: 1920,
            height: 1080,
            general_profile_space: 0,
            general_tier_flag: 0,
            general_profile_idc: 2,
            general_profile_compatibility_flags: 0x60000000,
            general_constraint_indicator_flags: 0xb0000000_00000000,
            general_level_idc: 120,
            chroma_format_idc: 1,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            min_spatial_segmentation_idc: 0,
            parallelism_type: 0,
            avg_frame_rate: 0,
            constant_frame_rate: 0,
            num_temporal_layers: 1,
            temporal_id_nested: 0,
            length_size_minus_one: 3,
            nalu_array_count: 3,
            nalu_types: nalu_types.as_ptr(),
            nalu_counts: nalu_counts.as_ptr(),
            nalu_data: nalu_data.as_ptr(),
            nalu_sizes: nalu_sizes_vec.as_ptr(),
        };

        let json = nojson::json(|f| fmt_json_mp4_sample_entry_hvc1(f, &sample_entry)).to_string();
        assert!(json.contains(r#""kind":"hvc1""#));
        assert!(json.contains(r#""width":1920"#));
        assert!(json.contains(r#""height":1080"#));
        assert!(json.contains(r#""generalProfileIdc":2"#));
        assert!(json.contains(r#""generalLevelIdc":120"#));
        assert!(json.contains(r#""lengthSizeMinusOne":3"#));
        assert!(json.contains(r#""naluArrays":"#));
    }
}
