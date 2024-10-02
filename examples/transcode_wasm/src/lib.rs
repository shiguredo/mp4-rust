use std::{future::Future, marker::PhantomData};

use futures::{channel::oneshot, TryFutureExt};
use orfail::{Failure, OrFail};
use serde::{Deserialize, Serialize};
use shiguredo_mp4::boxes::Avc1Box;
use transcode::{Codec, TranscodeOptions, TranscodeProgress};

pub mod mp4;
pub mod transcode;

#[derive(Serialize)]
pub struct VideoDecoderConfig {
    pub codec: String,
}

extern "C" {
    pub fn consoleLog(msg: *const u8, msg_len: i32);

    #[expect(improper_ctypes)]
    pub fn createVideoDecoder(
        result_future: *mut oneshot::Sender<orfail::Result<CoderId>>,
        config: JsonVec<VideoDecoderConfig>,
    );
}

pub struct WebCodec;

pub type CoderId = u32;
pub type Transcoder = transcode::Transcoder<WebCodec>;

#[expect(unused_variables, unreachable_code)]
impl Codec for WebCodec {
    type Coder = CoderId;

    fn create_h264_decoder(config: &Avc1Box) -> impl Future<Output = orfail::Result<Self::Coder>> {
        let (tx, rx) = oneshot::channel::<orfail::Result<_>>();
        let config = VideoDecoderConfig {
            codec: format!(
                "avc1.{:02x}{:02x}{:02x}",
                config.avcc_box.avc_profile_indication,
                config.avcc_box.profile_compatibility,
                config.avcc_box.avc_level_indication
            ),
        };
        unsafe {
            createVideoDecoder(Box::into_raw(Box::new(tx)), JsonVec::new(config));
        }
        rx.map_ok_or_else(|e| Err(Failure::new(e.to_string())), |r| r.or_fail())
    }

    fn decode_sample(
        decoder: &mut Self::Coder,
        encoded_data: &[u8],
    ) -> impl Future<Output = orfail::Result<Vec<u8>>> {
        futures::future::ok(todo!())
    }
}

#[no_mangle]
#[expect(non_snake_case)]
pub fn newTranscoder(options: JsonVec<TranscodeOptions>) -> *mut Transcoder {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        unsafe {
            consoleLog(msg.as_ptr(), msg.len() as i32);
        }
    }));

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
pub fn notifyCreateVideoDecoderResult(
    transcoder: *mut Transcoder,
    result_future: *mut oneshot::Sender<orfail::Result<CoderId>>,
    result: JsonVec<orfail::Result<CoderId>>,
) {
    let result = unsafe { result.into_value() };
    let tx = unsafe { Box::from_raw(result_future) };
    let _ = tx.send(result);
    let _ = pollTranscode(transcoder);
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
