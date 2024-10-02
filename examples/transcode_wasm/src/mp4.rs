use std::{collections::HashMap, time::Duration};

use orfail::{Failure, OrFail};
use shiguredo_mp4::{
    boxes::{
        Brand, DinfBox, FtypBox, HdlrBox, MdatBox, MdhdBox, MdiaBox, MinfBox, MoovBox, MvhdBox,
        RootBox, SampleEntry, SmhdBox, StblBox, StcoBox, StscBox, StscEntry, StsdBox, StszBox,
        SttsBox, SttsEntry, TkhdBox, TrakBox, VmhdBox,
    },
    BaseBox, Decode, Either, FixedPointNumber, Mp4File, Mp4FileTime,
};

const TIMESCALE: u32 = 1_000_000; // いったんマイクロ秒に決め打ち

// TOOD: rename
#[derive(Debug)]
pub struct InputMp4 {
    pub tracks: Vec<Track>,
    pub chunk_offsets: Vec<u32>,
    pub file_size: u32,
}

impl InputMp4 {
    pub fn new(tracks: Vec<Track>) -> Self {
        Self {
            tracks,
            chunk_offsets: Vec::new(),
            file_size: 0,
        }
    }

    pub fn build(mut self) -> orfail::Result<Mp4File> {
        let ftyp_box = self.build_ftyp_box();
        let mdat_box = self.build_mdat_box();
        let moov_box = self.build_moov_box().or_fail()?;
        Ok(Mp4File {
            ftyp_box,
            boxes: vec![RootBox::Mdat(mdat_box), RootBox::Moov(moov_box)],
        })
    }

    fn build_ftyp_box(&mut self) -> FtypBox {
        let ftyp_box = FtypBox {
            major_brand: Brand::ISOM,
            minor_version: 0,
            compatible_brands: vec![
                Brand::ISOM,
                Brand::ISO2,
                Brand::MP41,
                Brand::AVC1,
                Brand::AV01,
            ],
        };
        self.file_size += ftyp_box.box_size().get() as u32;
        ftyp_box
    }

    fn build_mdat_box(&mut self) -> MdatBox {
        let mut mdat_box = MdatBox {
            is_variable_size: false,
            payload: Vec::new(),
        };
        self.file_size += mdat_box.box_size().get() as u32;

        for track in &self.tracks {
            for chunk in &track.chunks {
                self.chunk_offsets.push(self.file_size);
                for sample in &chunk.samples {
                    mdat_box.payload.extend_from_slice(&sample.data);
                    self.file_size += sample.data.len() as u32;
                }
            }
        }
        mdat_box
    }

    fn build_moov_box(&mut self) -> orfail::Result<MoovBox> {
        let mvhd_box = MvhdBox {
            creation_time: Mp4FileTime::default(), // TODO: 現在時刻を使う
            modification_time: Mp4FileTime::default(),
            timescale: TIMESCALE,
            duration: self
                .tracks
                .iter()
                .map(|t| t.duration().as_micros())
                .max()
                .unwrap_or_default() as u64,
            rate: MvhdBox::DEFAULT_RATE,
            volume: MvhdBox::DEFAULT_VOLUME,
            matrix: MvhdBox::DEFAULT_MATRIX,
            next_track_id: self.tracks.len() as u32 + 1,
        };
        let mut trak_boxes = Vec::new();
        for (i, track) in std::mem::take(&mut self.tracks).iter().enumerate() {
            let trak_box = self.build_trak_box(i as u32 + 1, track).or_fail()?;
            trak_boxes.push(trak_box);
        }
        Ok(MoovBox {
            mvhd_box,
            trak_boxes,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_trak_box(&mut self, track_id: u32, track: &Track) -> orfail::Result<TrakBox> {
        let tkhd_box = TkhdBox {
            flag_track_enabled: true,
            flag_track_in_movie: true,
            flag_track_in_preview: false,
            flag_track_size_is_aspect_ratio: false,
            creation_time: Mp4FileTime::default(), // TODO
            modification_time: Mp4FileTime::default(),
            track_id,
            duration: track.duration().as_micros() as u64,
            layer: TkhdBox::DEFAULT_LAYER,
            alternate_group: TkhdBox::DEFAULT_ALTERNATE_GROUP,
            volume: TkhdBox::DEFAULT_AUDIO_VOLUME,
            matrix: TkhdBox::DEFAULT_MATRIX,
            width: FixedPointNumber::default(), // TODO: ちゃんと値を設定する
            height: FixedPointNumber::default(),
        };
        Ok(TrakBox {
            tkhd_box,
            edts_box: None,
            mdia_box: self.build_mdia_box(track).or_fail()?,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_mdia_box(&mut self, track: &Track) -> orfail::Result<MdiaBox> {
        let mdhd_box = MdhdBox {
            creation_time: Mp4FileTime::default(),
            modification_time: Mp4FileTime::default(),
            timescale: TIMESCALE,
            duration: track.duration().as_micros() as u64,
            language: MdhdBox::LANGUAGE_UNDEFINED,
        };
        let hdlr_box = HdlrBox {
            handler_type: if track.is_audio {
                HdlrBox::HANDLER_TYPE_SOUN
            } else {
                HdlrBox::HANDLER_TYPE_VIDE
            },
            name: vec![0], // TODO: Utf8String::to_vec()
        };
        let minf_box = MinfBox {
            smhd_or_vmhd_box: if track.is_audio {
                Either::A(SmhdBox::default())
            } else {
                Either::B(VmhdBox::default())
            },
            dinf_box: DinfBox::LOCAL_FILE,
            stbl_box: self.build_stbl_box(track).or_fail()?,
            unknown_boxes: Vec::new(),
        };
        Ok(MdiaBox {
            mdhd_box,
            hdlr_box,
            minf_box,
            unknown_boxes: Vec::new(),
        })
    }

    fn build_stbl_box(&mut self, track: &Track) -> orfail::Result<StblBox> {
        let mut uniq_sample_entries = HashMap::new(); // TODO: これはもう不要かも
        let mut stsd_entries = Vec::new();
        for chunk in &track.chunks {
            if uniq_sample_entries.contains_key(&chunk.sample_entry) {
                continue;
            }
            let index = uniq_sample_entries.len() as u32 + 1;
            uniq_sample_entries.insert(chunk.sample_entry.clone(), index);
            stsd_entries.push(chunk.sample_entry.clone());
        }
        let stsd_box = StsdBox {
            entries: stsd_entries,
        };
        let stts_box = SttsBox {
            // TODO: 圧縮する
            entries: track
                .samples()
                .map(|s| SttsEntry {
                    sample_count: 1,
                    sample_delta: s.duration.as_micros() as u32,
                })
                .collect(),
        };
        let stsc_box = StscBox {
            // TODO: 圧縮する
            entries: track
                .chunks
                .iter()
                .enumerate()
                .map(|(i, c)| StscEntry {
                    first_chunk: i as u32 + 1,
                    sample_per_chunk: c.samples.len() as u32,
                    sample_description_index: uniq_sample_entries[&c.sample_entry],
                })
                .collect(),
        };
        let stsz_box = StszBox::Variable {
            entry_sizes: track.samples().map(|s| s.data.len() as u32).collect(),
        };
        let stco_box = StcoBox {
            chunk_offsets: self.chunk_offsets.drain(0..track.chunks.len()).collect(),
        };
        Ok(StblBox {
            stsd_box,
            stts_box,
            stsc_box,
            stsz_box,
            stco_or_co64_box: Either::A(stco_box),
            stss_box: None,
            unknown_boxes: Vec::new(),
        })
    }

    pub fn parse(mp4_file_bytes: &[u8]) -> orfail::Result<Self> {
        let mp4_file = Mp4File::decode(&mut &mp4_file_bytes[..]).or_fail()?;
        let moov_box = mp4_file
            .boxes
            .iter()
            .find_map(|b| {
                if let RootBox::Moov(b) = b {
                    Some(b)
                } else {
                    None
                }
            })
            .or_fail_with(|()| "'moov' box not found".to_owned())?;
        let mut tracks = Vec::new();
        for (track_index, trak_box) in moov_box.trak_boxes.iter().enumerate() {
            let builder = InputTrackBuilder {
                track_index,
                mp4_file_bytes,
                is_audio: trak_box.mdia_box.hdlr_box.handler_type == HdlrBox::HANDLER_TYPE_SOUN,
                timescale: trak_box.mdia_box.mdhd_box.timescale, // TODO: NonZero にする
                stbl_box: &trak_box.mdia_box.minf_box.stbl_box,
            };
            tracks.push(builder.build().or_fail()?);
        }
        Ok(Self {
            tracks,

            // TODO:
            chunk_offsets: Vec::new(),
            file_size: mp4_file_bytes.len() as u32,
        })
    }
}

// TODO: move
#[derive(Debug)]
pub struct Track {
    pub is_audio: bool,
    pub chunks: Vec<Chunk>,
}

impl Track {
    fn duration(&self) -> Duration {
        self.chunks
            .iter()
            .flat_map(|c| c.samples.iter().map(|s| s.duration))
            .sum()
    }

    fn samples(&self) -> impl '_ + Iterator<Item = &Sample> {
        self.chunks.iter().flat_map(|c| c.samples.iter())
    }
}

#[derive(Debug)]
struct InputTrackBuilder<'a> {
    track_index: usize,
    mp4_file_bytes: &'a [u8],
    is_audio: bool,
    timescale: u32,
    stbl_box: &'a StblBox,
}

impl<'a> InputTrackBuilder<'a> {
    fn build(self) -> orfail::Result<Track> {
        let mut chunks = Vec::new();
        let mut chunk_index_end = self.chunk_count();
        let mut sample_index_end = self.sample_count();
        for StscEntry {
            first_chunk, // TODO: NonZero にする
            sample_per_chunk,
            sample_description_index, // TODO: NonZero にする
        } in self.stbl_box.stsc_box.entries.iter().rev()
        {
            let first_chunk_index = (*first_chunk as usize).checked_sub(1).or_fail_with(|()| {
                format!("Invalid chunk index in {}-th track", self.track_index + 1)
            })?;
            let sample_entry_index = (*sample_description_index as usize)
                .checked_sub(1)
                .or_fail_with(|()| {
                    format!(
                        "Invalid sample description index in {}-th track",
                        self.track_index + 1
                    )
                })?;
            for chunk_index in (first_chunk_index..chunk_index_end).rev() {
                let sample_index_start = sample_index_end
                    .checked_sub(*sample_per_chunk as usize)
                    .or_fail_with(|()| {
                    format!(
                        "Inconsistent `stsc` box entries in {}-th track",
                        self.track_index + 1
                    )
                })?;
                chunks.push(
                    self.build_chunk(
                        chunk_index,
                        sample_entry_index,
                        sample_index_start,
                        sample_index_end,
                    )
                    .or_fail()?,
                );
                sample_index_end = sample_index_start;
            }
            chunk_index_end = first_chunk_index;
        }
        chunks.reverse();
        Ok(Track {
            is_audio: self.is_audio,
            chunks,
        })
    }

    fn build_chunk(
        &self,
        chunk_index: usize,
        sample_entry_index: usize,
        sample_index_start: usize,
        sample_index_end: usize,
    ) -> orfail::Result<Chunk> {
        let sample_entry = self
            .stbl_box
            .stsd_box
            .entries
            .get(sample_entry_index)
            .or_fail_with(|()| {
                format!(
                    "Sample entry {} is not found in {}-th track",
                    sample_entry_index + 1,
                    self.track_index + 1
                )
            })?
            .clone();
        let mut samples = Vec::new();
        let mut sample_offset = 0;
        for sample_index in sample_index_start..sample_index_end {
            let sample = self
                .build_sample(sample_index, chunk_index, sample_offset)
                .or_fail()?;
            sample_offset += sample.data.len();
            samples.push(sample);
        }
        Ok(Chunk {
            sample_entry,
            samples,
        })
    }

    fn build_sample(
        &self,
        sample_index: usize,
        chunk_index: usize,
        sample_offset: usize,
    ) -> orfail::Result<Sample> {
        let duration = self.sample_duration(sample_index).or_fail()?;
        let chunk_offset = self.chunk_offset(chunk_index).or_fail()?;
        let sample_data_start = chunk_offset as usize + sample_offset;
        let sample_data_end =
            sample_data_start + self.sample_size(sample_index).or_fail()? as usize;
        (sample_data_end <= self.mp4_file_bytes.len()).or_fail()?;
        let data = self.mp4_file_bytes[sample_data_start..sample_data_end].to_vec();
        Ok(Sample { duration, data })
    }

    // TODO: StblBox に移す
    fn sample_size(&self, i: usize) -> orfail::Result<u32> {
        match &self.stbl_box.stsz_box {
            StszBox::Fixed { sample_size, .. } => Some(sample_size.get()),
            StszBox::Variable { entry_sizes } => entry_sizes.get(i).copied(),
        }
        .or_fail_with(|()| {
            format!(
                "Inconsistent 'stsz' box in {}-th track",
                self.track_index + 1
            )
        })
    }

    fn sample_duration(&self, sample_index: usize) -> orfail::Result<Duration> {
        // TODO: 最適化
        let mut i = sample_index;
        for entry in &self.stbl_box.stts_box.entries {
            i = i.saturating_sub(entry.sample_count as usize);
            if i == 0 {
                let duration = Duration::from_secs(entry.sample_delta as u64) / self.timescale;
                return Ok(duration);
            }
        }
        Err(Failure::new(format!(
            "Inconsistent 'stts' box in {}-th track: sample_index={sample_index}",
            self.track_index + 1
        )))
    }

    fn chunk_offset(&self, i: usize) -> orfail::Result<u64> {
        match &self.stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets.get(i).copied().map(|v| v as u64),
            Either::B(b) => b.chunk_offsets.get(i).copied(),
        }
        .or_fail_with(|()| {
            format!(
                "Inconsistent 'stco' or 'co64' box in {}-th track",
                self.track_index + 1
            )
        })
    }

    // TODO: StblBox に追加する
    fn chunk_count(&self) -> usize {
        match &self.stbl_box.stco_or_co64_box {
            Either::A(b) => b.chunk_offsets.len(),
            Either::B(b) => b.chunk_offsets.len(),
        }
    }

    // TODO: StblBox に追加する
    fn sample_count(&self) -> usize {
        self.stbl_box
            .stts_box
            .entries
            .iter()
            .map(|x| x.sample_count as usize)
            .sum()
    }
}

// TODO: move
#[derive(Debug, Clone)]
pub struct Chunk {
    pub sample_entry: SampleEntry,
    pub samples: Vec<Sample>,
}

// TODO: move
#[derive(Debug, Clone)]
pub struct Sample {
    pub duration: Duration,
    pub data: Vec<u8>,
}
