//! C API の mux.rs に対応するモジュール
use c_api::basic_types::Mp4TrackKind;
use c_api::boxes::Mp4SampleEntry;
use c_api::mux::Mp4MuxSample;

/// TODO: doc
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_mux_sample_from_json(
    json_bytes: *const u8,
    json_bytes_len: u32,
) -> *mut Mp4MuxSample {
    if json_bytes.is_null() {
        return std::ptr::null_mut();
    }

    let Ok(json_text) = std::str::from_utf8(unsafe {
        std::slice::from_raw_parts(json_bytes, json_bytes_len as usize)
    }) else {
        return std::ptr::null_mut();
    };

    let Ok(raw_json) = nojson::RawJson::parse(json_text) else {
        return std::ptr::null_mut();
    };

    let value = raw_json.value();
    let Ok(sample) = parse_json_mp4_mux_sample(value) else {
        return std::ptr::null_mut();
    };

    Box::into_raw(Box::new(sample))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_mux_sample_free(sample: *mut Mp4MuxSample) {
    if !sample.is_null() {
        let _ = unsafe { Box::from_raw(sample) };
    }
}

fn parse_json_mp4_mux_sample(
    value: nojson::RawJsonValue<'_, '_>,
) -> Result<Mp4MuxSample, nojson::JsonParseError> {
    let track_kind_value = value.to_member("track_kind")?.required()?;
    let track_kind = match track_kind_value.to_unquoted_string_str()?.as_ref() {
        "audio" => Mp4TrackKind::MP4_TRACK_KIND_AUDIO,
        "video" => Mp4TrackKind::MP4_TRACK_KIND_VIDEO,
        _ => return Err(track_kind_value.invalid("must be \"audio\" or \"video\"")),
    };

    let keyframe: bool = value.to_member("keyframe")?.required()?.try_into()?;
    let timescale: u32 = value.to_member("timescale")?.required()?.try_into()?;
    let duration: u32 = value.to_member("duration")?.required()?.try_into()?;
    let data_offset: u64 = value.to_member("data_offset")?.required()?.try_into()?;
    let data_size: u32 = value.to_member("data_size")?.required()?.try_into()?;

    // TODO: 後でちゃんと実装する
    let sample_entry: *const Mp4SampleEntry = std::ptr::null();

    Ok(Mp4MuxSample {
        track_kind,
        sample_entry,
        keyframe,
        timescale,
        duration,
        data_offset,
        data_size,
    })
}
