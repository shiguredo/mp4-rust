//! C API の mux.rs に対応するモジュール
use c_api::mux::Mp4MuxSample;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_mux_sample_from_json(
    _json_bytes: *const u8,
    _json_bytes_len: u32,
) -> *mut Mp4MuxSample {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_mux_sample_free(sample: *mut Mp4MuxSample) {
    if !sample.is_null() {
        let _ = unsafe { Box::from_raw(sample) };
    }
}
