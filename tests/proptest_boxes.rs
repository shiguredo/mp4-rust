//! ボックス構造体の Property-Based Testing

use std::num::NonZeroU32;

use proptest::prelude::*;
use shiguredo_mp4::{
    boxes::{
        Brand, Co64Box, ElstBox, ElstEntry, FtypBox, StcoBox, StscBox, StscEntry, SttsBox,
        SttsEntry,
    },
    Decode, Encode, FixedPointNumber,
};

/// SttsEntry を生成する Strategy
fn arb_stts_entry() -> impl Strategy<Value = SttsEntry> {
    (any::<u32>(), any::<u32>()).prop_map(|(sample_count, sample_delta)| SttsEntry {
        sample_count,
        sample_delta,
    })
}

/// StscEntry を生成する Strategy
fn arb_stsc_entry() -> impl Strategy<Value = StscEntry> {
    (1u32..=u32::MAX, any::<u32>(), 1u32..=u32::MAX).prop_map(
        |(first_chunk, sample_per_chunk, sample_description_index)| StscEntry {
            first_chunk: NonZeroU32::new(first_chunk).unwrap(),
            sample_per_chunk,
            sample_description_index: NonZeroU32::new(sample_description_index).unwrap(),
        },
    )
}

/// ElstEntry (version 0 互換) を生成する Strategy
fn arb_elst_entry_v0() -> impl Strategy<Value = ElstEntry> {
    (
        0u64..=(u32::MAX as u64),
        (i32::MIN as i64)..=(i32::MAX as i64),
        any::<i16>(),
        any::<i16>(),
    )
        .prop_map(|(edit_duration, media_time, rate_int, rate_frac)| ElstEntry {
            edit_duration,
            media_time,
            media_rate: FixedPointNumber::new(rate_int, rate_frac),
        })
}

/// ElstEntry (version 1) を生成する Strategy
fn arb_elst_entry_v1() -> impl Strategy<Value = ElstEntry> {
    (any::<u64>(), any::<i64>(), any::<i16>(), any::<i16>()).prop_map(
        |(edit_duration, media_time, rate_int, rate_frac)| ElstEntry {
            edit_duration,
            media_time,
            media_rate: FixedPointNumber::new(rate_int, rate_frac),
        },
    )
}

/// 4 文字のブランド名を生成する Strategy
fn arb_brand() -> impl Strategy<Value = Brand> {
    prop::array::uniform4(0x20u8..=0x7Eu8).prop_map(Brand::new)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    // ===== SttsBox のテスト =====

    /// SttsBox の encode/decode roundtrip
    #[test]
    fn stts_box_roundtrip(entries in prop::collection::vec(arb_stts_entry(), 0..50)) {
        let stts = SttsBox { entries: entries.clone() };
        let encoded = stts.encode_to_vec().unwrap();
        let (decoded, size) = SttsBox::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.entries.len(), entries.len());
        for (orig, dec) in entries.iter().zip(decoded.entries.iter()) {
            prop_assert_eq!(orig.sample_count, dec.sample_count);
            prop_assert_eq!(orig.sample_delta, dec.sample_delta);
        }
    }

    /// SttsBox::from_sample_deltas の不変条件: 連続する同じ delta は集約される
    #[test]
    fn stts_from_sample_deltas_invariant(deltas in prop::collection::vec(any::<u32>(), 0..100)) {
        let stts = SttsBox::from_sample_deltas(deltas.iter().cloned());

        // 隣接エントリは異なる sample_delta を持つ
        for window in stts.entries.windows(2) {
            prop_assert_ne!(window[0].sample_delta, window[1].sample_delta,
                "隣接エントリが同じ sample_delta を持っている");
        }

        // sample_count の合計が元の deltas 数と一致
        let total_count: u32 = stts.entries.iter().map(|e| e.sample_count).sum();
        prop_assert_eq!(total_count as usize, deltas.len());
    }

    // ===== StscBox のテスト =====

    /// StscBox の encode/decode roundtrip
    #[test]
    fn stsc_box_roundtrip(entries in prop::collection::vec(arb_stsc_entry(), 0..50)) {
        let stsc = StscBox { entries: entries.clone() };
        let encoded = stsc.encode_to_vec().unwrap();
        let (decoded, size) = StscBox::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.entries.len(), entries.len());
        for (orig, dec) in entries.iter().zip(decoded.entries.iter()) {
            prop_assert_eq!(orig.first_chunk, dec.first_chunk);
            prop_assert_eq!(orig.sample_per_chunk, dec.sample_per_chunk);
            prop_assert_eq!(orig.sample_description_index, dec.sample_description_index);
        }
    }

    // ===== StcoBox のテスト =====

    /// StcoBox の encode/decode roundtrip
    #[test]
    fn stco_box_roundtrip(offsets in prop::collection::vec(any::<u32>(), 0..100)) {
        let stco = StcoBox { chunk_offsets: offsets.clone() };
        let encoded = stco.encode_to_vec().unwrap();
        let (decoded, size) = StcoBox::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.chunk_offsets, offsets);
    }

    // ===== Co64Box のテスト =====

    /// Co64Box の encode/decode roundtrip
    #[test]
    fn co64_box_roundtrip(offsets in prop::collection::vec(any::<u64>(), 0..100)) {
        let co64 = Co64Box { chunk_offsets: offsets.clone() };
        let encoded = co64.encode_to_vec().unwrap();
        let (decoded, size) = Co64Box::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.chunk_offsets, offsets);
    }

    // ===== ElstBox のテスト =====

    /// ElstBox (version 0) の encode/decode roundtrip
    #[test]
    fn elst_box_v0_roundtrip(entries in prop::collection::vec(arb_elst_entry_v0(), 0..20)) {
        let elst = ElstBox { entries: entries.clone() };
        let encoded = elst.encode_to_vec().unwrap();
        let (decoded, size) = ElstBox::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.entries.len(), entries.len());
        for (orig, dec) in entries.iter().zip(decoded.entries.iter()) {
            prop_assert_eq!(orig.edit_duration, dec.edit_duration);
            prop_assert_eq!(orig.media_time, dec.media_time);
            prop_assert_eq!(orig.media_rate.integer, dec.media_rate.integer);
            prop_assert_eq!(orig.media_rate.fraction, dec.media_rate.fraction);
        }
    }

    /// ElstBox (version 1) の encode/decode roundtrip
    #[test]
    fn elst_box_v1_roundtrip(entries in prop::collection::vec(arb_elst_entry_v1(), 0..20)) {
        let elst = ElstBox { entries: entries.clone() };
        let encoded = elst.encode_to_vec().unwrap();
        let (decoded, size) = ElstBox::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.entries.len(), entries.len());
        for (orig, dec) in entries.iter().zip(decoded.entries.iter()) {
            prop_assert_eq!(orig.edit_duration, dec.edit_duration);
            prop_assert_eq!(orig.media_time, dec.media_time);
            prop_assert_eq!(orig.media_rate.integer, dec.media_rate.integer);
            prop_assert_eq!(orig.media_rate.fraction, dec.media_rate.fraction);
        }
    }

    // ===== FtypBox のテスト =====

    /// FtypBox の encode/decode roundtrip
    #[test]
    fn ftyp_box_roundtrip(
        major_brand in arb_brand(),
        minor_version in any::<u32>(),
        compatible_brands in prop::collection::vec(arb_brand(), 0..10)
    ) {
        let ftyp = FtypBox {
            major_brand,
            minor_version,
            compatible_brands: compatible_brands.clone(),
        };
        let encoded = ftyp.encode_to_vec().unwrap();
        let (decoded, size) = FtypBox::decode(&encoded).unwrap();

        prop_assert_eq!(size, encoded.len());
        prop_assert_eq!(decoded.major_brand.get(), major_brand.get());
        prop_assert_eq!(decoded.minor_version, minor_version);
        prop_assert_eq!(decoded.compatible_brands.len(), compatible_brands.len());
        for (orig, dec) in compatible_brands.iter().zip(decoded.compatible_brands.iter()) {
            prop_assert_eq!(orig.get(), dec.get());
        }
    }
}

// ===== 境界値テスト =====

#[cfg(test)]
mod boundary_tests {
    use super::*;

    /// SttsBox: 空のエントリリスト
    #[test]
    fn stts_box_empty() {
        let stts = SttsBox { entries: vec![] };
        let encoded = stts.encode_to_vec().unwrap();
        let (decoded, _) = SttsBox::decode(&encoded).unwrap();
        assert!(decoded.entries.is_empty());
    }

    /// StcoBox: 空のオフセットリスト
    #[test]
    fn stco_box_empty() {
        let stco = StcoBox {
            chunk_offsets: vec![],
        };
        let encoded = stco.encode_to_vec().unwrap();
        let (decoded, _) = StcoBox::decode(&encoded).unwrap();
        assert!(decoded.chunk_offsets.is_empty());
    }

    /// Co64Box: 空のオフセットリスト
    #[test]
    fn co64_box_empty() {
        let co64 = Co64Box {
            chunk_offsets: vec![],
        };
        let encoded = co64.encode_to_vec().unwrap();
        let (decoded, _) = Co64Box::decode(&encoded).unwrap();
        assert!(decoded.chunk_offsets.is_empty());
    }

    /// ElstBox: 空のエントリリスト
    #[test]
    fn elst_box_empty() {
        let elst = ElstBox { entries: vec![] };
        let encoded = elst.encode_to_vec().unwrap();
        let (decoded, _) = ElstBox::decode(&encoded).unwrap();
        assert!(decoded.entries.is_empty());
    }

    /// SttsEntry: 最大値
    #[test]
    fn stts_entry_max_values() {
        let stts = SttsBox {
            entries: vec![SttsEntry {
                sample_count: u32::MAX,
                sample_delta: u32::MAX,
            }],
        };
        let encoded = stts.encode_to_vec().unwrap();
        let (decoded, _) = SttsBox::decode(&encoded).unwrap();
        assert_eq!(decoded.entries[0].sample_count, u32::MAX);
        assert_eq!(decoded.entries[0].sample_delta, u32::MAX);
    }

    /// StscEntry: 最小値 (NonZeroU32 の制約)
    #[test]
    fn stsc_entry_min_values() {
        let stsc = StscBox {
            entries: vec![StscEntry {
                first_chunk: NonZeroU32::new(1).unwrap(),
                sample_per_chunk: 0,
                sample_description_index: NonZeroU32::new(1).unwrap(),
            }],
        };
        let encoded = stsc.encode_to_vec().unwrap();
        let (decoded, _) = StscBox::decode(&encoded).unwrap();
        assert_eq!(decoded.entries[0].first_chunk.get(), 1);
        assert_eq!(decoded.entries[0].sample_per_chunk, 0);
        assert_eq!(decoded.entries[0].sample_description_index.get(), 1);
    }

    /// Co64Box: u64 最大値
    #[test]
    fn co64_box_max_offset() {
        let co64 = Co64Box {
            chunk_offsets: vec![u64::MAX],
        };
        let encoded = co64.encode_to_vec().unwrap();
        let (decoded, _) = Co64Box::decode(&encoded).unwrap();
        assert_eq!(decoded.chunk_offsets[0], u64::MAX);
    }

    /// ElstEntry: version 0 と version 1 の境界
    #[test]
    fn elst_entry_version_boundary() {
        // version 0 の最大値
        let elst_v0 = ElstBox {
            entries: vec![ElstEntry {
                edit_duration: u32::MAX as u64,
                media_time: i32::MAX as i64,
                media_rate: FixedPointNumber::new(i16::MAX, i16::MAX),
            }],
        };
        let encoded_v0 = elst_v0.encode_to_vec().unwrap();
        let (decoded_v0, _) = ElstBox::decode(&encoded_v0).unwrap();
        assert_eq!(decoded_v0.entries[0].edit_duration, u32::MAX as u64);

        // version 1 が必要な値
        let elst_v1 = ElstBox {
            entries: vec![ElstEntry {
                edit_duration: (u32::MAX as u64) + 1,
                media_time: (i32::MAX as i64) + 1,
                media_rate: FixedPointNumber::new(0, 0),
            }],
        };
        let encoded_v1 = elst_v1.encode_to_vec().unwrap();
        let (decoded_v1, _) = ElstBox::decode(&encoded_v1).unwrap();
        assert_eq!(decoded_v1.entries[0].edit_duration, (u32::MAX as u64) + 1);
    }

    /// FtypBox: ブランドの境界値
    #[test]
    fn ftyp_box_brand_boundary() {
        let ftyp = FtypBox {
            major_brand: Brand::new([0x00, 0x00, 0x00, 0x00]),
            minor_version: 0,
            compatible_brands: vec![Brand::new([0xFF, 0xFF, 0xFF, 0xFF])],
        };
        let encoded = ftyp.encode_to_vec().unwrap();
        let (decoded, _) = FtypBox::decode(&encoded).unwrap();
        assert_eq!(decoded.major_brand.get(), [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(decoded.compatible_brands[0].get(), [0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
