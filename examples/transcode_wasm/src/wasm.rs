use std::{future::Future, marker::PhantomData};

use futures::{channel::oneshot, TryFutureExt};
use orfail::{Failure, OrFail};
use serde::{Deserialize, Serialize};
use shiguredo_mp4::boxes::Avc1Box;
use shiguredo_mp4::Encode;

use crate::mp4::Mp4FileSummary;
use crate::transcode::{
    TranscodeOptions, TranscodeProgress, Transcoder, VideoEncoderConfig, VideoFrame,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDecoderConfig {
    pub codec: String,
    pub coded_width: u16,
    pub coded_height: u16,
    pub description: Vec<u8>,
}

pub struct Encoded {
    pub description: Option<Vec<u8>>,
    pub data: Vec<u8>,
}

extern "C" {
    pub fn consoleLog(msg: *const u8, msg_len: i32);

    #[expect(improper_ctypes)]
    pub fn createVideoDecoder(
        result_future: *mut oneshot::Sender<orfail::Result<CoderId>>,
        config: JsonVec<VideoDecoderConfig>,
    );

    #[expect(improper_ctypes)]
    pub fn decode(
        result_future: *mut oneshot::Sender<orfail::Result<VideoFrame>>,
        coder_id: CoderId,
        keyframe: bool,
        data_offset: *const u8,
        data_len: u32,
    );

    #[expect(improper_ctypes)]
    pub fn createVideoEncoder(
        result_future: *mut oneshot::Sender<orfail::Result<CoderId>>,
        config: JsonVec<VideoEncoderConfig>,
    );

    #[expect(improper_ctypes)]
    pub fn encode(
        result_future: *mut oneshot::Sender<orfail::Result<Encoded>>,
        coder_id: CoderId,
        keyframe: bool,
        width: u32,
        height: u32,
        data_offset: *const u8,
        data_len: u32,
    );

    pub fn closeCoder(coder_id: CoderId);
}

pub struct WebCodec;

pub type CoderId = u32;

impl WebCodec {
    pub fn create_h264_decoder(config: &Avc1Box) -> impl Future<Output = orfail::Result<Coder>> {
        let (tx, rx) = oneshot::channel::<orfail::Result<_>>();

        let mut description = Vec::new();
        config
            .avcc_box
            .encode(&mut description)
            .expect("unreachable");
        description.drain(..8); // ボックスヘッダ部分を取り除く

        let config = VideoDecoderConfig {
            codec: format!(
                "avc1.{:02x}{:02x}{:02x}",
                config.avcc_box.avc_profile_indication,
                config.avcc_box.profile_compatibility,
                config.avcc_box.avc_level_indication
            ),
            description,
            coded_width: config.visual.width,
            coded_height: config.visual.height,
        };
        unsafe {
            createVideoDecoder(Box::into_raw(Box::new(tx)), JsonVec::new(config));
        }
        rx.map_ok_or_else(
            |e| Err(Failure::new(e.to_string())),
            |r| r.or_fail().map(Coder),
        )
    }

    pub fn decode(
        decoder: CoderId,
        keyframe: bool,
        encoded_data: &[u8],
    ) -> impl Future<Output = orfail::Result<VideoFrame>> {
        let (tx, rx) = oneshot::channel::<orfail::Result<_>>();
        unsafe {
            decode(
                Box::into_raw(Box::new(tx)),
                decoder,
                keyframe,
                encoded_data.as_ptr(),
                encoded_data.len() as u32,
            );
        }
        rx.map_ok_or_else(|e| Err(Failure::new(e.to_string())), |r| r.or_fail())
    }

    pub fn create_encoder(
        config: &VideoEncoderConfig,
    ) -> impl Future<Output = orfail::Result<Coder>> {
        let (tx, rx) = oneshot::channel::<orfail::Result<_>>();
        unsafe {
            createVideoEncoder(Box::into_raw(Box::new(tx)), JsonVec::new(config.clone()));
        }
        rx.map_ok_or_else(
            |e| Err(Failure::new(e.to_string())),
            |r| r.or_fail().map(Coder),
        )
    }

    pub fn encode(
        encoder: CoderId,
        keyframe: bool,
        frame: VideoFrame,
    ) -> impl Future<Output = orfail::Result<Encoded>> {
        let (tx, rx) = oneshot::channel::<orfail::Result<_>>();
        unsafe {
            encode(
                Box::into_raw(Box::new(tx)),
                encoder,
                keyframe,
                frame.width as u32,
                frame.height as u32,
                frame.data.as_ptr(),
                frame.data.len() as u32,
            );
        }
        rx.map_ok_or_else(|e| Err(Failure::new(e.to_string())), |r| r.or_fail())
    }
}

#[derive(Debug)]
pub struct Coder(pub CoderId);

impl Drop for Coder {
    fn drop(&mut self) {
        unsafe {
            closeCoder(self.0);
        }
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
pub fn notifyCreateVideoEncoderResult(
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
pub fn notifyDecodeResult(
    transcoder: *mut Transcoder,
    result_future: *mut oneshot::Sender<orfail::Result<VideoFrame>>,
    result: JsonVec<orfail::Result<VideoFrame>>,
    decoded_data: *mut Vec<u8>,
) {
    let result = unsafe { result.into_value() };
    let tx = unsafe { Box::from_raw(result_future) };
    let _ = tx.send(result.map(|mut frame| {
        frame.data = *unsafe { Box::from_raw(decoded_data) };
        frame
    }));
    let _ = pollTranscode(transcoder);
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn notifyEncodeResult(
    transcoder: *mut Transcoder,
    result_future: *mut oneshot::Sender<orfail::Result<Encoded>>,
    result: JsonVec<orfail::Result<Option<Vec<u8>>>>,
    encoded_data: *mut Vec<u8>,
) {
    let result = unsafe { result.into_value() };
    let tx = unsafe { Box::from_raw(result_future) };
    let _ = tx.send(result.map(|description| Encoded {
        description,
        data: *unsafe { Box::from_raw(encoded_data) },
    }));
    let _ = pollTranscode(transcoder);
}

#[no_mangle]
#[expect(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
pub fn parseInputMp4File(
    transcoder: *mut Transcoder,
    input_mp4: *mut Vec<u8>,
) -> JsonVec<orfail::Result<Mp4FileSummary>> {
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
