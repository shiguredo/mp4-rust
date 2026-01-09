//! C API の demux.rs に対応するモジュール
use c_api::basic_types::Mp4TrackKind;
use c_api::demux::{Mp4DemuxSample, Mp4DemuxTrackInfo};

/// トラック情報を JSON 文字列に変換する
//
/// # 引数
///
/// - `track_info`: 変換対象の Mp4DemuxTrackInfo へのポインタ
///
/// # 戻り値
///
/// JSON 文字列を含む Vec<u8> へのポインタ。エラー時は NULL
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demux_track_info_to_json(
    track_info: *const Mp4DemuxTrackInfo,
) -> *mut Vec<u8> {
    if track_info.is_null() {
        return std::ptr::null_mut();
    }

    let track_info = unsafe { &*track_info };
    let json = nojson::json(|f| fmt_json_mp4_demux_track_info(f, track_info)).to_string();
    Box::into_raw(Box::new(json.into_bytes()))
}

/// TODO: doc
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_demux_sample_to_json(sample: *const Mp4DemuxSample) -> *mut Vec<u8> {
    if sample.is_null() {
        return std::ptr::null_mut();
    }

    todo!()
}

fn fmt_json_mp4_demux_track_info(
    f: &mut nojson::JsonFormatter<'_, '_>,
    track_info: &Mp4DemuxTrackInfo,
) -> std::fmt::Result {
    let kind = match track_info.kind {
        Mp4TrackKind::MP4_TRACK_KIND_AUDIO => "audio",
        Mp4TrackKind::MP4_TRACK_KIND_VIDEO => "video",
    };

    f.object(|f| {
        f.member("track_id", track_info.track_id)?;
        f.member("kind", kind)?;
        f.member("duration", track_info.duration)?;
        f.member("timescale", track_info.timescale)
    })
}
