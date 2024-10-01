use futures::{executor::LocalPool, stream::FusedStream, task::LocalSpawnExt};
use orfail::OrFail;
use serde::{Deserialize, Serialize};

use crate::input_mp4::{InputMp4, Track};

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
pub struct Transcoder {
    options: TranscodeOptions,
    input_mp4: Option<InputMp4>,
    executor: LocalPool,
    executing: bool,
    transcode_result_rx: futures::channel::mpsc::UnboundedReceiver<orfail::Result<Track>>,
    output_tracks: Vec<Track>,
}

impl Transcoder {
    pub fn new(options: TranscodeOptions) -> Self {
        let (_transcode_result_tx, transcode_result_rx) = futures::channel::mpsc::unbounded(); // dummy
        Self {
            options,
            input_mp4: None,
            executor: LocalPool::new(),
            executing: false,
            transcode_result_rx,
            output_tracks: Vec::new(),
        }
    }

    pub fn parse_input_mp4_file(&mut self, mp4: &[u8]) -> orfail::Result<()> {
        self.input_mp4 = Some(InputMp4::parse(mp4).or_fail()?);
        Ok(())
    }

    pub fn start_transcode(&mut self) -> orfail::Result<()> {
        let input_mp4 = self.input_mp4.take().or_fail()?;
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
        todo!()
    }

    pub fn take_output_mp4_file(&mut self) -> Vec<u8> {
        todo!()
    }
}

#[derive(Debug)]
struct TrackTranscoder {
    track: Track,
}

impl TrackTranscoder {
    async fn run(self) -> orfail::Result<Track> {
        let _ = self.track;
        todo!()
    }
}
