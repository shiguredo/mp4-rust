//! shiguredo_mp4 のエラーをまとめて定義するためのモジュール
//!
//! C API で細かくエラー方が分かれていると煩雑なので、ひとつに集約している
use shiguredo_mp4::{
    Error, ErrorKind, aux::SampleTableAccessorError, demux::DemuxError, mux::MuxError,
};

#[repr(C)]
pub enum Mp4Error {
    Ok = 0,
    InvalidInput,
    InvalidData,
    InvalidState,
    InputRequired,
    OutputRequired,
    NullPointer,
    NoMoreSamples,
    Other,
}

impl From<Error> for Mp4Error {
    fn from(e: Error) -> Self {
        match e.kind {
            ErrorKind::InvalidInput => Self::InvalidInput,
            ErrorKind::InvalidData => Self::InvalidData,
            _ => Self::Other,
        }
    }
}

impl From<SampleTableAccessorError> for Mp4Error {
    fn from(_e: SampleTableAccessorError) -> Self {
        Self::InvalidData
    }
}

impl From<DemuxError> for Mp4Error {
    fn from(e: DemuxError) -> Self {
        match e {
            DemuxError::DecodeError(e) => e.into(),
            DemuxError::SampleTableError(e) => e.into(),
            DemuxError::InputRequired(_) => Self::InputRequired,
            _ => Self::Other,
        }
    }
}

impl From<MuxError> for Mp4Error {
    fn from(e: MuxError) -> Self {
        match e {
            MuxError::EncodeError(e) => e.into(),
            MuxError::AlreadyFinalized => Self::InvalidState,
            MuxError::PositionMismatch { .. }
            | MuxError::MissingSampleEntry { .. }
            | MuxError::SampleDurationOverflow { .. } => Self::InvalidInput,
            _ => Self::Other,
        }
    }
}
