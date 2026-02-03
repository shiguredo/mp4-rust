//! Fragmented MP4 の muxer の PBT。

use std::num::NonZeroU32;

use proptest::prelude::*;
use shiguredo_mp4::{
    BoxHeader, Decode, FixedPointNumber, TrackKind, Uint,
    boxes::{
        AudioSampleEntryFields, Avc1Box, AvccBox, DopsBox, FtypBox, MdatBox, MoofBox, MoovBox,
        OpusBox, SampleEntry, VisualSampleEntryFields,
    },
    mux::{FragmentSample, Mp4FragmentedFileMuxer, TrackConfig},
};

fn create_avc1_sample_entry(width: u16, height: u16) -> SampleEntry {
    SampleEntry::Avc1(Avc1Box {
        visual: VisualSampleEntryFields {
            data_reference_index: VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
            width,
            height,
            horizresolution: VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
            vertresolution: VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
            frame_count: VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
            compressorname: VisualSampleEntryFields::NULL_COMPRESSORNAME,
            depth: VisualSampleEntryFields::DEFAULT_DEPTH,
        },
        avcc_box: AvccBox {
            avc_profile_indication: 66,
            profile_compatibility: 0,
            avc_level_indication: 30,
            length_size_minus_one: Uint::new(3),
            sps_list: vec![],
            pps_list: vec![],
            chroma_format: None,
            bit_depth_luma_minus8: None,
            bit_depth_chroma_minus8: None,
            sps_ext_list: vec![],
        },
        unknown_boxes: vec![],
    })
}

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

fn arb_video_sample_info() -> impl Strategy<Value = (u32, u32, bool)> {
    (1u32..10, 100u32..2000, any::<bool>())
}

fn arb_audio_sample_info() -> impl Strategy<Value = (u32, u32)> {
    (1u32..10, 50u32..1000)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn fmp4_init_and_fragment(
        video_samples in prop::collection::vec(arb_video_sample_info(), 1..8),
        audio_samples in prop::collection::vec(arb_audio_sample_info(), 1..8),
    ) {
        let video_track = TrackConfig {
            track_id: 1,
            kind: TrackKind::Video,
            timescale: NonZeroU32::new(30).expect("timescale must be non-zero"),
            sample_entry: create_avc1_sample_entry(1920, 1080),
        };
        let audio_track = TrackConfig {
            track_id: 2,
            kind: TrackKind::Audio,
            timescale: NonZeroU32::new(48000).expect("timescale must be non-zero"),
            sample_entry: create_opus_sample_entry(2),
        };

        let mut muxer = Mp4FragmentedFileMuxer::new(vec![video_track, audio_track])
            .expect("failed to create fmp4 muxer");

        let init_bytes = muxer.init_segment_bytes();
        let (_ftyp, ftyp_size) = FtypBox::decode(init_bytes).expect("failed to decode ftyp");
        let (moov, _) = MoovBox::decode(&init_bytes[ftyp_size..]).expect("failed to decode moov");
        let mvex = moov.mvex_box.as_ref().expect("mvex must exist");
        prop_assert_eq!(mvex.trex_boxes.len(), 2);

        let mut samples = Vec::new();
        let mut expected_video = Vec::new();
        for (duration, data_size, keyframe) in &video_samples {
            samples.push(FragmentSample {
                track_id: 1,
                duration: *duration,
                data_size: *data_size,
                keyframe: *keyframe,
                composition_time_offset: None,
                sample_flags: None,
            });
            expected_video.push((*duration, *data_size));
        }
        let mut expected_audio = Vec::new();
        for (duration, data_size) in &audio_samples {
            samples.push(FragmentSample {
                track_id: 2,
                duration: *duration,
                data_size: *data_size,
                keyframe: true,
                composition_time_offset: None,
                sample_flags: None,
            });
            expected_audio.push((*duration, *data_size));
        }

        let fragment = muxer.build_fragment(&samples).expect("failed to build fragment");
        let (moof, _) = MoofBox::decode(fragment.moof_bytes()).expect("failed to decode moof");
        prop_assert_eq!(moof.mfhd_box.sequence_number, 1);

        let moof_len = fragment.moof_bytes().len() as u64;
        let mdat_header_len = fragment.mdat_header_bytes().len() as u64;
        let video_data_size: u64 = expected_video
            .iter()
            .map(|(_, size)| *size as u64)
            .sum();

        let traf_video = moof
            .traf_boxes
            .iter()
            .find(|traf| traf.tfhd_box.track_id == 1)
            .expect("video traf must exist");
        let traf_audio = moof
            .traf_boxes
            .iter()
            .find(|traf| traf.tfhd_box.track_id == 2)
            .expect("audio traf must exist");

        let trun_video = traf_video
            .trun_boxes
            .first()
            .expect("video trun must exist");
        let trun_audio = traf_audio
            .trun_boxes
            .first()
            .expect("audio trun must exist");

        prop_assert_eq!(
            trun_video.data_offset,
            Some((moof_len + mdat_header_len) as i32)
        );
        prop_assert_eq!(
            trun_audio.data_offset,
            Some((moof_len + mdat_header_len + video_data_size) as i32)
        );

        let tfdt_video = traf_video.tfdt_box.as_ref().expect("video tfdt must exist");
        let tfdt_audio = traf_audio.tfdt_box.as_ref().expect("audio tfdt must exist");
        prop_assert_eq!(tfdt_video.base_media_decode_time, 0);
        prop_assert_eq!(tfdt_audio.base_media_decode_time, 0);

        prop_assert_eq!(trun_video.samples.len(), expected_video.len());
        for (sample, expected) in trun_video.samples.iter().zip(expected_video.iter()) {
            prop_assert_eq!(sample.duration, Some(expected.0));
            prop_assert_eq!(sample.size, Some(expected.1));
        }
        prop_assert_eq!(trun_audio.samples.len(), expected_audio.len());
        for (sample, expected) in trun_audio.samples.iter().zip(expected_audio.iter()) {
            prop_assert_eq!(sample.duration, Some(expected.0));
            prop_assert_eq!(sample.size, Some(expected.1));
        }

        let (mdat_header, _) = BoxHeader::decode(fragment.mdat_header_bytes())
            .expect("failed to decode mdat header");
        prop_assert_eq!(mdat_header.box_type, MdatBox::TYPE);
        let expected_mdat_size = fragment.media_data_size() + fragment.mdat_header_bytes().len() as u64;
        prop_assert_eq!(mdat_header.box_size.get(), expected_mdat_size);

        let fragment2 = muxer.build_fragment(&samples).expect("failed to build fragment");
        let (moof2, _) = MoofBox::decode(fragment2.moof_bytes()).expect("failed to decode moof");
        prop_assert_eq!(moof2.mfhd_box.sequence_number, 2);

        let video_duration_sum: u64 = expected_video
            .iter()
            .map(|(duration, _)| *duration as u64)
            .sum();
        let audio_duration_sum: u64 = expected_audio
            .iter()
            .map(|(duration, _)| *duration as u64)
            .sum();

        let traf_video2 = moof2
            .traf_boxes
            .iter()
            .find(|traf| traf.tfhd_box.track_id == 1)
            .expect("video traf must exist");
        let traf_audio2 = moof2
            .traf_boxes
            .iter()
            .find(|traf| traf.tfhd_box.track_id == 2)
            .expect("audio traf must exist");

        let tfdt_video2 = traf_video2.tfdt_box.as_ref().expect("video tfdt must exist");
        let tfdt_audio2 = traf_audio2.tfdt_box.as_ref().expect("audio tfdt must exist");
        prop_assert_eq!(tfdt_video2.base_media_decode_time, video_duration_sum);
        prop_assert_eq!(tfdt_audio2.base_media_decode_time, audio_duration_sum);
    }
}
