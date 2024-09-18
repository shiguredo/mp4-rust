use shiguredo_mp4::{Decode, Mp4File, RawBox};

#[test]
fn decode_black_h264_video_mp4() -> std::io::Result<()> {
    let input_bytes = include_bytes!("testdata/black-h264-video.mp4");
    let file = Mp4File::<RawBox>::decode(&input_bytes[..])?;
    assert_eq!(file.boxes.len(), 2); // TODO
    Ok(())
}
