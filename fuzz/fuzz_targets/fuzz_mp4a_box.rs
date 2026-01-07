#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::Mp4aBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((mp4a, _)) = Mp4aBox::decode(data) {
        let _ = mp4a.encode_to_vec();
    }
});
