#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::Co64Box, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((co64, _)) = Co64Box::decode(data) {
        let _ = co64.encode_to_vec();
    }
});
