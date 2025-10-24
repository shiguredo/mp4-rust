#![expect(missing_docs, dead_code)]

use crate::Error;

#[derive(Debug, Clone)]
pub struct Mp4FileMuxerOptions {
    pub audio_track: bool, // TODO: Option<TrackInfo>,
    pub video_track: bool, // TODO: Option<TrackInfo>,
    pub reserved_moov_box_size: usize,
}

impl Default for Mp4FileMuxerOptions {
    fn default() -> Self {
        Self {
            audio_track: true,
            video_track: true,
            reserved_moov_box_size: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sample {}

#[derive(Debug)]
pub enum MuxError {
    EncodeError(Error),
}

#[derive(Debug)]
pub struct Mp4FileMuxer {
    options: Mp4FileMuxerOptions,
}

impl Mp4FileMuxer {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_options(Mp4FileMuxerOptions::default())
    }

    pub fn with_options(options: Mp4FileMuxerOptions) -> Self {
        Self { options }
    }

    pub fn header_bytes(&self) -> &[u8] {
        todo!()
    }

    pub fn append_sample(&mut self, _sample: &Sample) -> Result<(), MuxError> {
        todo!()
    }

    pub fn finalize(self) {}
}
