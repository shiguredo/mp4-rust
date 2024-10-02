use std::marker::PhantomData;

use orfail::OrFail;
use serde::{Deserialize, Serialize};
use transcode::{Codec, TranscodeOptions, TranscodeProgress};

pub mod mp4;
pub mod transcode;

pub struct WebCodec;

pub type CoderId = u32;
pub type Transcoder = transcode::Transcoder<WebCodec>;

#[expect(unused_variables, unreachable_code)]
impl Codec for WebCodec {
    type Coder = CoderId;

    fn create_h264_decoder(
        config: &shiguredo_mp4::boxes::Avc1Box,
    ) -> impl futures::Future<Output = orfail::Result<Self::Coder>> {
        futures::future::ok(todo!())
    }

    fn decode_sample(
        decoder: &mut Self::Coder,
        encoded_data: &[u8],
    ) -> impl futures::Future<Output = orfail::Result<Vec<u8>>> {
        futures::future::ok(todo!())
    }
}

#[no_mangle]
#[expect(non_snake_case)]
pub fn newTranscoder(options: JsonVec<TranscodeOptions>) -> *mut Transcoder {
    let options = unsafe { options.into_value() };
    Box::into_raw(Box::new(Transcoder::new(options)))
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn freeTranscoder(transcoder: *mut Transcoder) {
    let _ = unsafe { Box::from_raw(transcoder) };
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn parseInputMp4File(
    transcoder: *mut Transcoder,
    input_mp4: *mut Vec<u8>,
) -> JsonVec<orfail::Result<()>> {
    let transcoder = unsafe { &mut *transcoder };
    let input_mp4 = unsafe { Box::from_raw(input_mp4) };
    let result = transcoder.parse_input_mp4_file(&input_mp4).or_fail();
    JsonVec::new(result)
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn startTranscode(transcoder: *mut Transcoder) -> JsonVec<orfail::Result<()>> {
    let transcoder = unsafe { &mut *transcoder };
    let result = transcoder.start_transcode().or_fail();
    JsonVec::new(result)
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn pollTranscode(transcoder: *mut Transcoder) -> JsonVec<orfail::Result<TranscodeProgress>> {
    let transcoder = unsafe { &mut *transcoder };
    let result = transcoder.poll_transcode().or_fail();
    JsonVec::new(result)
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn buildOutputMp4File(transcoder: *mut Transcoder) -> JsonVec<orfail::Result<()>> {
    let transcoder = unsafe { &mut *transcoder };
    let result = transcoder.build_output_mp4_file().or_fail();
    JsonVec::new(result)
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn getOutputMp4File(transcoder: *mut Transcoder) -> *const Vec<u8> {
    let transcoder = unsafe { &mut *transcoder };
    transcoder.get_output_mp4_file() as *const _
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn vecOffset(v: *mut Vec<u8>) -> *mut u8 {
    unsafe { &mut *v }.as_mut_ptr()
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn vecLen(v: *mut Vec<u8>) -> i32 {
    unsafe { &*v }.len() as i32
}

#[no_mangle]
#[expect(non_snake_case)]
pub fn allocateVec(len: i32) -> *mut Vec<u8> {
    Box::into_raw(Box::new(vec![0; len as usize]))
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn freeVec(v: *mut Vec<u8>) {
    let _ = unsafe { Box::from_raw(v) };
}

#[repr(transparent)]
pub struct JsonVec<T> {
    bytes: *mut Vec<u8>,
    _ty: PhantomData<T>,
}

impl<T: Serialize> JsonVec<T> {
    fn new(value: T) -> Self {
        let bytes = Box::into_raw(Box::new(serde_json::to_vec(&value).expect("unreachable")));
        Self {
            bytes,
            _ty: PhantomData,
        }
    }
}

impl<T: for<'de> Deserialize<'de>> JsonVec<T> {
    unsafe fn into_value(self) -> T {
        let bytes = Box::from_raw(self.bytes);
        let value: T = serde_json::from_slice(&bytes).expect("Invalid JSON");
        value
    }
}
