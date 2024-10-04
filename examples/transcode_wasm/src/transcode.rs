use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures::{
    executor::{LocalPool, LocalSpawner},
    stream::FusedStream,
    task::LocalSpawnExt,
};
use orfail::{Failure, OrFail};
use serde::{Deserialize, Serialize};
use shiguredo_mp4::{
    boxes::{SampleEntry, VisualSampleEntryFields, Vp08Box, VpccBox},
    BaseBox, Encode, Uint,
};

use crate::{
    mp4::{Chunk, InputMp4, Mp4FileSummary, OutputMp4Builder, Track},
    wasm::WebCodec,
};

// B フレームが使われている場合には（おそらく）デコーダーのキューをある程度埋める必要があるので、
// どの程度の個数を一度に詰め込むかを指定するための定数値。
// 理想的には、入力ストリームから適切な値を取得すべきだが、簡単のために十分に大きな固定値を使用している。
const DECODE_QUEQUE_SIZE: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrame {
    pub width: u16,
    pub height: u16,
    #[serde(default)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncoderConfig {
    pub codec: String,
    pub bitrate: u32,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeOptions {
    #[serde(flatten)]
    pub video_encoder_config: VideoEncoderConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeProgress {
    pub done: bool,
    pub rate: f32,
}

#[derive(Debug)]
pub struct Transcoder {
    options: TranscodeOptions,
    input_mp4: Option<InputMp4>,
    output_mp4: Vec<u8>,
    executor: LocalPool,
    executing: bool,
    transcode_result_rx: futures::channel::mpsc::UnboundedReceiver<orfail::Result<Track>>,
    output_tracks: Vec<Track>,
    transcode_target_sample_count: usize,
    transcoded_sample_count: Arc<AtomicUsize>,
    transcode_error: Option<Failure>,
}

impl Transcoder {
    pub fn new(options: TranscodeOptions) -> Self {
        let (_transcode_result_tx, transcode_result_rx) = futures::channel::mpsc::unbounded(); // dummy
        Self {
            options,
            input_mp4: None,
            output_mp4: Vec::new(),
            executor: LocalPool::new(),
            executing: false,
            transcode_result_rx,
            output_tracks: Vec::new(),
            transcode_target_sample_count: 0,
            transcoded_sample_count: Arc::new(AtomicUsize::new(0)),
            transcode_error: None,
        }
    }

    pub fn parse_input_mp4_file(&mut self, mp4: &[u8]) -> orfail::Result<Mp4FileSummary> {
        let mp4 = InputMp4::parse(mp4).or_fail()?;
        let summary = mp4.summary();
        self.input_mp4 = Some(mp4);
        Ok(summary)
    }

    pub fn start_transcode(&mut self) -> orfail::Result<()> {
        let input_mp4 = self.input_mp4.take().or_fail()?;
        for track in &input_mp4.tracks {
            if track.is_audio {
                continue;
            }

            // 入力映像は H.264 のみ
            if let Some(sample_entry) = track.chunks.iter().find_map(|c| {
                if matches!(c.sample_entry, SampleEntry::Avc1(_)) {
                    None
                } else {
                    Some(&c.sample_entry)
                }
            }) {
                return Err(Failure::new(format!(
                    "Only H.264 is supported for input video codec: {}",
                    sample_entry.box_type()
                )));
            }
        }

        let (transcode_result_tx, transcode_result_rx) = futures::channel::mpsc::unbounded(); // dummy
        let _ = self.options;
        for track in input_mp4.tracks {
            self.transcode_target_sample_count += track
                .chunks
                .iter()
                .filter(|c| matches!(c.sample_entry, SampleEntry::Avc1(_)))
                .map(|c| c.samples.len())
                .sum::<usize>();

            let transcoder = TrackTranscoder {
                track,
                spawner: self.executor.spawner(),
                transcoded_sample_count: Arc::clone(&self.transcoded_sample_count),
                options: self.options.clone(),
            };
            let transcode_result_tx = transcode_result_tx.clone();
            self.executor
                .spawner()
                .spawn_local(async move {
                    let result = transcoder.run().await.or_fail();
                    let _ = transcode_result_tx.unbounded_send(result);
                })
                .or_fail()?;
        }
        self.transcode_result_rx = transcode_result_rx;
        Ok(())
    }

    pub fn poll_transcode(&mut self) -> orfail::Result<TranscodeProgress> {
        if let Some(e) = self.transcode_error.clone() {
            return Err(e);
        }

        if !self.executing {
            self.executing = true;
            self.executor.run_until_stalled();
            self.executing = false;

            let mut do_continue = true;
            while do_continue {
                do_continue = false;
                match self.transcode_result_rx.try_next() {
                    Err(_) => {
                        // 全ての変換が終了した
                    }
                    Ok(None) => {
                        // 変換中
                    }
                    Ok(Some(result)) => {
                        // 特定のトラックの変換が完了した or 失敗した
                        let result = result.or_fail();
                        self.transcode_error = result.as_ref().err().cloned();
                        self.output_tracks.push(result?);
                        do_continue = true;
                    }
                }
            }
        }

        Ok(TranscodeProgress {
            done: self.transcode_result_rx.is_terminated(),
            rate: if self.transcode_target_sample_count > 0 {
                self.transcoded_sample_count.load(Ordering::SeqCst) as f32
                    / self.transcode_target_sample_count as f32
            } else {
                1.0
            },
        })
    }

    pub fn build_output_mp4_file(&mut self) -> orfail::Result<()> {
        let builder = OutputMp4Builder::new(std::mem::take(&mut self.output_tracks));
        let mp4 = builder.build().or_fail()?;
        self.output_mp4.clear();
        mp4.encode(&mut self.output_mp4).or_fail()?;
        Ok(())
    }

    pub fn get_output_mp4_file(&mut self) -> &Vec<u8> {
        &self.output_mp4
    }
}

#[derive(Debug)]
struct TrackTranscoder {
    track: Track,
    spawner: LocalSpawner,
    transcoded_sample_count: Arc<AtomicUsize>,
    options: TranscodeOptions,
}

impl TrackTranscoder {
    async fn run(self) -> orfail::Result<Track> {
        let mut output_track = Track {
            is_audio: self.track.is_audio,
            chunks: Vec::new(),
        };

        // サンプルエントリーが同じチャンクはまとめる
        let mut current_chunk = None;
        for chunk in &self.track.chunks {
            let Some(current) = &mut current_chunk else {
                current_chunk = Some(chunk.clone());
                continue;
            };
            if current.sample_entry != chunk.sample_entry {
                output_track
                    .chunks
                    .push(std::mem::replace(current, chunk.clone()));
                continue;
            }
            current.samples.extend(chunk.samples.iter().cloned());
        }
        if let Some(chunk) = current_chunk {
            output_track.chunks.push(chunk);
        }

        for chunk in &mut output_track.chunks {
            if !matches!(chunk.sample_entry, SampleEntry::Avc1(_)) {
                // H.264 以外 (= 音声) は無変換
                continue;
            }
            self.transcode_chunk(chunk).await.or_fail()?;
        }

        Ok(output_track)
    }

    async fn transcode_chunk(&self, chunk: &mut Chunk) -> orfail::Result<()> {
        let SampleEntry::Avc1(sample_entry) = &chunk.sample_entry else {
            unreachable!();
        };

        let decoder = WebCodec::create_h264_decoder(sample_entry)
            .await
            .or_fail()?;
        let encoder = WebCodec::create_encoder(&self.options.video_encoder_config)
            .await
            .or_fail()?;

        let decoder_id = decoder.0;
        let encoder_id = encoder.0;
        let mut futures = VecDeque::new();
        for mut sample in std::mem::take(&mut chunk.samples) {
            futures.push_back(
                self.spawner
                    .spawn_local_with_handle(async move {
                        let decoded = WebCodec::decode(decoder_id, sample.keyframe, &sample.data)
                            .await
                            .or_fail()?;
                        let encoded = WebCodec::encode(encoder_id, sample.keyframe, decoded)
                            .await
                            .or_fail()?;
                        sample.data = encoded;
                        Ok::<_, Failure>(sample)
                    })
                    .or_fail()?,
            );
            if futures.len() > DECODE_QUEQUE_SIZE {
                let sample = futures.pop_front().or_fail()?.await.or_fail()?;
                chunk.samples.push(sample);
                self.transcoded_sample_count.fetch_add(1, Ordering::SeqCst);
            }
        }
        std::mem::drop(decoder); // もうデコードすべきサンプルがないことをデコーダーに伝える

        for future in futures {
            let sample = future.await.or_fail()?;
            chunk.samples.push(sample);
            self.transcoded_sample_count.fetch_add(1, Ordering::SeqCst);
        }

        chunk.sample_entry = SampleEntry::Vp08(Vp08Box {
            visual: VisualSampleEntryFields {
                data_reference_index: VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
                width: self.options.video_encoder_config.width,
                height: self.options.video_encoder_config.height,
                horizresolution: VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
                vertresolution: VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
                frame_count: VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
                compressorname: VisualSampleEntryFields::NULL_COMPRESSORNAME,
                depth: VisualSampleEntryFields::DEFAULT_DEPTH,
            },
            vpcc_box: VpccBox {
                profile: 0,
                level: 0,
                bit_depth: Uint::new(8),
                chroma_subsampling: Uint::new(1),
                video_full_range_flag: Uint::new(0),
                colour_primaries: 1,
                transfer_characteristics: 1,
                matrix_coefficients: 1,
                codec_initialization_data: Vec::new(),
            },
            unknown_boxes: Vec::new(),
        });

        Ok(())
    }
}
