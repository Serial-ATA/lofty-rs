use crate::{FileProperties, Result};

use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub(in crate::components) fn read_properties<R>(
	data: &mut R,
	first_page: Page,
	stream_len: u64,
) -> Result<FileProperties>
where
	R: Read + Seek,
{
	Ok(FileProperties::default())
}
