//! shiguredo_mp4 の wasm バインディング
//!
//! c-api の機能に加えて、wasm 固有の機能を提供する

#![expect(clippy::missing_safety_doc)]

pub mod boxes;

use std::alloc::Layout;

/// メモリを確保する
///
/// # 引数
///
/// - `size`: 確保するバイト数
///
/// # 戻り値
///
/// 確保したメモリの先頭アドレス
#[unsafe(no_mangle)]
pub extern "C" fn mp4_alloc(size: u32) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { std::alloc::alloc(layout) }
}

/// メモリを解放する
///
/// # 引数
///
/// - `ptr`: 解放するメモリの先頭アドレス
/// - `size`: 解放するバイト数
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_free(ptr: *mut u8, size: u32) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { std::alloc::dealloc(ptr, layout) };
}

/// Vec<u8> のポインタを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_vec_ptr(v: *const Vec<u8>) -> *const u8 {
    if v.is_null() {
        return std::ptr::null();
    }
    unsafe { (*v).as_ptr() }
}

/// Vec<u8> の長さを取得する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_vec_len(v: *const Vec<u8>) -> u32 {
    if v.is_null() {
        return 0;
    }
    unsafe { (*v).len() as u32 }
}

/// Vec<u8> を解放する
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp4_vec_free(v: *mut Vec<u8>) {
    if !v.is_null() {
        let _ = unsafe { Box::from_raw(v) };
    }
}

/// ライブラリのバージョンを取得する
///
/// # 戻り値
///
/// バージョン文字列を含む Vec<u8> へのポインタ
#[unsafe(no_mangle)]
pub extern "C" fn mp4_version() -> *mut Vec<u8> {
    let version = env!("SHIGUREDO_MP4_VERSION").as_bytes().to_vec();
    Box::into_raw(Box::new(version))
}
