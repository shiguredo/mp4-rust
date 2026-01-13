//! c_api::boxes の JSON シリアライズ機能を提供する wasm 専用モジュール

use c_api::boxes::{Mp4SampleEntry, Mp4SampleEntryKind};

pub fn fmt_json_mp4_sample_entry(
    f: &mut nojson::JsonFormatter<'_, '_>,
    sample_entry: &Mp4SampleEntry,
) -> std::fmt::Result {
    match sample_entry.kind {
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AVC1 => {
            let data = unsafe { &sample_entry.data.avc1 };
            crate::boxes_avc1::fmt_json_mp4_sample_entry_avc1(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_HEV1 => {
            let data = unsafe { &sample_entry.data.hev1 };
            crate::boxes_hev1::fmt_json_mp4_sample_entry_hev1(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_HVC1 => {
            let data = unsafe { &sample_entry.data.hvc1 };
            crate::boxes_hvc1::fmt_json_mp4_sample_entry_hvc1(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP08 => {
            let data = unsafe { &sample_entry.data.vp08 };
            crate::boxes_vp08::fmt_json_mp4_sample_entry_vp08(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_VP09 => {
            let data = unsafe { &sample_entry.data.vp09 };
            crate::boxes_vp09::fmt_json_mp4_sample_entry_vp09(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_AV01 => {
            let data = unsafe { &sample_entry.data.av01 };
            crate::boxes_av01::fmt_json_mp4_sample_entry_av01(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_OPUS => {
            let data = unsafe { &sample_entry.data.opus };
            crate::boxes_opus::fmt_json_mp4_sample_entry_opus(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_MP4A => {
            let data = unsafe { &sample_entry.data.mp4a };
            crate::boxes_mp4a::fmt_json_mp4_sample_entry_mp4a(f, data)?;
        }
        Mp4SampleEntryKind::MP4_SAMPLE_ENTRY_KIND_FLAC => {
            let data = unsafe { &sample_entry.data.flac };
            crate::boxes_flac::fmt_json_mp4_sample_entry_flac(f, data)?;
        }
    }
    Ok(())
}

/// バイト配列を mp4_alloc で確保してコピーするユーティリティ関数
pub fn allocate_and_copy_bytes(data: &[u8]) -> (*const u8, u32) {
    if data.is_empty() {
        return (std::ptr::null(), 0);
    }

    let size = data.len() as u32;
    let ptr = unsafe {
        let allocated = crate::mp4_alloc(size);
        if allocated.is_null() {
            return (std::ptr::null(), 0);
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), allocated, data.len());
        allocated as *const u8
    };
    (ptr, size)
}

/// 複数のバイト列をメモリに割り当ててコピーする
///
/// JSON から複数の配列（SPS/PPS や NALU リストなど）を割り当てる際に使用する
pub fn allocate_and_copy_array_list(arrays: &[Vec<u8>]) -> (*const *const u8, *const u32, u32) {
    let count = arrays.len() as u32;

    if count == 0 {
        return (std::ptr::null(), std::ptr::null(), 0);
    }

    // データポインタ配列を割り当て
    let data_ptrs: Vec<*const u8> = arrays
        .iter()
        .map(|array| allocate_and_copy_bytes(array).0)
        .collect();
    let data_ptr = allocate_and_copy_bytes(unsafe {
        std::slice::from_raw_parts(
            data_ptrs.as_ptr() as *const u8,
            data_ptrs.len() * std::mem::size_of::<*const u8>(),
        )
    })
    .0 as *const *const u8;

    // サイズ配列を割り当て
    let sizes: Vec<u32> = arrays.iter().map(|array| array.len() as u32).collect();
    let sizes_ptr = allocate_and_copy_bytes(unsafe {
        std::slice::from_raw_parts(
            sizes.as_ptr() as *const u8,
            sizes.len() * std::mem::size_of::<u32>(),
        )
    })
    .0 as *const u32;

    (data_ptr, sizes_ptr, count)
}

/// `allocate_and_copy_array_list()` で割り当てられたメモリを解放する
pub fn free_array_list(data_ptrs: *mut *mut u8, sizes: *mut u32, count: u32) {
    if count == 0 {
        return;
    }

    // 各バイト列のメモリを解放
    if !data_ptrs.is_null() {
        let ptrs = unsafe { std::slice::from_raw_parts(data_ptrs, count as usize) };
        for ptr in ptrs {
            if !ptr.is_null() {
                unsafe {
                    crate::mp4_free(*ptr as *mut u8, 0);
                }
            }
        }
        // ポインタ配列自体を解放
        unsafe {
            crate::mp4_free(data_ptrs as *mut u8, 0);
        }
    }

    // サイズ配列を解放
    if !sizes.is_null() {
        unsafe {
            crate::mp4_free(sizes as *mut u8, 0);
        }
    }
}
