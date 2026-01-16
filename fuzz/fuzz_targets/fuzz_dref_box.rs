#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::DrefBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((dref, _)) = DrefBox::decode(data) {
        let _ = dref.encode_to_vec();
    }
});
