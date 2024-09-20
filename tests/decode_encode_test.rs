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

    // エンコード結果をデコードしたら同じ MP4 になっていることを確認する。
    let encoded_file: Mp4File = Mp4File::decode(&mut &output_bytes[..])?;
    assert_eq!(file, encoded_file);

    // エンコード結果のバイト列が正しいことを確認する。
    assert_eq!(input_bytes.len(), output_bytes.len());
    assert_eq!(&input_bytes[..], output_bytes);

    Ok(())
}
