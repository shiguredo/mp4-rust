#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::HdlrBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((hdlr, _)) = HdlrBox::decode(data) {
        let _ = hdlr.encode_to_vec();
    }
});
