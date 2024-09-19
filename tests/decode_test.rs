use shiguredo_mp4::{boxes::RootBox, Decode, IterUnknownBoxes, Mp4File, Result};

#[test]
fn decode_black_h264_video_mp4() -> Result<()> {
    let input_bytes = include_bytes!("testdata/black-h264-video.mp4");
    let file = Mp4File::<RootBox>::decode(&mut &input_bytes[..])?;
    assert_eq!(
        file.iter_unknown_boxes().map(|x| x.0).collect::<Vec<_>>(),
        Vec::new()
    );
    Ok(())
}
