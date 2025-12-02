//! MP4 ファイルをデマルチプレックスして、メディアトラックとサンプル情報を表示する例
//!
//! このプログラムは、MP4 ファイルをデマルチプレックスして、含まれるメディアトラックとサンプルの情報を表示する
//!
//! # 実行方法
//!
//! ```bash
//! cargo run --example demux -- /path/to/MP4_FILE
//! ```
use std::env;
use std::fs;
use std::io;
use std::path::Path;

use shiguredo_mp4::TrackKind;
use shiguredo_mp4::boxes::SampleEntry;
use shiguredo_mp4::demux::{Input, Mp4FileDemuxer};

/// トラック種別を文字列に変換
fn get_track_kind_name(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Audio => "Audio",
        TrackKind::Video => "Video",
    }
}

/// サンプルエントリー種別を文字列に変換
fn get_sample_entry_kind_name(sample_entry: &shiguredo_mp4::boxes::SampleEntry) -> String {
    use shiguredo_mp4::boxes::SampleEntry;

    match sample_entry {
        SampleEntry::Avc1(_) => "AVC1 (H.264)".to_string(),
        SampleEntry::Hev1(_) => "HEV1 (H.265/HEVC)".to_string(),
        SampleEntry::Vp08(_) => "VP08 (VP8)".to_string(),
        SampleEntry::Vp09(_) => "VP09 (VP9)".to_string(),
        SampleEntry::Av01(_) => "AV01 (AV1)".to_string(),
        SampleEntry::Opus(_) => "Opus".to_string(),
        SampleEntry::Mp4a(_) => "MP4A (AAC)".to_string(),
        SampleEntry::Unknown(_) => "Unknown".to_string(),
    }
}

/// サンプルエントリー情報を表示
fn print_sample_entry_info(sample_entry: &shiguredo_mp4::boxes::SampleEntry) {
    println!("    Codec: {}", get_sample_entry_kind_name(sample_entry));

    match sample_entry {
        SampleEntry::Avc1(avc1) => {
            println!(
                "    Resolution: {}x{}",
                avc1.visual.width, avc1.visual.height
            );
            let avcc = &avc1.avcc_box;
            println!(
                "    Profile: {}, Level: {}",
                avcc.avc_profile_indication, avcc.avc_level_indication
            );
            println!(
                "    SPS count: {}, PPS count: {}",
                avcc.sps_list.len(),
                avcc.pps_list.len()
            );
        }
        SampleEntry::Hev1(hev1) => {
            println!(
                "    Resolution: {}x{}",
                hev1.visual.width, hev1.visual.height
            );
            let hvcc = &hev1.hvcc_box;
            println!(
                "    Profile: {}, Level: {}",
                hvcc.general_profile_idc.get(),
                hvcc.general_level_idc
            );
            println!(
                "    Chroma format: {}, Bit depth (luma): {}",
                hvcc.chroma_format_idc.get(),
                hvcc.bit_depth_luma_minus8.get() + 8
            );
        }
        SampleEntry::Vp08(vp08) => {
            println!(
                "    Resolution: {}x{}",
                vp08.visual.width, vp08.visual.height
            );
            let vpcc = &vp08.vpcc_box;
            println!("    Bit depth: {}", vpcc.bit_depth.get());
        }
        SampleEntry::Vp09(vp09) => {
            println!(
                "    Resolution: {}x{}",
                vp09.visual.width, vp09.visual.height
            );
            let vpcc = &vp09.vpcc_box;
            println!(
                "    Profile: {}, Level: {}, Bit depth: {}",
                vpcc.profile,
                vpcc.level,
                vpcc.bit_depth.get()
            );
        }
        SampleEntry::Av01(av01) => {
            println!(
                "    Resolution: {}x{}",
                av01.visual.width, av01.visual.height
            );
            let av1c = &av01.av1c_box;
            println!(
                "    Profile: {}, Level: {}, Bit depth: {}",
                av1c.seq_profile.get(),
                av1c.seq_level_idx_0.get(),
                if av1c.high_bitdepth.get() == 1 {
                    "10"
                } else {
                    "8"
                }
            );
        }
        SampleEntry::Opus(opus) => {
            println!(
                "    Channels: {}, Sample rate: {} Hz",
                opus.audio.channelcount, opus.audio.samplerate.integer
            );
        }
        SampleEntry::Mp4a(mp4a) => {
            println!(
                "    Channels: {}, Sample rate: {} Hz",
                mp4a.audio.channelcount, mp4a.audio.samplerate.integer
            );
        }
        SampleEntry::Unknown(_) => {
            println!("    (Unknown codec - no details available)");
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4_file>", args[0]);
        std::process::exit(1);
    }

    let filepath = &args[1];
    if !Path::new(filepath).exists() {
        eprintln!("Error: Could not open file '{}'", filepath);
        std::process::exit(1);
    }

    // MP4 ファイル全体をメモリに読み込む
    let file_data = fs::read(filepath)?;

    // デマルチプレックス処理を初期化し、ファイルデータ全体を提供する
    let mut demuxer = Mp4FileDemuxer::new();
    let input = Input {
        position: 0,
        data: &file_data,
    };
    demuxer.handle_input(input);

    // トラック情報を取得する
    let tracks = match demuxer.tracks() {
        Ok(tracks) => tracks,
        Err(e) => {
            eprintln!("Error: Failed to get tracks: {}", e);
            std::process::exit(1);
        }
    };

    println!("Found {} track(s)\n", tracks.len());

    // トラック情報を表示
    for (i, track) in tracks.iter().enumerate() {
        println!("Track {}:", i + 1);
        println!("  Track ID: {}", track.track_id);
        println!("  Kind: {}", get_track_kind_name(track.kind));
        println!(
            "  Duration: {} (timescale: {})",
            track.duration, track.timescale
        );
        println!();
    }

    // サンプル情報を表示
    let mut sample_count = 0;
    let mut keyframe_count = 0;

    println!("Samples:");

    // 時系列順にサンプルを抽出する
    loop {
        match demuxer.next_sample() {
            Ok(Some(sample)) => {
                sample_count += 1;

                println!("  Sample {}:", sample_count);
                println!("    Track ID: {}", sample.track.track_id);
                println!(
                    "    Keyframe: {}",
                    if sample.keyframe { "Yes" } else { "No" }
                );
                println!("    Timestamp: {}", sample.timestamp);
                println!("    Duration: {}", sample.duration);
                println!("    Data offset: 0x{:x}", sample.data_offset);
                println!("    Data size: {} bytes", sample.data_size);

                // 最初のサンプルのエントリ情報を表示
                if sample_count == 1 {
                    print_sample_entry_info(sample.sample_entry.unwrap());
                }

                if sample.keyframe {
                    keyframe_count += 1;
                }

                println!();

                // 最初の10個のサンプルのみ表示
                if sample_count >= 10 {
                    println!("  ... (showing first 10 samples)");
                    break;
                }
            }
            Ok(None) => {
                // すべてのサンプルを取得し終えた
                break;
            }
            Err(e) => {
                eprintln!("Error: Failed to get next sample: {}", e);
                std::process::exit(1);
            }
        }
    }

    println!(
        "Total: {} samples, {} keyframes",
        sample_count, keyframe_count
    );

    Ok(())
}
