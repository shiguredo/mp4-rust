use serde::{Deserialize, Serialize};

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
}

impl Transcoder {
    pub fn new(options: TranscodeOptions) -> Self {
        Self { options }
    }

    pub fn parse_input_mp4_file(&mut self, mp4: &[u8]) -> orfail::Result<()> {
        Ok(())
    }

    pub fn start_transcode(&mut self) -> orfail::Result<()> {
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
