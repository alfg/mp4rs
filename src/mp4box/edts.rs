#[cfg(feature = "use_serde")]
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::elst::ElstBox;
use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "use_serde", derive(Serialize))]
pub struct EdtsBox {
    pub elst: Option<ElstBox>,
}

impl EdtsBox {
    pub(crate) fn new() -> EdtsBox {
        Default::default()
    }

    pub fn get_type(&self) -> BoxType {
        BoxType::EdtsBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(ref elst) = self.elst {
            size += elst.box_size();
        }
        size
    }
}

impl Mp4Box for EdtsBox {
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

impl<R: Read + Seek> ReadBox<&mut R> for EdtsBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let mut edts = EdtsBox::new();

        let header = BoxHeader::read(reader)?;
        let BoxHeader { name, size: s } = header;

        match name {
            BoxType::ElstBox => {
                let elst = ElstBox::read_box(reader, s)?;
                edts.elst = Some(elst);
            }
            _ => {}
        }

        skip_bytes_to(reader, start + size)?;

        Ok(edts)
    }
}

impl<W: Write> WriteBox<&mut W> for EdtsBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        if let Some(ref elst) = self.elst {
            elst.write_box(writer)?;
        }

        Ok(size)
    }
}
