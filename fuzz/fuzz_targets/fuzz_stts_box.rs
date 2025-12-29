#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::SttsBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((stts, _)) = SttsBox::decode(data) {
        let _ = stts.encode_to_vec();
    }
});
