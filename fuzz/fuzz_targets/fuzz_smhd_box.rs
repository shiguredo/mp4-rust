#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::SmhdBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((smhd, _)) = SmhdBox::decode(data) {
        let _ = smhd.encode_to_vec();
    }
});
