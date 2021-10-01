mod chunk_file;
mod frame;

use super::find_id3v2;
use crate::error::Result;
use crate::logic::id3::synch_u32;
use crate::types::tag::{Tag, TagFlags};

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ByteOrder, WriteBytesExt};

pub(in crate::logic) fn write_id3v2(data: &mut File, tag: &Tag) -> Result<()> {
	if tag.item_count() == 0 {
		find_id3v2(data, false)?;

		if data.seek(SeekFrom::Current(0))? != 0 {
			let mut file_bytes = Vec::new();
			data.read_to_end(&mut file_bytes)?;

			data.seek(SeekFrom::Start(0))?;
			data.set_len(0)?;
			data.write_all(&*file_bytes)?;
		}

		return Ok(());
	}

	let id3v2 = create_tag(tag)?;

	// find_id3v2 will seek us to the end of the tag
	find_id3v2(data, false)?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	file_bytes.splice(0..0, id3v2);

	data.seek(SeekFrom::Start(0))?;
	data.set_len(0)?;
	data.write_all(&*file_bytes)?;

	Ok(())
}

// Formats such as WAV and AIFF store the ID3v2 tag in an 'ID3 ' chunk rather than at the beginning of the file
pub(in crate::logic) fn write_id3v2_to_chunk_file<B>(data: &mut File, tag: &Tag) -> Result<()>
where
	B: ByteOrder,
{
	let id3v2 = if tag.item_count() == 0 {
		Vec::new()
	} else {
		create_tag(tag)?
	};
	chunk_file::write_to_chunk_file::<B>(data, &id3v2)?;

	Ok(())
}

fn create_tag(tag: &Tag) -> Result<Vec<u8>> {
	let mut id3v2 = create_tag_header(tag.flags())?;
	let header_len = id3v2.get_ref().len();

	// Write the items
	frame::create_items(&mut id3v2, tag.items())?;

	let len = id3v2.get_ref().len() - header_len;

	// Go back to the start and write the final size
	id3v2.seek(SeekFrom::Start(6))?;
	id3v2.write_u32::<BigEndian>(synch_u32(len as u32)?)?;

	Ok(id3v2.into_inner())
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn create_tag_header(flags: &TagFlags) -> Result<Cursor<Vec<u8>>> {
	let mut header = Cursor::new(Vec::new());

	header.write_all(&[b'I', b'D', b'3'])?;

	let mut tag_flags = 0;

	// Version 4, rev 0
	header.write_all(&[4, 0])?;

	#[cfg(not(feature = "id3v2_restrictions"))]
	let extended_header = flags.crc;

	#[cfg(feature = "id3v2_restrictions")]
	let extended_header = flags.crc || flags.restrictions.0;

	if flags.experimental {
		tag_flags |= 0x20
	}

	if extended_header {
		tag_flags |= 0x40
	}

	if flags.unsynchronisation {
		tag_flags |= 0x80
	}

	header.write_u8(tag_flags)?;
	header.write_u32::<BigEndian>(0)?;

	if extended_header {
		// Size (4)
		// Number of flag bytes (1)
		// Flags (1)
		header.write_all(&[0, 0, 0, 0, 1, 0])?;

		let mut size = 6_u32;
		let mut ext_flags = 0_u8;

		if flags.crc {
			// TODO
			ext_flags |= 0x20;
			size += 5;

			header.write_all(&[5, 0, 0, 0, 0, 0])?;
		}

		#[cfg(feature = "id3v2_restrictions")]
		if flags.restrictions.0 {
			ext_flags |= 0x10;
			size += 2;

			header.write_u8(1)?;
			header.write_u8(flags.restrictions.1.as_bytes())?;
		}

		header.seek(SeekFrom::Start(10))?;

		header.write_u32::<BigEndian>(synch_u32(size)?)?;
		header.seek(SeekFrom::Current(1))?;
		header.write_u8(ext_flags)?;

		header.seek(SeekFrom::End(0))?;
	}

	Ok(header)
}
