use shiguredo_mp4::{Decode, Encode, IterUnknownBoxes, Mp4File, Result};

#[test]
fn decode_encode_black_h264_video_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/black-h264-video.mp4");
    let file: Mp4File = Mp4File::decode(&mut &input_bytes[..])?;

    // デコード時に未処理のボックスがないことを確認する。
    assert_eq!(
        file.iter_unknown_boxes().map(|x| x.0).collect::<Vec<_>>(),
        Vec::new()
    );

    let mut output_bytes = Vec::new();
    file.encode(&mut output_bytes)?;

    // エンコード結果が正しいことを確認する。
    // ボックスの順番は入れ替わる可能性があるので、バイト列をソートした上で比較する。
    let mut input_bytes = input_bytes.to_vec();
    input_bytes.sort();
    output_bytes.sort();
    assert_eq!(input_bytes.len(), output_bytes.len());
    assert_eq!(input_bytes, output_bytes);

    Ok(())
}
