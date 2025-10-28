//! ../../../src/basic_types.rs の C API を定義するためのモジュール
use shiguredo_mp4::TrackKind;

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
