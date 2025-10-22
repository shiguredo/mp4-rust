#![allow(missing_docs)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mp4FileDemuxerAction {
    Read { size: usize, position: Option<u64> },
}

#[derive(Debug)]
pub struct Mp4FileDemuxer {}
