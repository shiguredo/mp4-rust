use std::io::Read;

use shiguredo_mp4::{Decode, Mp4File};

#[test]
fn decode_black_h264_video_mp4() -> std::io::Result<()> {
    let input_bytes = include_bytes!("testdata/black-h264-video.mp4");
    let file = Mp4File::decode(&input_bytes[..])?;
    Ok(())
}
