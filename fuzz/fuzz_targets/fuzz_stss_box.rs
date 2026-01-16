#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::StssBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((stss, _)) = StssBox::decode(data) {
        let _ = stss.encode_to_vec();
    }
});
