#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::VpccBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((vpcc, _)) = VpccBox::decode(data) {
        let _ = vpcc.encode_to_vec();
    }
});
