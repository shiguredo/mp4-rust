use std::{future::Future, marker::PhantomData};

use futures::{executor::LocalPool, stream::FusedStream, task::LocalSpawnExt};
use orfail::{Failure, OrFail};
use serde::{Deserialize, Serialize};
use shiguredo_mp4::{
    boxes::{Avc1Box, SampleEntry},
    BaseBox, Encode,
};

use crate::mp4::{InputMp4, Track};

pub trait Codec {
    type Coder;

    fn create_h264_decoder(config: &Avc1Box) -> impl Future<Output = orfail::Result<Self::Coder>>;
    fn decode_sample(
        decoder: &mut Self::Coder,
        encoded_data: &[u8],
    ) -> impl Future<Output = orfail::Result<Vec<u8>>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeOptions {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeProgress {
    pub done: bool,
    pub transcoded_frames: usize,
    pub total_frames: usize,
}

#[derive(Debug)]
pub struct Transcoder<CODEC> {
    options: TranscodeOptions,
    input_mp4: Option<InputMp4>,
    output_mp4: Vec<u8>,
    executor: LocalPool,
    executing: bool,
    transcode_result_rx: futures::channel::mpsc::UnboundedReceiver<orfail::Result<Track>>,
    output_tracks: Vec<Track>,
    _codec: PhantomData<CODEC>,
}

impl<CODEC: Codec> Transcoder<CODEC> {
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
            _codec: PhantomData,
        }
    }

    pub fn parse_input_mp4_file(&mut self, mp4: &[u8]) -> orfail::Result<()> {
        self.input_mp4 = Some(InputMp4::parse(mp4).or_fail()?);
        Ok(())
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
            let transcoder = TrackTranscoder { track };
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
                        self.output_tracks.push(result.or_fail()?);
                        do_continue = true;
                    }
                }
            }
        }

        Ok(TranscodeProgress {
            done: self.transcode_result_rx.is_terminated(),
            transcoded_frames: 0,
            total_frames: 0,
        })
    }

    pub fn build_output_mp4_file(&mut self) -> orfail::Result<()> {
        let builder = InputMp4::new(std::mem::take(&mut self.output_tracks));
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

        // for chunk in &self.track.chunks {
        //     let output_chunk = self.transcode_chunk(chunk).await.or_fail()?;
        //     output_track.chunks.push(output_chunk);
        // }

        Ok(output_track)
    }

    // async fn transcode_chunk(&self, input_chunk: &Chunk) -> orfail::Result<Chunk> {
    //     // TODO: 入出力のチャンク数を一対一にマッピングする必要はない
    //     Ok(input_chunk.clone())
    // }
}
