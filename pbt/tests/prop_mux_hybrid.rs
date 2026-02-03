//! Mp4HybridFileMuxer の PBT
use std::{num::NonZeroU32, time::Duration};

use proptest::prelude::*;
use shiguredo_mp4::{
    boxes::{AudioSampleEntryFields, DopsBox, OpusBox, RootBox, SampleEntry},
    mux::{Mp4HybridFileMuxer, Mp4HybridFileMuxerOptions, Mp4HybridSample},
    Decode, FixedPointNumber, Mp4File, TrackKind,
};

fn create_opus_sample_entry(channel_count: u8) -> SampleEntry {
    SampleEntry::Opus(OpusBox {
        audio: AudioSampleEntryFields {
            data_reference_index: AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
            channelcount: channel_count as u16,
            samplesize: AudioSampleEntryFields::DEFAULT_SAMPLESIZE,
            samplerate: FixedPointNumber::new(48000u16, 0),
        },
        dops_box: DopsBox {
            output_channel_count: channel_count,
            pre_skip: 312,
            input_sample_rate: 48000,
            output_gain: 0,
        },
        unknown_boxes: vec![],
    })
}

fn apply_output(file_data: &mut Vec<u8>, offset: u64, bytes: &[u8], append_pos: &mut usize) {
    let offset = offset as usize;
    if file_data.len() < offset + bytes.len() {
        file_data.resize(offset + bytes.len(), 0);
    }
    file_data[offset..offset + bytes.len()].copy_from_slice(bytes);
    if offset == *append_pos {
        *append_pos += bytes.len();
    }
}

fn append_sample_data(file_data: &mut Vec<u8>, append_pos: &mut usize, data: &[u8]) {
    let offset = *append_pos;
    if file_data.len() < offset + data.len() {
        file_data.resize(offset + data.len(), 0);
    }
    file_data[offset..offset + data.len()].copy_from_slice(data);
    *append_pos += data.len();
}

fn flush_outputs(
    muxer: &mut Mp4HybridFileMuxer,
    file_data: &mut Vec<u8>,
    append_pos: &mut usize,
    pending_samples: &mut Vec<Vec<u8>>,
) {
    let mut saw_moof = false;
    while let Some((offset, bytes)) = muxer.next_output() {
        apply_output(file_data, offset, bytes, append_pos);
        if bytes.len() >= 8 {
            let box_type = &bytes[4..8];
            if box_type == b"moof" {
                saw_moof = true;
            } else if saw_moof && box_type == b"mdat" {
                for data in pending_samples.drain(..) {
                    append_sample_data(file_data, append_pos, &data);
                }
                saw_moof = false;
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_hybrid_audio_only(samples in prop::collection::vec((1u32..10u32, 1usize..256usize), 1..30)) {
        let options = Mp4HybridFileMuxerOptions {
            fragment_duration: Some(Duration::from_millis(10)),
            ..Default::default()
        };
        let mut muxer = Mp4HybridFileMuxer::with_options(options)
            .expect("failed to create hybrid muxer");
        let sample_entry = create_opus_sample_entry(2);
        let timescale = NonZeroU32::MIN.saturating_add(1000 - 1);

        let mut file_data = Vec::new();
        let mut append_pos = 0usize;
        let mut pending_samples: Vec<Vec<u8>> = Vec::new();

        flush_outputs(&mut muxer, &mut file_data, &mut append_pos, &mut pending_samples);

        for (index, (duration, size)) in samples.iter().enumerate() {
            let data = vec![index as u8; *size];
            let sample = Mp4HybridSample {
                track_kind: TrackKind::Audio,
                sample_entry: if index == 0 { Some(sample_entry.clone()) } else { None },
                keyframe: true,
                timescale,
                duration: *duration,
                data_size: *size,
            };
            muxer.append_sample(&sample).expect("failed to append sample");
            flush_outputs(&mut muxer, &mut file_data, &mut append_pos, &mut pending_samples);
            pending_samples.push(data);
        }

        muxer.finalize().expect("failed to finalize hybrid muxer");
        flush_outputs(&mut muxer, &mut file_data, &mut append_pos, &mut pending_samples);

        prop_assert!(pending_samples.is_empty());
        let _: (Mp4File<RootBox>, _) = Mp4File::decode(&file_data).expect("failed to decode mp4");
    }
}
