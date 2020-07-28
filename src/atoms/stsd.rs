use std::io::{BufReader, SeekFrom, Seek, Read, BufWriter, Write};

use byteorder::{BigEndian, ReadBytesExt};

use crate::{Result};
use crate::{BoxType, BoxHeader, Mp4Box, ReadBox, WriteBox};
use crate::{HEADER_SIZE};
use crate::{read_box_header, read_box_header_ext, skip_read};


#[derive(Debug)]
pub struct StsdBox {
    pub version: u8,
    pub flags: u32,
    pub entry_count: u32,
}

impl Mp4Box for StsdBox {
    fn box_type(&self) -> BoxType {
        BoxType::StsdBox
    }

    fn box_size(&self) -> u64 {
        // TODO
        0
    }
}

impl<R: Read + Seek> ReadBox<&mut BufReader<R>> for StsdBox {
    fn read_box(reader: &mut BufReader<R>, size: u64) -> Result<Self> {
        let current = reader.seek(SeekFrom::Current(0)).unwrap(); // Current cursor position.

        let (version, flags) = read_box_header_ext(reader).unwrap();

        let entry_count = reader.read_u32::<BigEndian>().unwrap();

        let mut start = 0u64;
        while start < size {
            // Get box header.
            let header = read_box_header(reader, start).unwrap();
            let BoxHeader{ name, size: s } = header;

            match name {
                BoxType::Avc1Box => {}
                BoxType::Mp4aBox => {}
                _ => break
            }
            start += s - HEADER_SIZE;
        }
        skip_read(reader, current, size);

        Ok(StsdBox {
            version,
            flags,
            entry_count,
        })
    }
}

impl<W: Write> WriteBox<&mut BufWriter<W>> for StsdBox {
    fn write_box(&self, _writer: &mut BufWriter<W>) -> Result<u64> {
        // TODO
        Ok(0)
    }
}
