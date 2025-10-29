//! ../../../src/basic_types.rs の C API を定義するためのモジュール
use shiguredo_mp4::TrackKind;

#[derive(Clone, Copy)]
#[repr(C)]
#[expect(non_camel_case_types)]
pub enum Mp4TrackKind {
    /// 音声トラック
    MP4_TRACK_KIND_AUDIO = 0,

    /// 映像トラック
    MP4_TRACK_KIND_VIDEO = 1,
}

impl From<TrackKind> for Mp4TrackKind {
    fn from(kind: TrackKind) -> Self {
        match kind {
            TrackKind::Audio => Self::MP4_TRACK_KIND_AUDIO,
            TrackKind::Video => Self::MP4_TRACK_KIND_VIDEO,
        }
    }
}

impl From<Mp4TrackKind> for TrackKind {
    fn from(kind: Mp4TrackKind) -> Self {
        match kind {
            Mp4TrackKind::MP4_TRACK_KIND_AUDIO => Self::Audio,
            Mp4TrackKind::MP4_TRACK_KIND_VIDEO => Self::Video,
        }
    }
}
