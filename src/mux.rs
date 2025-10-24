#![expect(missing_docs, dead_code)]

#[derive(Debug, Clone)]
pub struct Mp4FileMuxerOptions {
    pub reserved_moov_box_size: usize,
}

#[expect(clippy::derivable_impls)]
impl Default for Mp4FileMuxerOptions {
    fn default() -> Self {
        Self {
            reserved_moov_box_size: 0,
        }
    }
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
}
