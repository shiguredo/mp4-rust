//! MP4 のハイブリッド形式（fMP4 + 最終的な mp4）のマルチプレックス機能
use alloc::{vec, vec::Vec};
use core::{num::NonZeroU32, time::Duration};

use crate::{
    BoxHeader, BoxSize, Either, Encode, Error, FixedPointNumber, Mp4FileTime, SampleFlags,
    TrackKind, Utf8String,
    boxes::{
        Brand, Co64Box, DinfBox, FreeBox, FtypBox, HdlrBox, MdatBox, MdhdBox, MdiaBox, MfhdBox,
        MinfBox, MoofBox, MoovBox, MvexBox, MvhdBox, SampleEntry, SmhdBox, StblBox, StcoBox,
        StscBox, StscEntry, StsdBox, StssBox, StszBox, SttsBox, TfdtBox, TfhdBox, TkhdBox, TrafBox,
        TrakBox, TrexBox, TrunBox, TrunSample, VmhdBox,
    },
};

use super::MuxError;

const DEFAULT_FRAGMENT_DURATION: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
struct SampleMetadata {
    duration: u32,
    keyframe: bool,
    size: u32,
}

#[derive(Debug, Clone)]
struct Chunk {
    offset: u64,
    sample_entry: SampleEntry,
    samples: Vec<SampleMetadata>,
}

#[derive(Debug, Clone)]
struct Output {
    offset: u64,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct FragmentSample {
    track_kind: TrackKind,
    sample_entry: SampleEntry,
    duration: u32,
    keyframe: bool,
    size: u32,
}

#[derive(Debug, Clone)]
struct FragmentRun {
    track_kind: TrackKind,
    payload_offset: u64,
    samples: Vec<FragmentSample>,
}

/// [`Mp4HybridFileMuxer`] 用のオプション
#[derive(Debug, Clone)]
pub struct Mp4HybridFileMuxerOptions {
    /// faststart 形式用に事前に確保する moov ボックスのサイズ（バイト単位）
    pub reserved_moov_box_size: usize,

    /// ファイル作成時刻（構築される MP4 ファイル内のメタデータとして使われる）
    pub creation_timestamp: Duration,

    /// fMP4 のフラグメント尺
    ///
    /// `None` の場合はデフォルト値が使用される
    pub fragment_duration: Option<Duration>,
}

impl Default for Mp4HybridFileMuxerOptions {
    fn default() -> Self {
        Self {
            reserved_moov_box_size: 0,
            creation_timestamp: Duration::ZERO,
            fragment_duration: Some(DEFAULT_FRAGMENT_DURATION),
        }
    }
}

/// ハイブリッド形式用のサンプル情報
#[derive(Debug, Clone)]
pub struct Mp4HybridSample {
    /// サンプルのトラック種別
    pub track_kind: TrackKind,

    /// サンプルの詳細情報（コーデック種別など）
    ///
    /// 最初のサンプルでは必須。以降は省略可能で、
    /// 省略した場合は前のサンプルと同じ sample_entry が使用される
    pub sample_entry: Option<SampleEntry>,

    /// キーフレームかどうか
    pub keyframe: bool,

    /// サンプルのタイムスケール（時間単位）
    pub timescale: NonZeroU32,

    /// サンプルの尺（タイムスケール単位）
    pub duration: u32,

    /// サンプルデータのサイズ（バイト単位）
    pub data_size: usize,
}

/// MP4 ファイルをハイブリッド形式で生成するマルチプレックス処理を行うための構造体
///
/// この構造体は、録画途中は fMP4 形式での追記を行い、
/// `finalize()` 呼び出し時に通常の MP4 形式に変換する。
#[derive(Debug, Clone)]
pub struct Mp4HybridFileMuxer {
    options: Mp4HybridFileMuxerOptions,
    outputs: Vec<Output>,
    next_output_index: usize,
    file_size: u64,
    ftyp_box_end_offset: u64,
    moov_box_end_offset: u64,
    next_sequence_number: u32,
    next_track_id: u32,
    last_moov_box: Option<MoovBox>,
    last_sample_kind: Option<TrackKind>,
    audio_chunks: Vec<Chunk>,
    video_chunks: Vec<Chunk>,
    audio_sample_entries: Vec<SampleEntry>,
    video_sample_entries: Vec<SampleEntry>,
    audio_track_id: Option<u32>,
    video_track_id: Option<u32>,
    audio_track_timescale: Option<NonZeroU32>,
    video_track_timescale: Option<NonZeroU32>,
    audio_decode_time: u64,
    video_decode_time: u64,
    fragment_samples: Vec<FragmentSample>,
    fragment_elapsed_audio: Duration,
    fragment_elapsed_video: Duration,
    fragment_pending_cut: bool,
    fragment_has_audio: bool,
    fragment_has_video: bool,
    fragment_base_decode_time_audio: Option<u64>,
    fragment_base_decode_time_video: Option<u64>,
    fragment_sample_entry_audio: Option<SampleEntry>,
    fragment_sample_entry_video: Option<SampleEntry>,
    fragment_sample_entry_index_audio: Option<u32>,
    fragment_sample_entry_index_video: Option<u32>,
    finalized: bool,
}

impl Mp4HybridFileMuxer {
    /// [`Mp4HybridFileMuxer`] インスタンスを生成する
    pub fn new() -> Result<Self, MuxError> {
        Self::with_options(Mp4HybridFileMuxerOptions::default())
    }

    /// 指定したオプションで [`Mp4HybridFileMuxer`] インスタンスを生成する
    pub fn with_options(options: Mp4HybridFileMuxerOptions) -> Result<Self, MuxError> {
        let mut this = Self {
            options,
            outputs: Vec::new(),
            next_output_index: 0,
            file_size: 0,
            ftyp_box_end_offset: 0,
            moov_box_end_offset: 0,
            next_sequence_number: 1,
            next_track_id: 1,
            last_moov_box: None,
            last_sample_kind: None,
            audio_chunks: Vec::new(),
            video_chunks: Vec::new(),
            audio_sample_entries: Vec::new(),
            video_sample_entries: Vec::new(),
            audio_track_id: None,
            video_track_id: None,
            audio_track_timescale: None,
            video_track_timescale: None,
            audio_decode_time: 0,
            video_decode_time: 0,
            fragment_samples: Vec::new(),
            fragment_elapsed_audio: Duration::ZERO,
            fragment_elapsed_video: Duration::ZERO,
            fragment_pending_cut: false,
            fragment_has_audio: false,
            fragment_has_video: false,
            fragment_base_decode_time_audio: None,
            fragment_base_decode_time_video: None,
            fragment_sample_entry_audio: None,
            fragment_sample_entry_video: None,
            fragment_sample_entry_index_audio: None,
            fragment_sample_entry_index_video: None,
            finalized: false,
        };
        this.build_initial_outputs()?;
        Ok(this)
    }

    /// 書き込むべき出力データを順番に取得する
    pub fn next_output(&mut self) -> Option<(u64, &[u8])> {
        let output = self.outputs.get(self.next_output_index)?;
        self.next_output_index += 1;
        Some((output.offset, output.bytes.as_slice()))
    }

    /// 映像ないし音声サンプルのデータを MP4 ファイルに追記する前に呼び出す
    pub fn append_sample(&mut self, sample: &Mp4HybridSample) -> Result<(), MuxError> {
        if self.finalized {
            return Err(MuxError::AlreadyFinalized);
        }

        self.ensure_track_timescale(sample.track_kind, sample.timescale)?;
        self.ensure_track_id(sample.track_kind);

        if self.should_finalize_before_sample(sample)? {
            self.finalize_fragment()?;
        }

        let sample_entry = self.resolve_sample_entry(sample)?;
        let sample_entry_index = self.ensure_sample_entry_index(sample.track_kind, &sample_entry);
        self.ensure_fragment_sample_entry(sample.track_kind, &sample_entry, sample_entry_index);

        if self.fragment_samples.is_empty() {
            self.fragment_elapsed_audio = Duration::ZERO;
            self.fragment_elapsed_video = Duration::ZERO;
            self.fragment_pending_cut = false;
            self.fragment_has_audio = false;
            self.fragment_has_video = false;
            self.fragment_base_decode_time_audio = None;
            self.fragment_base_decode_time_video = None;
        }

        let size = u32::try_from(sample.data_size).map_err(|_| {
            MuxError::EncodeError(Error::invalid_input("sample data size is too large"))
        })?;

        let fragment_sample = FragmentSample {
            track_kind: sample.track_kind,
            sample_entry,
            duration: sample.duration,
            keyframe: sample.keyframe,
            size,
        };

        self.register_fragment_sample(&fragment_sample);

        self.fragment_samples.push(fragment_sample);
        self.update_fragment_duration(sample.track_kind, sample.duration, sample.timescale);
        self.update_pending_cut();

        Ok(())
    }

    /// すべてのサンプルの追加が完了したことを通知する
    pub fn finalize(&mut self) -> Result<(), MuxError> {
        if self.finalized {
            return Err(MuxError::AlreadyFinalized);
        }

        self.finalize_fragment()?;
        self.finalized = true;

        let mdat_box_size = self.file_size - self.ftyp_box_end_offset;
        let mdat_header = self.build_mdat_header(mdat_box_size)?;
        self.outputs.push(Output {
            offset: self.ftyp_box_end_offset,
            bytes: mdat_header,
        });

        let moov_box = self.build_moov_box(false)?;
        let moov_bytes = moov_box.encode_to_vec()?;
        let moov_offset = self.file_size;
        self.outputs.push(Output {
            offset: moov_offset,
            bytes: moov_bytes.clone(),
        });
        self.file_size += moov_bytes.len() as u64;

        Ok(())
    }

    fn build_initial_outputs(&mut self) -> Result<(), MuxError> {
        if self.options.reserved_moov_box_size > 0 {
            let max_payload = u32::MAX as usize - BoxHeader::MIN_SIZE;
            if self.options.reserved_moov_box_size > max_payload {
                return Err(MuxError::EncodeError(Error::invalid_input(
                    "reserved moov box size is too large",
                )));
            }
        }

        let ftyp_box = FtypBox {
            major_brand: Brand::ISOM,
            minor_version: 0,
            compatible_brands: vec![
                Brand::ISOM,
                Brand::ISO2,
                Brand::MP41,
                Brand::AVC1,
                Brand::AV01,
                Brand::ISO6,
            ],
        };
        let ftyp_bytes = ftyp_box.encode_to_vec()?;
        self.outputs.push(Output {
            offset: 0,
            bytes: ftyp_bytes.clone(),
        });
        self.file_size = ftyp_bytes.len() as u64;
        self.ftyp_box_end_offset = self.file_size;

        if self.options.reserved_moov_box_size > 0 {
            let free_box = FreeBox {
                payload: vec![0; self.options.reserved_moov_box_size],
            };
            let free_bytes = free_box.encode_to_vec()?;
            self.outputs.push(Output {
                offset: self.file_size,
                bytes: free_bytes.clone(),
            });
            self.file_size += free_bytes.len() as u64;
        }

        self.moov_box_end_offset = self.file_size;

        Ok(())
    }

    fn ensure_track_id(&mut self, track_kind: TrackKind) {
        let slot = match track_kind {
            TrackKind::Audio => &mut self.audio_track_id,
            TrackKind::Video => &mut self.video_track_id,
        };
        if slot.is_none() {
            let track_id = self.next_track_id;
            self.next_track_id = self.next_track_id.saturating_add(1);
            *slot = Some(track_id);
        }
    }

    fn ensure_track_timescale(
        &mut self,
        track_kind: TrackKind,
        timescale: NonZeroU32,
    ) -> Result<(), MuxError> {
        let slot = match track_kind {
            TrackKind::Audio => &mut self.audio_track_timescale,
            TrackKind::Video => &mut self.video_track_timescale,
        };
        match slot {
            Some(existing) if *existing != timescale => Err(MuxError::TimescaleMismatch {
                track_kind,
                expected: *existing,
                actual: timescale,
            }),
            Some(_) => Ok(()),
            None => {
                *slot = Some(timescale);
                Ok(())
            }
        }
    }

    fn resolve_sample_entry(&self, sample: &Mp4HybridSample) -> Result<SampleEntry, MuxError> {
        if let Some(entry) = &sample.sample_entry {
            return Ok(entry.clone());
        }

        let fragment_entry = match sample.track_kind {
            TrackKind::Audio => self.fragment_sample_entry_audio.clone(),
            TrackKind::Video => self.fragment_sample_entry_video.clone(),
        };
        if let Some(entry) = fragment_entry {
            return Ok(entry);
        }

        let last_entry = match sample.track_kind {
            TrackKind::Audio => self.audio_chunks.last().map(|c| c.sample_entry.clone()),
            TrackKind::Video => self.video_chunks.last().map(|c| c.sample_entry.clone()),
        };
        last_entry.ok_or(MuxError::MissingSampleEntry {
            track_kind: sample.track_kind,
        })
    }

    fn ensure_sample_entry_index(
        &mut self,
        track_kind: TrackKind,
        sample_entry: &SampleEntry,
    ) -> u32 {
        let entries = match track_kind {
            TrackKind::Audio => &mut self.audio_sample_entries,
            TrackKind::Video => &mut self.video_sample_entries,
        };
        if let Some(index) = entries.iter().position(|entry| entry == sample_entry) {
            return index as u32 + 1;
        }
        entries.push(sample_entry.clone());
        entries.len() as u32
    }

    fn ensure_fragment_sample_entry(
        &mut self,
        track_kind: TrackKind,
        sample_entry: &SampleEntry,
        sample_entry_index: u32,
    ) {
        match track_kind {
            TrackKind::Audio => {
                self.fragment_sample_entry_audio = Some(sample_entry.clone());
                self.fragment_sample_entry_index_audio = Some(sample_entry_index);
            }
            TrackKind::Video => {
                self.fragment_sample_entry_video = Some(sample_entry.clone());
                self.fragment_sample_entry_index_video = Some(sample_entry_index);
            }
        }
    }

    fn should_finalize_before_sample(
        &mut self,
        sample: &Mp4HybridSample,
    ) -> Result<bool, MuxError> {
        if self.fragment_samples.is_empty() {
            return Ok(false);
        }

        if let Some(entry) = &sample.sample_entry {
            let current_entry = match sample.track_kind {
                TrackKind::Audio => &self.fragment_sample_entry_audio,
                TrackKind::Video => &self.fragment_sample_entry_video,
            };
            if current_entry
                .as_ref()
                .is_some_and(|current| current != entry)
            {
                return Ok(true);
            }
        }

        if self.fragment_pending_cut {
            if self.fragment_has_video {
                return Ok(sample.track_kind == TrackKind::Video && sample.keyframe);
            }
            return Ok(true);
        }

        if self.fragment_has_video
            && sample.track_kind == TrackKind::Video
            && sample.keyframe
            && self.fragment_elapsed_with_sample(sample) >= self.fragment_duration()
        {
            return Ok(true);
        }

        Ok(false)
    }

    fn fragment_elapsed_with_sample(&self, sample: &Mp4HybridSample) -> Duration {
        let sample_duration = Self::sample_duration(sample.duration, sample.timescale);
        let mut audio = self.fragment_elapsed_audio;
        let mut video = self.fragment_elapsed_video;
        match sample.track_kind {
            TrackKind::Audio => audio += sample_duration,
            TrackKind::Video => video += sample_duration,
        }
        audio.max(video)
    }

    fn fragment_duration(&self) -> Duration {
        self.options
            .fragment_duration
            .unwrap_or(DEFAULT_FRAGMENT_DURATION)
    }

    fn register_fragment_sample(&mut self, sample: &FragmentSample) {
        match sample.track_kind {
            TrackKind::Audio => {
                if self.fragment_base_decode_time_audio.is_none() {
                    self.fragment_base_decode_time_audio = Some(self.audio_decode_time);
                }
                self.audio_decode_time += sample.duration as u64;
                self.fragment_has_audio = true;
            }
            TrackKind::Video => {
                if self.fragment_base_decode_time_video.is_none() {
                    self.fragment_base_decode_time_video = Some(self.video_decode_time);
                }
                self.video_decode_time += sample.duration as u64;
                self.fragment_has_video = true;
            }
        }
    }

    fn update_fragment_duration(
        &mut self,
        track_kind: TrackKind,
        duration: u32,
        timescale: NonZeroU32,
    ) {
        let delta = Self::sample_duration(duration, timescale);
        match track_kind {
            TrackKind::Audio => self.fragment_elapsed_audio += delta,
            TrackKind::Video => self.fragment_elapsed_video += delta,
        }
    }

    fn update_pending_cut(&mut self) {
        if self.fragment_elapsed() >= self.fragment_duration() {
            self.fragment_pending_cut = true;
        }
    }

    fn fragment_elapsed(&self) -> Duration {
        self.fragment_elapsed_audio.max(self.fragment_elapsed_video)
    }

    fn sample_duration(duration: u32, timescale: NonZeroU32) -> Duration {
        Duration::from_secs(duration as u64) / timescale.get()
    }

    fn finalize_fragment(&mut self) -> Result<(), MuxError> {
        if self.fragment_samples.is_empty() {
            return Ok(());
        }

        let (runs, fragment_payload_size) = self.build_fragment_runs();
        let mdat_header = self.build_mdat_header(fragment_payload_size)?;
        let mdat_header_size = mdat_header.len();

        let moof_bytes = self.build_moof_bytes(&runs, mdat_header_size)?;
        let moof_size = moof_bytes.len();

        let fragment_data_start = self.file_size + moof_size as u64 + mdat_header_size as u64;
        self.record_fragment_samples(fragment_data_start)?;

        self.outputs.push(Output {
            offset: self.file_size,
            bytes: moof_bytes,
        });
        self.outputs.push(Output {
            offset: self.file_size + moof_size as u64,
            bytes: mdat_header,
        });

        self.file_size += moof_size as u64 + mdat_header_size as u64 + fragment_payload_size;

        self.reset_fragment_state();
        self.maybe_update_moov_box()?;

        Ok(())
    }

    fn build_fragment_runs(&self) -> (Vec<FragmentRun>, u64) {
        let mut runs: Vec<FragmentRun> = Vec::new();
        let mut payload_offset = 0u64;

        for sample in &self.fragment_samples {
            let is_new_run = match runs.last() {
                Some(run) => run.track_kind != sample.track_kind,
                None => true,
            };
            if is_new_run {
                runs.push(FragmentRun {
                    track_kind: sample.track_kind,
                    payload_offset,
                    samples: Vec::new(),
                });
            }
            let last_run = runs.last_mut().expect("fragment run should exist");
            last_run.samples.push(sample.clone());
            payload_offset += sample.size as u64;
        }

        (runs, payload_offset)
    }

    fn build_moof_bytes(
        &self,
        runs: &[FragmentRun],
        mdat_header_size: usize,
    ) -> Result<Vec<u8>, MuxError> {
        let moof_box = self.build_moof_box(runs, mdat_header_size, 0)?;
        let moof_size = moof_box.encode_to_vec()?.len();
        let moof_box = self.build_moof_box(runs, mdat_header_size, moof_size)?;
        Ok(moof_box.encode_to_vec()?)
    }

    fn build_moof_box(
        &self,
        runs: &[FragmentRun],
        mdat_header_size: usize,
        moof_size: usize,
    ) -> Result<MoofBox, MuxError> {
        let mut traf_boxes = Vec::new();

        if self.fragment_has_audio {
            let track_id = self
                .audio_track_id
                .expect("audio track id should be assigned");
            let sample_entry_index = self
                .fragment_sample_entry_index_audio
                .expect("audio sample entry index should be set");
            let base_decode_time = self
                .fragment_base_decode_time_audio
                .expect("audio base decode time should be set");
            let trun_boxes =
                self.build_trun_boxes(runs, TrackKind::Audio, mdat_header_size, moof_size)?;
            traf_boxes.push(self.build_traf_box(
                track_id,
                sample_entry_index,
                base_decode_time,
                trun_boxes,
            ));
        }

        if self.fragment_has_video {
            let track_id = self
                .video_track_id
                .expect("video track id should be assigned");
            let sample_entry_index = self
                .fragment_sample_entry_index_video
                .expect("video sample entry index should be set");
            let base_decode_time = self
                .fragment_base_decode_time_video
                .expect("video base decode time should be set");
            let trun_boxes =
                self.build_trun_boxes(runs, TrackKind::Video, mdat_header_size, moof_size)?;
            traf_boxes.push(self.build_traf_box(
                track_id,
                sample_entry_index,
                base_decode_time,
                trun_boxes,
            ));
        }

        Ok(MoofBox {
            mfhd_box: MfhdBox {
                sequence_number: self.next_sequence_number,
            },
            traf_boxes,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_traf_box(
        &self,
        track_id: u32,
        sample_entry_index: u32,
        base_decode_time: u64,
        trun_boxes: Vec<TrunBox>,
    ) -> TrafBox {
        TrafBox {
            tfhd_box: TfhdBox {
                track_id,
                base_data_offset: None,
                sample_description_index: Some(sample_entry_index),
                default_sample_duration: None,
                default_sample_size: None,
                default_sample_flags: None,
                duration_is_empty: false,
                default_base_is_moof: true,
            },
            tfdt_box: Some(TfdtBox {
                version: 0,
                base_media_decode_time: base_decode_time,
            }),
            trun_boxes,
            unknown_boxes: Vec::new(),
        }
    }

    fn build_trun_boxes(
        &self,
        runs: &[FragmentRun],
        track_kind: TrackKind,
        mdat_header_size: usize,
        moof_size: usize,
    ) -> Result<Vec<TrunBox>, MuxError> {
        let mut trun_boxes = Vec::new();
        for run in runs.iter().filter(|run| run.track_kind == track_kind) {
            let data_offset = moof_size
                .saturating_add(mdat_header_size)
                .saturating_add(run.payload_offset as usize);
            let data_offset = i32::try_from(data_offset).map_err(|_| {
                MuxError::EncodeError(Error::invalid_data("trun data_offset is too large"))
            })?;
            let samples = run
                .samples
                .iter()
                .map(|sample| TrunSample {
                    duration: Some(sample.duration),
                    size: Some(sample.size),
                    flags: Some(Self::trun_sample_flags(sample.keyframe)),
                    composition_time_offset: None,
                })
                .collect();
            trun_boxes.push(TrunBox {
                data_offset: Some(data_offset),
                first_sample_flags: None,
                samples,
            });
        }
        Ok(trun_boxes)
    }

    fn trun_sample_flags(keyframe: bool) -> SampleFlags {
        let non_sync = if keyframe { 0 } else { 1 };
        let padding = 7u32 << 17;
        SampleFlags::new(padding | (non_sync << 16))
    }

    fn build_mdat_header(&self, payload_size: u64) -> Result<Vec<u8>, MuxError> {
        let box_size = BoxSize::with_payload_size(MdatBox::TYPE, payload_size);
        let header = BoxHeader::new(MdatBox::TYPE, box_size);
        Ok(header.encode_to_vec()?)
    }

    fn record_fragment_samples(&mut self, fragment_data_start: u64) -> Result<(), MuxError> {
        let mut offset = fragment_data_start;
        let samples = self.fragment_samples.clone();
        for sample in &samples {
            self.record_sample_for_moov(sample, offset)?;
            offset += sample.size as u64;
        }
        Ok(())
    }

    fn record_sample_for_moov(
        &mut self,
        sample: &FragmentSample,
        data_offset: u64,
    ) -> Result<(), MuxError> {
        let is_new_chunk_needed = self.is_new_chunk_needed(sample.track_kind, &sample.sample_entry);
        let chunks = match sample.track_kind {
            TrackKind::Audio => &mut self.audio_chunks,
            TrackKind::Video => &mut self.video_chunks,
        };
        if is_new_chunk_needed {
            chunks.push(Chunk {
                offset: data_offset,
                sample_entry: sample.sample_entry.clone(),
                samples: Vec::new(),
            });
        }

        chunks
            .last_mut()
            .expect("chunk should exist")
            .samples
            .push(SampleMetadata {
                duration: sample.duration,
                keyframe: sample.keyframe,
                size: sample.size,
            });

        self.last_sample_kind = Some(sample.track_kind);

        Ok(())
    }

    fn is_new_chunk_needed(&self, track_kind: TrackKind, sample_entry: &SampleEntry) -> bool {
        if self.last_sample_kind != Some(track_kind) {
            return true;
        }

        let chunks = match track_kind {
            TrackKind::Audio => &self.audio_chunks,
            TrackKind::Video => &self.video_chunks,
        };

        chunks
            .last()
            .is_none_or(|chunk| chunk.sample_entry != *sample_entry)
    }

    fn reset_fragment_state(&mut self) {
        self.fragment_samples.clear();
        self.fragment_elapsed_audio = Duration::ZERO;
        self.fragment_elapsed_video = Duration::ZERO;
        self.fragment_pending_cut = false;
        self.fragment_has_audio = false;
        self.fragment_has_video = false;
        self.fragment_base_decode_time_audio = None;
        self.fragment_base_decode_time_video = None;
        self.fragment_sample_entry_audio = None;
        self.fragment_sample_entry_video = None;
        self.fragment_sample_entry_index_audio = None;
        self.fragment_sample_entry_index_video = None;
        self.next_sequence_number = self.next_sequence_number.saturating_add(1);
    }

    fn maybe_update_moov_box(&mut self) -> Result<(), MuxError> {
        if self.options.reserved_moov_box_size == 0 {
            return Ok(());
        }

        let moov_box = self.build_moov_box(true)?;
        if self
            .last_moov_box
            .as_ref()
            .is_some_and(|last| last == &moov_box)
        {
            return Ok(());
        }

        let moov_bytes = moov_box.encode_to_vec()?;
        if moov_bytes.len() > self.options.reserved_moov_box_size {
            self.last_moov_box = Some(moov_box);
            return Ok(());
        }

        let moov_offset = self.moov_box_end_offset - moov_bytes.len() as u64;
        let free_size = moov_offset - self.ftyp_box_end_offset;
        let free_size = u32::try_from(free_size).map_err(|_| {
            MuxError::EncodeError(Error::invalid_input("free box size is too large"))
        })?;

        self.outputs.push(Output {
            offset: self.ftyp_box_end_offset,
            bytes: free_size.to_be_bytes().to_vec(),
        });
        self.outputs.push(Output {
            offset: moov_offset,
            bytes: moov_bytes,
        });

        self.last_moov_box = Some(moov_box);
        Ok(())
    }

    fn build_moov_box(&self, include_mvex: bool) -> Result<MoovBox, MuxError> {
        let mut trak_boxes = Vec::new();

        if !self.audio_chunks.is_empty() {
            let track_id = self
                .audio_track_id
                .expect("audio track id should be assigned");
            trak_boxes.push(self.build_audio_trak_box(track_id)?);
        }

        if !self.video_chunks.is_empty() {
            let track_id = self
                .video_track_id
                .expect("video track id should be assigned");
            trak_boxes.push(self.build_video_trak_box(track_id)?);
        }

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let (timescale, duration) = self.calculate_total_duration();
        let mvhd_box = MvhdBox {
            creation_time,
            modification_time: creation_time,
            timescale,
            duration,
            rate: MvhdBox::DEFAULT_RATE,
            volume: MvhdBox::DEFAULT_VOLUME,
            matrix: MvhdBox::DEFAULT_MATRIX,
            next_track_id: trak_boxes.len() as u32 + 1,
        };

        let mvex_box = if include_mvex {
            Some(self.build_mvex_box(trak_boxes.len() as u32))
        } else {
            None
        };

        Ok(MoovBox {
            mvhd_box,
            trak_boxes,
            mvex_box,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_mvex_box(&self, track_count: u32) -> MvexBox {
        let default_flags = Self::trun_sample_flags(true);
        let trex_boxes = (1..=track_count)
            .map(|track_id| TrexBox {
                track_id,
                default_sample_description_index: 1,
                default_sample_duration: 0,
                default_sample_size: 0,
                default_sample_flags: default_flags,
            })
            .collect();

        MvexBox {
            mehd_box: None,
            trex_boxes,
            unknown_boxes: Vec::new(),
        }
    }

    fn build_audio_trak_box(&self, track_id: u32) -> Result<TrakBox, MuxError> {
        let total_duration = self
            .audio_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let tkhd_box = TkhdBox {
            flag_track_enabled: true,
            flag_track_in_movie: true,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,
            creation_time,
            modification_time: creation_time,
            track_id,
            duration: total_duration,
            layer: TkhdBox::DEFAULT_LAYER,
            alternate_group: TkhdBox::DEFAULT_ALTERNATE_GROUP,
            volume: TkhdBox::DEFAULT_AUDIO_VOLUME,
            matrix: TkhdBox::DEFAULT_MATRIX,
            width: FixedPointNumber::default(),
            height: FixedPointNumber::default(),
        };

        Ok(TrakBox {
            tkhd_box,
            edts_box: None,
            mdia_box: self.build_audio_mdia_box()?,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_video_trak_box(&self, track_id: u32) -> Result<TrakBox, MuxError> {
        let total_duration = self
            .video_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let (max_width, max_height) = self
            .video_chunks
            .iter()
            .filter_map(|c| c.sample_entry.video_resolution())
            .fold((0u16, 0u16), |(max_w, max_h), (w, h)| {
                (max_w.max(w), max_h.max(h))
            });

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let tkhd_box = TkhdBox {
            flag_track_enabled: true,
            flag_track_in_movie: true,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,
            creation_time,
            modification_time: creation_time,
            track_id,
            duration: total_duration,
            layer: TkhdBox::DEFAULT_LAYER,
            alternate_group: TkhdBox::DEFAULT_ALTERNATE_GROUP,
            volume: TkhdBox::DEFAULT_VIDEO_VOLUME,
            matrix: TkhdBox::DEFAULT_MATRIX,
            width: FixedPointNumber::new(max_width as i16, 0),
            height: FixedPointNumber::new(max_height as i16, 0),
        };

        Ok(TrakBox {
            tkhd_box,
            edts_box: None,
            mdia_box: self.build_video_mdia_box()?,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_audio_mdia_box(&self) -> Result<MdiaBox, MuxError> {
        let total_duration = self
            .audio_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let timescale = self
            .audio_track_timescale
            .expect("audio track timescale should be set");
        let mdhd_box = MdhdBox {
            creation_time,
            modification_time: creation_time,
            timescale,
            duration: total_duration,
            language: MdhdBox::LANGUAGE_UNDEFINED,
        };

        let hdlr_box = HdlrBox {
            handler_type: HdlrBox::HANDLER_TYPE_SOUN,
            name: Utf8String::EMPTY.into_null_terminated_bytes(),
        };

        let minf_box = MinfBox {
            smhd_or_vmhd_box: Some(Either::A(SmhdBox::default())),
            dinf_box: DinfBox::LOCAL_FILE,
            stbl_box: self.build_stbl_box(&self.audio_chunks),
            unknown_boxes: Vec::new(),
        };

        Ok(MdiaBox {
            mdhd_box,
            hdlr_box,
            minf_box,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_video_mdia_box(&self) -> Result<MdiaBox, MuxError> {
        let total_duration = self
            .video_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        let creation_time = Mp4FileTime::from_unix_time(self.options.creation_timestamp);
        let timescale = self
            .video_track_timescale
            .expect("video track timescale should be set");
        let mdhd_box = MdhdBox {
            creation_time,
            modification_time: creation_time,
            timescale,
            duration: total_duration,
            language: MdhdBox::LANGUAGE_UNDEFINED,
        };

        let hdlr_box = HdlrBox {
            handler_type: HdlrBox::HANDLER_TYPE_VIDE,
            name: Utf8String::EMPTY.into_null_terminated_bytes(),
        };

        let minf_box = MinfBox {
            smhd_or_vmhd_box: Some(Either::B(VmhdBox::default())),
            dinf_box: DinfBox::LOCAL_FILE,
            stbl_box: self.build_stbl_box(&self.video_chunks),
            unknown_boxes: Vec::new(),
        };

        Ok(MdiaBox {
            mdhd_box,
            hdlr_box,
            minf_box,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_stbl_box(&self, chunks: &[Chunk]) -> StblBox {
        let mut sample_entries = Vec::new();
        for chunk in chunks {
            if sample_entries.contains(&chunk.sample_entry) {
                continue;
            }
            sample_entries.push(chunk.sample_entry.clone());
        }

        let stsd_box = StsdBox {
            entries: sample_entries.clone(),
        };

        let stts_box = SttsBox::from_sample_deltas(
            chunks
                .iter()
                .flat_map(|c| c.samples.iter().map(|s| s.duration)),
        );

        let stsc_box = StscBox {
            entries: chunks
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let sample_description_index = sample_entries
                        .iter()
                        .position(|entry| entry == &c.sample_entry)
                        .map(|idx| NonZeroU32::MIN.saturating_add(idx as u32))
                        .expect("sample_entry should exist in sample_entries");
                    StscEntry {
                        first_chunk: NonZeroU32::MIN.saturating_add(i as u32),
                        sample_per_chunk: c.samples.len() as u32,
                        sample_description_index,
                    }
                })
                .collect(),
        };

        let stsz_box = StszBox::Variable {
            entry_sizes: chunks
                .iter()
                .flat_map(|c| c.samples.iter().map(|s| s.size))
                .collect(),
        };

        let use_co64 = chunks.iter().any(|c| c.offset > u32::MAX as u64);
        let stco_or_co64_box = if use_co64 {
            Either::B(Co64Box {
                chunk_offsets: chunks.iter().map(|c| c.offset).collect(),
            })
        } else {
            Either::A(StcoBox {
                chunk_offsets: chunks.iter().map(|c| c.offset as u32).collect(),
            })
        };

        let is_all_keyframe = chunks.iter().all(|c| c.samples.iter().all(|s| s.keyframe));
        let stss_box = if is_all_keyframe {
            None
        } else {
            Some(StssBox {
                sample_numbers: chunks
                    .iter()
                    .flat_map(|c| c.samples.iter())
                    .enumerate()
                    .filter_map(|(i, s)| {
                        s.keyframe
                            .then_some(NonZeroU32::MIN.saturating_add(i as u32))
                    })
                    .collect(),
            })
        };

        StblBox {
            stsd_box,
            stts_box,
            stsc_box,
            stsz_box,
            stco_or_co64_box,
            stss_box,
            unknown_boxes: Vec::new(),
        }
    }

    fn calculate_total_duration(&self) -> (NonZeroU32, u64) {
        let audio_duration = self
            .audio_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();
        let video_duration = self
            .video_chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration as u64))
            .sum::<u64>();

        match (self.audio_chunks.is_empty(), self.video_chunks.is_empty()) {
            (false, true) => (
                self.audio_track_timescale
                    .expect("audio track timescale should be set"),
                audio_duration,
            ),
            (true, false) => (
                self.video_track_timescale
                    .expect("video track timescale should be set"),
                video_duration,
            ),
            (true, true) => (NonZeroU32::MIN, 0),
            (false, false) => {
                let audio_timescale = self
                    .audio_track_timescale
                    .expect("audio track timescale should be set");
                let video_timescale = self
                    .video_track_timescale
                    .expect("video track timescale should be set");
                let normalized_audio_duration =
                    Duration::from_secs(audio_duration) / audio_timescale.get();
                let normalized_video_duration =
                    Duration::from_secs(video_duration) / video_timescale.get();

                if normalized_audio_duration < normalized_video_duration {
                    (video_timescale, video_duration)
                } else {
                    (audio_timescale, audio_duration)
                }
            }
        }
    }
}
