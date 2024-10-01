use orfail::OrFail;
use serde::{Deserialize, Serialize};
use shiguredo_mp4::{Decode, Mp4File};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeOptions {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeProgress {
    pub transcoded_frames: usize,
    pub total_frames: usize,
}

#[derive(Debug)]
pub struct Transcoder {
    options: TranscodeOptions,
    input_mp4: Option<Mp4File>,
}

impl Transcoder {
    pub fn new(options: TranscodeOptions) -> Self {
        Self {
            options,
            input_mp4: None,
        }
    }

    pub fn parse_input_mp4_file(&mut self, mut mp4: &[u8]) -> orfail::Result<()> {
        self.input_mp4 = Some(Mp4File::decode(&mut mp4).or_fail()?);
        Ok(())
    }

    pub fn start_transcode(&mut self) -> orfail::Result<()> {
        let _ = self.options;
        Ok(())
    }

    pub fn poll_transcode(&mut self) -> orfail::Result<TranscodeProgress> {
        Ok(TranscodeProgress {
            transcoded_frames: 0,
            total_frames: 0,
        })
    }

    pub fn build_output_mp4_file(&mut self) -> orfail::Result<()> {
        todo!()
    }

    pub fn take_output_mp4_file(&mut self) -> Vec<u8> {
        todo!()
    }
}
