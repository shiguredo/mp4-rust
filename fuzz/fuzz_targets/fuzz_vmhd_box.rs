#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_mp4::{boxes::VmhdBox, Decode, Encode};

fuzz_target!(|data: &[u8]| {
    if let Ok((vmhd, _)) = VmhdBox::decode(data) {
        let _ = vmhd.encode_to_vec();
    }
});
