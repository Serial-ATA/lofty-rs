mod error;
mod crc;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub use error::{PageError, Result};

#[derive(Clone)]
pub struct Page {
    pub content: Vec<u8>,
    pub header_type: u8,
    pub abgp: u64,
    pub serial: u32,
    pub seq_num: u32,
    pub checksum: u32,
    pub start: usize,
    pub end: usize,
}

impl Page {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let segments = self.segments();
        let segment_count = [segments.len() as u8];

        bytes.extend(b"OggS".iter());
        bytes.extend([0_u8].iter());
        bytes.extend(self.header_type.to_le_bytes().iter());
        bytes.extend(self.abgp.to_le_bytes().iter());
        bytes.extend(self.serial.to_le_bytes().iter());
        bytes.extend(self.seq_num.to_le_bytes().iter());
        bytes.extend(self.checksum.to_le_bytes().iter());
        bytes.extend(segment_count.iter());
        bytes.extend(segments.iter());
        bytes.extend(self.content.iter());

        bytes
    }

    pub fn segments(&self) -> Vec<u8> {
        let len = self.content.len();

        let mut last_len = (len % 255) as u8;
        if last_len == 0 {
            last_len = 255
        }

        let mut needed = len / 255;
        if needed != 255 {
            needed += 1
        }

        let mut segments = Vec::new();

        for i in 0..needed {
            if i + 1 < needed {
                segments.push(255)
            } else {
                segments.push(last_len)
            }
        }

        segments
    }

    pub fn read<V>(mut data: V) -> Result<Self>
    where
        V: Read + Seek,
    {
        let start = data.seek(SeekFrom::Current(0))? as usize;

        let mut sig = [0; 4];
        data.read_exact(&mut sig)?;

        if &sig != b"OggS" {
            return Err(PageError::MissingMagic);
        }

        // Version, always 0
        let version = data.read_u8()?;

        if version != 0 {
            return Err(PageError::InvalidVersion);
        }

        let header_type = data.read_u8()?;

        let abgp = data.read_u64::<LittleEndian>()?;
        let serial = data.read_u32::<LittleEndian>()?;
        let seq_num = data.read_u32::<LittleEndian>()?;
        let checksum = data.read_u32::<LittleEndian>()?;

        let segments = data.read_u8()?;

        if segments < 1 {
            return Err(PageError::BadSegmentCount);
        }

        let mut segment_table = vec![0; segments as usize];
        data.read_exact(&mut segment_table)?;

        let mut content = vec![0; segment_table.iter().map(|&b| b as usize).sum()];
        data.read_exact(&mut content)?;

        let end = data.seek(SeekFrom::Current(0))? as usize;

        Ok(Page {
            content,
            header_type,
            abgp,
            serial,
            seq_num,
            checksum,
            start,
            end,
        })
    }

    pub fn gen_crc(&mut self) {
        self.checksum = crc::crc32(&*self.as_bytes());
    }
}
