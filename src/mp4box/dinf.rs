#[cfg(feature = "use_serde")]
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "use_serde", derive(Serialize))]
pub struct DinfBox {
    pub dref: DrefBox,
}

impl DinfBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::DinfBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.dref.box_size()
    }
}

impl Mp4Box for DinfBox {
    fn box_type(&self) -> BoxType {
        return self.get_type();
    }

    fn box_size(&self) -> u64 {
        return self.get_size();
    }

    #[cfg(feature = "use_serde")]
    #[cfg(feature = "use_serde")]
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("");
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for DinfBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let mut dref = None;

        let mut current = reader.seek(SeekFrom::Current(0))?;
        let end = start + size;
        while current < end {
            // Get box header.
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;

            match name {
                BoxType::DrefBox => {
                    dref = Some(DrefBox::read_box(reader, s)?);
                }
                _ => {
                    // XXX warn!()
                    skip_box(reader, s)?;
                }
            }

            current = reader.seek(SeekFrom::Current(0))?;
        }

        if dref.is_none() {
            return Err(Error::BoxNotFound(BoxType::DrefBox));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(DinfBox {
            dref: dref.unwrap(),
        })
    }
}

impl<W: Write> WriteBox<&mut W> for DinfBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;
        self.dref.write_box(writer)?;
        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize))]
pub struct DrefBox {
    pub version: u8,
    pub flags: u32,

    pub data_entries: Vec<UrlBox>,
}

impl Default for DrefBox {
    fn default() -> Self {
        DrefBox {
            version: 0,
            flags: 0,
            data_entries: vec![UrlBox::default()],
        }
    }
}

impl DrefBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::DrefBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE + 4;
        for entry in self.data_entries.iter() {
            size += entry.box_size();
        }
        size
    }
}

impl Mp4Box for DrefBox {
    fn box_type(&self) -> BoxType {
        return self.get_type();
    }

    fn box_size(&self) -> u64 {
        return self.get_size();
    }

    #[cfg(feature = "use_serde")]
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("");
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for DrefBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let mut current = reader.seek(SeekFrom::Current(0))?;

        let (version, flags) = read_box_header_ext(reader)?;
        let end = start + size;

        let mut data_entries = vec![];

        let entry_count = reader.read_u32::<BigEndian>()?;
        for _i in 0..entry_count {
            if current >= end {
                break;
            }

            // Get box header.
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;

            match name {
                BoxType::UrlBox => {
                    data_entries.push(UrlBox::read_box(reader, s)?);
                }
                _ => {
                    skip_box(reader, s)?;
                }
            }

            current = reader.seek(SeekFrom::Current(0))?;
        }

        skip_bytes_to(reader, start + size)?;

        Ok(DrefBox {
            version,
            flags,
            data_entries,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for DrefBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(1)?;

        for entry in self.data_entries.iter() {
            entry.write_box(writer)?;
        }

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize))]
pub struct UrlBox {
    pub version: u8,
    pub flags: u32,
    pub location: String,
}

impl Default for UrlBox {
    fn default() -> Self {
        UrlBox {
            version: 0,
            flags: 1,
            location: String::default(),
        }
    }
}

impl UrlBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::UrlBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;

        if !self.location.is_empty() {
            size += self.location.bytes().len() as u64 + 1;
        }

        size
    }
}

impl Mp4Box for UrlBox {
    fn box_type(&self) -> BoxType {
        return self.get_type();
    }

    fn box_size(&self) -> u64 {
        return self.get_size();
    }

    #[cfg(feature = "use_serde")]
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("location={}", self.location);
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for UrlBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let (version, flags) = read_box_header_ext(reader)?;

        println!("FOOBAR");

        let rem_size = size - HEADER_SIZE - HEADER_EXT_SIZE;

        let location = if rem_size > 0 {
            let buf_size = rem_size - 1;
            let mut buf = vec![0u8; buf_size as usize];
            reader.read_exact(&mut buf)?;
            match String::from_utf8(buf) {
                Ok(t) => {
                    assert_eq!(t.len(), buf_size as usize);
                    t
                }
                _ => String::default(),
            }
        } else {
            String::new()
        };

        skip_bytes_to(reader, start + size)?;

        Ok(UrlBox {
            version,
            flags,
            location,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for UrlBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if !self.location.is_empty() {
            writer.write(self.location.as_bytes())?;
            writer.write_u8(0)?;
        }

        Ok(size)
    }
}
