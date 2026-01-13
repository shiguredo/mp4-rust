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
