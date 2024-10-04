use serde::Serialize;
use shiguredo_mp4::{boxes::RootBox, BaseBox, Decode, Mp4File};

#[derive(Debug, Serialize)]
struct BoxInfo {
    #[serde(rename = "type")]
    pub ty: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Self>,
}

impl BoxInfo {
    fn new(b: &dyn BaseBox) -> Self {
        Self {
            ty: b.box_type().to_string(),
            size: b.box_size().get(),
            unknown: b.is_unknown_box().then_some(true),
            children: b.children().map(Self::new).collect(),
        }
    }
}

#[no_mangle]
#[expect(clippy::not_unsafe_ptr_arg_deref)]
pub fn dump(bytes: *const u8, bytes_len: i32) -> *mut Vec<u8> {
    let bytes = unsafe { std::slice::from_raw_parts(bytes, bytes_len as usize) };

    let json = Mp4File::<RootBox>::decode(&mut &bytes[..])
        .map_err(|e| e.to_string())
        .and_then(|mp4| {
            let infos = mp4.iter().map(BoxInfo::new).collect::<Vec<_>>();
            serde_json::to_string_pretty(&infos).map_err(|e| e.to_string())
        })
        .unwrap_or_else(|e| e);

    Box::into_raw(Box::new(json.into_bytes()))
}

#[no_mangle]
#[expect(clippy::not_unsafe_ptr_arg_deref)]
pub fn vec_offset(v: *mut Vec<u8>) -> *mut u8 {
    unsafe { &mut *v }.as_mut_ptr()
}

#[no_mangle]
#[expect(clippy::not_unsafe_ptr_arg_deref)]
pub fn vec_len(v: *mut Vec<u8>) -> i32 {
    unsafe { &*v }.len() as i32
}

#[no_mangle]
pub fn allocate_vec(len: i32) -> *mut Vec<u8> {
    Box::into_raw(Box::new(vec![0; len as usize]))
}

#[no_mangle]
#[expect(clippy::not_unsafe_ptr_arg_deref)]
pub fn free_vec(v: *mut Vec<u8>) {
    let _ = unsafe { Box::from_raw(v) };
}
