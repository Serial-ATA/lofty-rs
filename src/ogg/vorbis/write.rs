use crate::error::{FileEncodingError, Result};
use crate::ogg::constants::VORBIS_SETUP_HEAD;
use crate::types::file::FileType;

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub(crate) fn write_to(
	data: &mut File,
	writer: &mut Vec<u8>,
	first_md_content: Vec<u8>,
	mut pages: &mut [Page],
) -> Result<()> {
	let mut remaining = Vec::new();

	let reached_md_end: bool;

	// Find the total comment count in the first page's content
	let mut c = Cursor::new(first_md_content);

	// Skip the header
	c.seek(SeekFrom::Start(7))?;

	// Skip the vendor
	let vendor_len = c.read_u32::<LittleEndian>()?;
	c.seek(SeekFrom::Current(i64::from(vendor_len)))?;

	let total_comments = c.read_u32::<LittleEndian>()?;
	let comments_pos = c.seek(SeekFrom::Current(0))?;

	c.seek(SeekFrom::End(0))?;

	loop {
		let p = Page::read(data, false)?;

		if p.header_type() & 0x01 != 1 {
			data.seek(SeekFrom::Start(p.start as u64))?;
			data.read_to_end(&mut remaining)?;

			reached_md_end = true;
			break;
		}

		c.write_all(p.content())?;
	}

	if !reached_md_end {
		return Err(
			FileEncodingError::new(FileType::Vorbis, "File ends with comment header").into(),
		);
	}

	c.seek(SeekFrom::Start(comments_pos))?;

	for _ in 0..total_comments {
		let len = c.read_u32::<LittleEndian>()?;
		c.seek(SeekFrom::Current(i64::from(len)))?;
	}

	if c.read_u8()? != 1 {
		return Err(FileEncodingError::new(
			FileType::Vorbis,
			"Comment header is missing a framing bit",
		)
		.into());
	}

	// Comments should be followed by the setup header
	let mut header_ident = [0; 7];
	c.read_exact(&mut header_ident)?;

	if header_ident != VORBIS_SETUP_HEAD {
		return Err(
			FileEncodingError::new(FileType::Vorbis, "File is missing setup header").into(),
		);
	}

	c.seek(SeekFrom::Current(-7))?;

	let mut setup = Vec::new();
	c.read_to_end(&mut setup)?;

	// Safe to unwrap, since `pages` is guaranteed to not be empty
	let (last_page, remaining_pages) = pages.split_last_mut().unwrap();
	pages = remaining_pages;

	for p in pages.iter_mut() {
		p.gen_crc()?;
		writer.write_all(&*p.as_bytes()?)?;
	}

	build_remaining_header(writer, last_page, &*setup)?;

	writer.write_all(&*remaining)?;

	Ok(())
}

fn build_remaining_header(
	writer: &mut Vec<u8>,
	last_page: &mut Page,
	setup_header: &[u8],
) -> Result<()> {
	let mut segment_table = ogg_pager::segment_table(last_page.content().len())?;
	let seg_table_len = segment_table.len();

	if seg_table_len == 255 {
		last_page.gen_crc()?;

		let p_bytes = last_page.as_bytes()?;
		writer.write_all(&*p_bytes)?;
		return Ok(());
	}

	// The segment tables of current page and the setup header have to be combined
	if seg_table_len < 255 {
		let remaining_segments = 255 - seg_table_len;
		let setup_segment_table = ogg_pager::segment_table(setup_header.len())?;

		let mut i = 0;
		for e in setup_segment_table {
			segment_table.push(e);
			i += 1;

			if i == remaining_segments {
				break;
			}
		}
	}

	// Add the number of segments to the front of the table
	segment_table.insert(0, segment_table.len() as u8);

	let page = last_page.extend(setup_header)?;

	let mut p_bytes = last_page.as_bytes()?;
	let seg_count = p_bytes[26] as usize;

	// Replace segment table and checksum
	p_bytes.splice(26..27 + seg_count, segment_table);
	p_bytes.splice(22..26, ogg_pager::crc32(&*p_bytes).to_le_bytes());

	writer.write_all(&*p_bytes)?;

	if let Some(mut page) = page {
		page.gen_crc()?;

		writer.write_all(&*page.as_bytes()?)?;
	}

	Ok(())
}
