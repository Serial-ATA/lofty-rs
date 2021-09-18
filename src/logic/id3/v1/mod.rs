pub(crate) mod constants;
pub(in crate::logic) mod read;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::types::tag::Tag;

use std::io::{Read, Seek, SeekFrom};

pub(in crate::logic) fn find_id3v1<R>(data: &mut R, read: bool) -> Result<(bool, Option<Tag>)>
where
	R: Read + Seek,
{
	let mut id3v1 = None;
	let mut exists = false;

	data.seek(SeekFrom::End(-128))?;

	let mut id3v1_header = [0; 3];
	data.read_exact(&mut id3v1_header)?;

	data.seek(SeekFrom::Current(-3))?;

	if &id3v1_header == b"TAG" {
		exists = true;

		if read {
			let mut id3v1_tag = [0; 128];
			data.read_exact(&mut id3v1_tag)?;

			data.seek(SeekFrom::End(-128))?;

			id3v1 = Some(read::parse_id3v1(id3v1_tag))
		}
	} else {
		// No ID3v1 tag found
		data.seek(SeekFrom::End(0))?;
	}

	Ok((exists, id3v1))
}
