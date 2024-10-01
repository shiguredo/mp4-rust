use std::time::Duration;

use orfail::{Failure, OrFail};
use shiguredo_mp4::{
    boxes::{RootBox, SampleEntry, StblBox, StscEntry, StszBox},
    Decode, Either, Mp4File,
};

#[derive(Debug)]
pub struct InputMp4 {
    pub tracks: Vec<InputTrack>,
}

impl InputMp4 {
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
                timescale: trak_box.mdia_box.mdhd_box.timescale, // TODO: NonZero にする
                stbl_box: &trak_box.mdia_box.minf_box.stbl_box,
            };
            tracks.push(builder.build().or_fail()?);
        }
        Ok(Self { tracks })
    }
}

#[derive(Debug)]
pub struct InputTrack {
    pub chunks: Vec<InputChunk>,
}

#[derive(Debug)]
struct InputTrackBuilder<'a> {
    track_index: usize,
    mp4_file_bytes: &'a [u8],
    timescale: u32,
    stbl_box: &'a StblBox,
}

impl<'a> InputTrackBuilder<'a> {
    fn build(self) -> orfail::Result<InputTrack> {
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
        Ok(InputTrack { chunks })
    }

    fn build_chunk(
        &self,
        chunk_index: usize,
        sample_entry_index: usize,
        sample_index_start: usize,
        sample_index_end: usize,
    ) -> orfail::Result<InputChunk> {
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
        Ok(InputChunk {
            sample_entry,
            samples,
        })
    }

    fn build_sample(
        &self,
        sample_index: usize,
        chunk_index: usize,
        sample_offset: usize,
    ) -> orfail::Result<InputSample> {
        let duration = self.sample_duration(sample_index).or_fail()?;
        let chunk_offset = self.chunk_offset(chunk_index).or_fail()?;
        let sample_data_start = chunk_offset as usize + sample_offset as usize;
        let sample_data_end =
            sample_data_start + self.sample_size(sample_index).or_fail()? as usize;
        (sample_data_end <= self.mp4_file_bytes.len()).or_fail()?;
        let data = self.mp4_file_bytes[sample_data_start..sample_data_end].to_vec();
        Ok(InputSample { duration, data })
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

#[derive(Debug, Clone)]
pub struct InputChunk {
    pub sample_entry: SampleEntry,
    pub samples: Vec<InputSample>,
}

#[derive(Debug, Clone)]
pub struct InputSample {
    pub duration: Duration,
    pub data: Vec<u8>,
}
