#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::UnknownBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((unknown, _)) = UnknownBox::decode(data) {
        let _ = unknown.encode_to_vec();
    }
});
