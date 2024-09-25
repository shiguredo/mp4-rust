use shiguredo_mp4::{BoxType, Decode, Encode, Mp4File, Result};

#[test]
fn decode_encode_black_h264_video_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/black-h264-video.mp4");
    let file: Mp4File = Mp4File::decode(&mut &input_bytes[..])?;

    // デコード時に未処理のボックスがないことを確認する。
    assert_eq!(collect_unknown_box_types(&file), Vec::new());

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

#[test]
fn decode_encode_black_h265_video_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/black-h265-video.mp4");
    let file: Mp4File = Mp4File::decode(&mut &input_bytes[..])?;

    // デコード時に未処理のボックスがないことを確認する。
    assert_eq!(collect_unknown_box_types(&file), Vec::new());

    let mut output_bytes = Vec::new();
    file.encode(&mut output_bytes)?;

    // エンコード結果をデコードしたら同じ MP4 になっていることを確認する。
    let encoded_file: Mp4File = Mp4File::decode(&mut &output_bytes[..])?;
    assert_eq!(file, encoded_file);

    // エンコード結果のバイト列が正しいことを確認する。
    assert_eq!(input_bytes.len(), output_bytes.len());

    // ボックスの順番は入れ替わるのでソートした結果を比較する
    let mut input_bytes = input_bytes.to_vec();
    input_bytes.sort();
    output_bytes.sort();
    assert_eq!(&input_bytes[..], output_bytes);

    Ok(())
}

#[test]
fn decode_encode_black_vp9_video_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/black-vp9-video.mp4");
    let file: Mp4File = Mp4File::decode(&mut &input_bytes[..])?;

    // デコード時に未処理のボックスがないことを確認する。
    assert_eq!(collect_unknown_box_types(&file), Vec::new());

    let mut output_bytes = Vec::new();
    file.encode(&mut output_bytes)?;

    // エンコード結果をデコードしたら同じ MP4 になっていることを確認する。
    let encoded_file: Mp4File = Mp4File::decode(&mut &output_bytes[..])?;
    assert_eq!(file, encoded_file);

    // エンコード結果のバイト列が正しいことを確認する。
    assert_eq!(input_bytes.len(), output_bytes.len());

    // ボックスの順番は入れ替わるのでソートした結果を比較する
    let mut input_bytes = input_bytes.to_vec();
    input_bytes.sort();
    output_bytes.sort();
    assert_eq!(&input_bytes[..], output_bytes);

    Ok(())
}

#[test]
fn decode_encode_black_av1_video_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/black-av1-video.mp4");
    let file: Mp4File = Mp4File::decode(&mut &input_bytes[..])?;

    // デコード時に未処理のボックスがないことを確認する。
    assert_eq!(collect_unknown_box_types(&file), Vec::new());

    let mut output_bytes = Vec::new();
    file.encode(&mut output_bytes)?;

    // エンコード結果をデコードしたら同じ MP4 になっていることを確認する。
    let encoded_file: Mp4File = Mp4File::decode(&mut &output_bytes[..])?;
    assert_eq!(file, encoded_file);

    // エンコード結果のバイト列が正しいことを確認する。
    assert_eq!(input_bytes.len(), output_bytes.len());

    // ボックスの順番は入れ替わるのでソートした結果を比較する
    let mut input_bytes = input_bytes.to_vec();
    input_bytes.sort();
    output_bytes.sort();
    assert_eq!(&input_bytes[..], output_bytes);

    Ok(())
}

#[test]
fn decode_encode_beep_opus_audio_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/beep-opus-audio.mp4");
    let file: Mp4File = Mp4File::decode(&mut &input_bytes[..])?;

    // デコード時に未処理のボックスがないことを確認する。
    assert_eq!(collect_unknown_box_types(&file), Vec::new());

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

fn collect_unknown_box_types(mp4: &Mp4File) -> Vec<BoxType> {
    let mut stack = mp4.iter().collect::<Vec<_>>();
    let mut unknowns = Vec::new();

    while let Some(b) = stack.pop() {
        if b.is_unknown_box()
            && !matches!(
                b.box_type().as_bytes(),
                b"btrt" | b"ctts" | b"fiel" | b"pasp" | b"sbgp" | b"sdtp" | b"sgpd" | b"udta"
            )
        {
            unknowns.push(b.box_type());
        }
        stack.extend(b.children());
    }

    unknowns
}
