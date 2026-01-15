//! Demuxer の Property-Based Testing
//!
//! 破損した MP4 データで無限ループが発生する問題を再現・検出するテスト

use proptest::prelude::*;
use shiguredo_mp4::demux::{Input, Mp4FileDemuxer, RequiredInput};

/// テスト用の簡易 MP4 風データ
const TEST_MP4_H264: &[u8] = &[
    0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00, 0x00, 0x00,
    b'i', b's', b'o', b'm', b'i', b's', b'o', b'2', 0x00, 0x00, 0x00, 0x08, b'm', b'o', b'o', b'v',
    0x00, 0x00, 0x00, 0x10, b'm', b'd', b'a', b't', 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
];
const TEST_MP4_AAC: &[u8] = &[
    0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'm', b'p', b'4', b'2', 0x00, 0x00, 0x00, 0x00,
    b'm', b'p', b'4', b'2', b'i', b's', b'o', b'm', 0x00, 0x00, 0x00, 0x08, b'm', b'o', b'o', b'v',
    0x00, 0x00, 0x00, 0x10, b'm', b'd', b'a', b't', 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
];

/// 破損の種類
#[derive(Debug, Clone, Copy)]
enum CorruptionType {
    /// 単一バイトを変更
    SingleByte { position: usize, value: u8 },
    /// 複数バイトをゼロで埋める
    ZeroFill { start: usize, len: usize },
    /// 複数バイトをランダム値で埋める
    RandomFill { start: usize, values: [u8; 8] },
    /// バイトを削除（切り詰め）
    Truncate { new_len: usize },
}

/// MP4 データを破損させる
fn corrupt_mp4(data: &[u8], corruption: CorruptionType) -> Vec<u8> {
    let mut corrupted = data.to_vec();

    match corruption {
        CorruptionType::SingleByte { position, value } => {
            if position < corrupted.len() {
                corrupted[position] = value;
            }
        }
        CorruptionType::ZeroFill { start, len } => {
            let end = (start + len).min(corrupted.len());
            if start < corrupted.len() {
                for byte in &mut corrupted[start..end] {
                    *byte = 0;
                }
            }
        }
        CorruptionType::RandomFill { start, values } => {
            for (i, &v) in values.iter().enumerate() {
                if start + i < corrupted.len() {
                    corrupted[start + i] = v;
                }
            }
        }
        CorruptionType::Truncate { new_len } => {
            corrupted.truncate(new_len);
        }
    }

    corrupted
}

/// 破損タイプを生成する Strategy
fn arb_corruption(data_len: usize) -> impl Strategy<Value = CorruptionType> {
    prop_oneof![
        // 単一バイト変更
        (0..data_len, any::<u8>())
            .prop_map(|(position, value)| CorruptionType::SingleByte { position, value }),
        // ゼロ埋め
        (0..data_len, 1usize..=64).prop_map(|(start, len)| CorruptionType::ZeroFill { start, len }),
        // ランダム埋め
        (0..data_len, any::<[u8; 8]>())
            .prop_map(|(start, values)| CorruptionType::RandomFill { start, values }),
        // 切り詰め（最低 8 バイトは残す）
        (8usize..data_len).prop_map(|new_len| CorruptionType::Truncate { new_len }),
    ]
}

/// Demuxer が無限ループに陥らないことを確認する
///
/// 同じ RequiredInput が連続して返された場合は無限ループとみなす
fn demux_with_loop_detection(data: &[u8], max_iterations: usize) -> Result<(), String> {
    let mut demuxer = Mp4FileDemuxer::new();
    let mut last_required: Option<RequiredInput> = None;
    let mut same_request_count = 0;
    const MAX_SAME_REQUESTS: usize = 3;

    for iteration in 0..max_iterations {
        // 必要な入力を確認
        let required = demuxer.required_input();

        if let Some(req) = required {
            // 同じリクエストが繰り返されているかチェック
            if last_required == Some(req) {
                same_request_count += 1;
                if same_request_count >= MAX_SAME_REQUESTS {
                    return Err(format!(
                        "無限ループ検出: 同じ入力要求が {} 回繰り返された (position={}, size={:?}) at iteration {}",
                        same_request_count, req.position, req.size, iteration
                    ));
                }
            } else {
                same_request_count = 0;
                last_required = Some(req);
            }

            // データを提供
            let input = Input { position: 0, data };
            demuxer.handle_input(input);
        } else {
            // 初期化完了またはエラー
            break;
        }
    }

    // tracks() を呼んでエラーをチェック
    match demuxer.tracks() {
        Ok(_) => {
            // サンプルも読んでみる
            let mut sample_iterations = 0;
            loop {
                match demuxer.next_sample() {
                    Ok(Some(_)) => {
                        sample_iterations += 1;
                        if sample_iterations > max_iterations {
                            return Err("サンプル読み取りで反復回数超過".to_string());
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break, // エラーは許容（破損データなので）
                }
            }
            Ok(())
        }
        Err(_) => Ok(()), // エラーは許容（破損データなので）
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// 破損した H264 MP4 で無限ループが発生しないことを確認
    #[test]
    fn corrupted_h264_mp4_no_infinite_loop(corruption in arb_corruption(TEST_MP4_H264.len())) {
        let corrupted = corrupt_mp4(TEST_MP4_H264, corruption);
        let result = demux_with_loop_detection(&corrupted, 1000);
        prop_assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    /// 破損した AAC MP4 で無限ループが発生しないことを確認
    #[test]
    fn corrupted_aac_mp4_no_infinite_loop(corruption in arb_corruption(TEST_MP4_AAC.len())) {
        let corrupted = corrupt_mp4(TEST_MP4_AAC, corruption);
        let result = demux_with_loop_detection(&corrupted, 1000);
        prop_assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    /// 複数箇所を破損させた場合も無限ループが発生しないことを確認
    #[test]
    fn multi_corrupted_mp4_no_infinite_loop(
        corruptions in prop::collection::vec(arb_corruption(TEST_MP4_H264.len()), 1..5)
    ) {
        let mut data = TEST_MP4_H264.to_vec();
        for corruption in corruptions {
            data = corrupt_mp4(&data, corruption);
        }
        let result = demux_with_loop_detection(&data, 1000);
        prop_assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    /// ランダムバイト列で無限ループが発生しないことを確認
    #[test]
    fn random_bytes_no_infinite_loop(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let result = demux_with_loop_detection(&data, 100);
        prop_assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    /// ボックスヘッダー付近の破損で無限ループが発生しないことを確認
    #[test]
    fn header_corruption_no_infinite_loop(
        offset in 0usize..32,
        value in any::<u8>()
    ) {
        let corruption = CorruptionType::SingleByte { position: offset, value };
        let corrupted = corrupt_mp4(TEST_MP4_H264, corruption);
        let result = demux_with_loop_detection(&corrupted, 1000);
        prop_assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    /// サイズフィールドを極端な値に破損させた場合も無限ループが発生しないことを確認
    #[test]
    fn extreme_size_corruption_no_infinite_loop(
        size_bytes in prop::array::uniform4(any::<u8>())
    ) {
        // 最初の 4 バイト（ftyp のサイズフィールド）を破損
        let mut corrupted = TEST_MP4_H264.to_vec();
        if corrupted.len() >= 4 {
            corrupted[0..4].copy_from_slice(&size_bytes);
        }
        let result = demux_with_loop_detection(&corrupted, 1000);
        prop_assert!(result.is_ok(), "Error: {:?}", result.err());
    }
}

mod boundary_tests {
    use super::*;

    /// 空データでパニックしない
    #[test]
    fn empty_data_no_panic() {
        let result = demux_with_loop_detection(&[], 100);
        assert!(result.is_ok());
    }

    /// 最小サイズのデータでパニックしない
    #[test]
    fn minimal_data_no_panic() {
        let result = demux_with_loop_detection(&[0; 8], 100);
        assert!(result.is_ok());
    }

    /// ftyp だけのデータ
    #[test]
    fn ftyp_only_no_infinite_loop() {
        // ftyp ボックスのみ（moov がない）
        let ftyp_only = &TEST_MP4_H264[..32.min(TEST_MP4_H264.len())];
        let result = demux_with_loop_detection(ftyp_only, 100);
        assert!(result.is_ok());
    }

    /// 全バイト 0xFF
    #[test]
    fn all_ff_no_infinite_loop() {
        let data = vec![0xFF; 256];
        let result = demux_with_loop_detection(&data, 100);
        assert!(result.is_ok());
    }

    /// 全バイト 0x00
    #[test]
    fn all_zero_no_infinite_loop() {
        let data = vec![0x00; 256];
        let result = demux_with_loop_detection(&data, 100);
        assert!(result.is_ok());
    }
}
