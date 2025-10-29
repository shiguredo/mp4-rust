//! ../../../src/basic_types.rs の C API を定義するためのモジュール
use shiguredo_mp4::TrackKind;

#[unsafe(no_mangle)]
pub extern "C" fn foo() -> Mp4TrackKind {
    Mp4TrackKind::Audio
}

#[derive(Clone, Copy)]
#[repr(C)]
pub enum Mp4TrackKind {
    /// 音声トラック
    Audio = 0,

    /// 映像トラック
    Video = 1,
}

impl From<TrackKind> for Mp4TrackKind {
    fn from(kind: TrackKind) -> Self {
        match kind {
            TrackKind::Audio => Self::Audio,
            TrackKind::Video => Self::Video,
        }
    }
}

impl From<Mp4TrackKind> for TrackKind {
    fn from(kind: Mp4TrackKind) -> Self {
        match kind {
            Mp4TrackKind::Audio => Self::Audio,
            Mp4TrackKind::Video => Self::Video,
        }
    }
}
