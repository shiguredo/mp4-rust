//! shiguredo_mp4 のエラーをまとめて定義するためのモジュール
//!
//! C API で細かくエラー方が分かれていると煩雑なので、ひとつに集約している
use shiguredo_mp4::{
    Error, ErrorKind, aux::SampleTableAccessorError, demux::DemuxError, mux::MuxError,
};

#[repr(C)]
#[expect(non_camel_case_types)]
pub enum Mp4Error {
    MP4_ERROR_OK = 0,
    MP4_ERROR_INVALID_INPUT,
    MP4_ERROR_INVALID_DATA,
    MP4_ERROR_INVALID_STATE,
    MP4_ERROR_INPUT_REQUIRED,
    MP4_ERROR_OUTPUT_REQUIRED,
    MP4_ERROR_NULL_POINTER,
    MP4_ERROR_NO_MORE_SAMPLES,
    MP4_ERROR_UNSUPPORTED,
    MP4_ERROR_OTHER,
}

impl From<Error> for Mp4Error {
    fn from(e: Error) -> Self {
        match e.kind {
            ErrorKind::InvalidInput => Self::MP4_ERROR_INVALID_INPUT,
            ErrorKind::InvalidData => Self::MP4_ERROR_INVALID_DATA,
            ErrorKind::Unsupported => Self::MP4_ERROR_UNSUPPORTED,
            _ => Self::MP4_ERROR_OTHER,
        }
    }
}

impl From<SampleTableAccessorError> for Mp4Error {
    fn from(_e: SampleTableAccessorError) -> Self {
        Self::MP4_ERROR_INVALID_DATA
    }
}

impl From<DemuxError> for Mp4Error {
    fn from(e: DemuxError) -> Self {
        match e {
            DemuxError::DecodeError(e) => e.into(),
            DemuxError::SampleTableError(e) => e.into(),
            DemuxError::InputRequired(_) => Self::MP4_ERROR_INPUT_REQUIRED,
            _ => Self::MP4_ERROR_OTHER,
        }
    }
}

impl From<MuxError> for Mp4Error {
    fn from(e: MuxError) -> Self {
        match e {
            MuxError::EncodeError(e) => e.into(),
            MuxError::AlreadyFinalized => Self::MP4_ERROR_INVALID_STATE,
            MuxError::PositionMismatch { .. }
            | MuxError::MissingSampleEntry { .. }
            | MuxError::SampleDurationOverflow { .. } => Self::MP4_ERROR_INVALID_INPUT,
            _ => Self::MP4_ERROR_OTHER,
        }
    }
}
