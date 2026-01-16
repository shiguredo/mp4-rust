#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::DopsBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((dops, _)) = DopsBox::decode(data) {
        let _ = dops.encode_to_vec();
    }
});
