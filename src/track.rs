use std::convert::TryFrom;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use crate::atoms::trak::TrakBox;
use crate::atoms::*;
use crate::*;

#[derive(Debug)]
pub struct Mp4Track {
    trak: TrakBox,
}

impl Mp4Track {
    pub(crate) fn from(trak: &TrakBox) -> Self {
        let trak = trak.clone();
        Self { trak }
    }

    pub fn track_id(&self) -> u32 {
        self.trak.tkhd.track_id
    }

    pub fn track_type(&self) -> Result<TrackType> {
        TrackType::try_from(&self.trak.mdia.hdlr.handler_type)
    }

    pub fn media_type(&self) -> Result<MediaType> {
        if self.trak.mdia.minf.stbl.stsd.avc1.is_some() {
            Ok(MediaType::H264)
        } else if self.trak.mdia.minf.stbl.stsd.mp4a.is_some() {
            Ok(MediaType::AAC)
        } else {
            Err(Error::InvalidData("unsupported media type"))
        }
    }

    pub fn box_type(&self) -> Result<FourCC> {
        if self.trak.mdia.minf.stbl.stsd.avc1.is_some() {
            Ok(FourCC::from(BoxType::Avc1Box))
        } else if self.trak.mdia.minf.stbl.stsd.mp4a.is_some() {
            Ok(FourCC::from(BoxType::Mp4aBox))
        } else {
            Err(Error::InvalidData("unsupported sample entry box"))
        }
    }

    pub fn width(&self) -> u16 {
        if let Some(ref avc1) = self.trak.mdia.minf.stbl.stsd.avc1 {
            avc1.width
        } else {
            self.trak.tkhd.width.to_integer() as u16
        }
    }

    pub fn height(&self) -> u16 {
        if let Some(ref avc1) = self.trak.mdia.minf.stbl.stsd.avc1 {
            avc1.height
        } else {
            self.trak.tkhd.height.to_integer() as u16
        }
    }

    pub fn frame_rate(&self) -> f64 {
        let dur_sec_f64 = self.duration().as_secs_f64();
        if dur_sec_f64 > 0.0 {
            self.sample_count() as f64 / dur_sec_f64
        } else {
            0.0
        }
    }

    pub fn sample_rate(&self) -> u32 {
        if let Some(ref mp4a) = self.trak.mdia.minf.stbl.stsd.mp4a {
            mp4a.samplerate.to_integer() as u32
        } else {
            0 // XXX
        }
    }

    pub fn channel_count(&self) -> u16 {
        if let Some(ref mp4a) = self.trak.mdia.minf.stbl.stsd.mp4a {
            mp4a.channelcount
        } else {
            0 // XXX
        }
    }

    pub fn language(&self) -> &str {
        &self.trak.mdia.mdhd.language
    }

    pub fn timescale(&self) -> u32 {
        self.trak.mdia.mdhd.timescale
    }

    pub fn duration(&self) -> Duration {
        Duration::from_micros(
            self.trak.mdia.mdhd.duration * 1_000_000 / self.trak.mdia.mdhd.timescale as u64,
        )
    }

    pub fn bitrate(&self) -> u32 {
        let dur_sec = self.duration().as_secs();
        if dur_sec > 0 {
            let bitrate = self.total_sample_size() * 8 / dur_sec;
            bitrate as u32
        } else {
            0
        }
    }

    pub fn sample_count(&self) -> u32 {
        self.trak.mdia.minf.stbl.stsz.sample_sizes.len() as u32
    }

    fn stsc_index(&self, sample_id: u32) -> usize {
        for (i, entry) in self.trak.mdia.minf.stbl.stsc.entries.iter().enumerate() {
            if sample_id < entry.first_sample {
                assert_ne!(i, 0);
                return i - 1;
            }
        }

        assert_ne!(self.trak.mdia.minf.stbl.stsc.entries.len(), 0);
        self.trak.mdia.minf.stbl.stsc.entries.len() - 1
    }

    fn chunk_offset(&self, chunk_id: u32) -> Result<u64> {
        if let Some(ref stco) = self.trak.mdia.minf.stbl.stco {
            if let Some(offset) = stco.entries.get(chunk_id as usize - 1) {
                return Ok(*offset as u64);
            } else {
                return Err(Error::EntryInStblNotFound(
                    self.track_id(),
                    BoxType::StcoBox,
                    chunk_id,
                ));
            }
        } else {
            if let Some(ref co64) = self.trak.mdia.minf.stbl.co64 {
                if let Some(offset) = co64.entries.get(chunk_id as usize - 1) {
                    return Ok(*offset);
                } else {
                    return Err(Error::EntryInStblNotFound(
                        self.track_id(),
                        BoxType::Co64Box,
                        chunk_id,
                    ));
                }
            }
        }

        assert!(self.trak.mdia.minf.stbl.stco.is_some() || self.trak.mdia.minf.stbl.co64.is_some());
        return Err(Error::Box2NotFound(BoxType::StcoBox, BoxType::Co64Box));
    }

    fn ctts_index(&self, sample_id: u32) -> Result<(usize, u32)> {
        assert!(self.trak.mdia.minf.stbl.ctts.is_some());
        let ctts = if let Some(ref ctts) = self.trak.mdia.minf.stbl.ctts {
            ctts
        } else {
            return Err(Error::BoxInStblNotFound(self.track_id(), BoxType::CttsBox));
        };

        let mut sample_count = 1;
        for (i, entry) in ctts.entries.iter().enumerate() {
            if sample_id <= sample_count + entry.sample_count - 1 {
                return Ok((i, sample_count));
            }
            sample_count += entry.sample_count;
        }

        return Err(Error::EntryInStblNotFound(
            self.track_id(),
            BoxType::CttsBox,
            sample_id,
        ));
    }

    fn sample_size(&self, sample_id: u32) -> Result<u32> {
        let stsz = &self.trak.mdia.minf.stbl.stsz;
        if stsz.sample_size > 0 {
            return Ok(stsz.sample_size);
        }
        if let Some(size) = stsz.sample_sizes.get(sample_id as usize - 1) {
            Ok(*size)
        } else {
            return Err(Error::EntryInStblNotFound(
                self.track_id(),
                BoxType::StszBox,
                sample_id,
            ));
        }
    }

    fn total_sample_size(&self) -> u64 {
        let stsz = &self.trak.mdia.minf.stbl.stsz;
        if stsz.sample_size > 0 {
            stsz.sample_size as u64 * self.sample_count() as u64
        } else {
            let mut total_size = 0;
            for size in stsz.sample_sizes.iter() {
                total_size += *size as u64;
            }
            total_size
        }
    }

    fn sample_offset(&self, sample_id: u32) -> Result<u64> {
        let stsc_index = self.stsc_index(sample_id);

        let stsc = &self.trak.mdia.minf.stbl.stsc;
        let stsc_entry = stsc.entries.get(stsc_index).unwrap();

        let first_chunk = stsc_entry.first_chunk;
        let first_sample = stsc_entry.first_sample;
        let samples_per_chunk = stsc_entry.samples_per_chunk;

        let chunk_id = first_chunk + (sample_id - first_sample) / samples_per_chunk;

        let chunk_offset = self.chunk_offset(chunk_id)?;

        let first_sample_in_chunk = sample_id - (sample_id - first_sample) % samples_per_chunk;

        let mut sample_offset = 0;
        for i in first_sample_in_chunk..sample_id {
            sample_offset += self.sample_size(i)?;
        }

        Ok(chunk_offset + sample_offset as u64)
    }

    fn sample_time(&self, sample_id: u32) -> Result<(u64, u32)> {
        let stts = &self.trak.mdia.minf.stbl.stts;

        let mut sample_count = 1;
        let mut elapsed = 0;

        for entry in stts.entries.iter() {
            if sample_id <= sample_count + entry.sample_count - 1 {
                let start_time =
                    (sample_id - sample_count) as u64 * entry.sample_delta as u64 + elapsed;
                return Ok((start_time, entry.sample_delta));
            }

            sample_count += entry.sample_count;
            elapsed += entry.sample_count as u64 * entry.sample_delta as u64;
        }

        return Err(Error::EntryInStblNotFound(
            self.track_id(),
            BoxType::SttsBox,
            sample_id,
        ));
    }

    fn sample_rendering_offset(&self, sample_id: u32) -> i32 {
        if let Some(ref ctts) = self.trak.mdia.minf.stbl.ctts {
            if let Ok((ctts_index, _)) = self.ctts_index(sample_id) {
                let ctts_entry = ctts.entries.get(ctts_index).unwrap();
                return ctts_entry.sample_offset;
            }
        }
        0
    }

    fn is_sync_sample(&self, sample_id: u32) -> bool {
        if let Some(ref stss) = self.trak.mdia.minf.stbl.stss {
            match stss.entries.binary_search(&sample_id) {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            true
        }
    }

    pub(crate) fn read_sample<R: Read + Seek>(
        &self,
        reader: &mut R,
        sample_id: u32,
    ) -> Result<Option<Mp4Sample>> {
        let sample_size = match self.sample_size(sample_id) {
            Ok(size) => size,
            Err(Error::EntryInStblNotFound(_, _, _)) => return Ok(None),
            Err(err) => return Err(err),
        };
        let sample_offset = self.sample_offset(sample_id).unwrap(); // XXX

        let mut buffer = vec![0x0u8; sample_size as usize];
        reader.seek(SeekFrom::Start(sample_offset))?;
        reader.read_exact(&mut buffer)?;

        let (start_time, duration) = self.sample_time(sample_id).unwrap(); // XXX
        let rendering_offset = self.sample_rendering_offset(sample_id);
        let is_sync = self.is_sync_sample(sample_id);

        Ok(Some(Mp4Sample {
            start_time,
            duration,
            rendering_offset,
            is_sync,
            bytes: Bytes::from(buffer),
        }))
    }
}
