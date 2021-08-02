use crate::{FileProperties, LoftyError, Picture, Result, Tag};

use std::io::{Read, Seek};

use ogg_pager::Page;
use crate::components::logic::ogg::constants::{VORBIS_IDENT_HEAD, VORBIS_COMMENT_HEAD, OPUSHEAD, OPUSTAGS};

pub(crate) mod constants;
pub(crate) mod read;
pub(crate) mod write;

#[cfg(feature = "format-flac")]
pub(crate) mod flac;
#[cfg(feature = "format-opus")]
mod opus;
#[cfg(feature = "format-vorbis")]
mod vorbis;

#[cfg(feature = "format-opus")]
pub struct OpusFile {
	pub properties: FileProperties,
	pub pictures: Vec<Picture>,
	pub vorbis_comments: Tag,
}

impl OpusFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::flac::read_from(reader)
	}
}

#[cfg(feature = "format-flac")]
pub struct FlacFile {
	pub properties: FileProperties,
	pub pictures: Vec<Picture>,
	pub vorbis_comments: Tag,
}

impl FlacFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::read::read_from(reader, OPUSHEAD, OPUSTAGS)
	}
}

#[cfg(feature = "format-vorbis")]
pub struct VorbisFile {
	pub properties: FileProperties,
	pub pictures: Vec<Picture>,
	pub vorbis_comments: Tag,
}

impl VorbisFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::read::read_from(reader, VORBIS_IDENT_HEAD, VORBIS_COMMENT_HEAD)
	}
}

pub fn page_from_packet(packet: &mut [u8]) -> Result<Vec<Page>> {
	let mut pages: Vec<Page> = Vec::new();

	let reader = &mut &packet[..];

	let mut start = 0_u64;
	let mut i = 0;

	while !reader.is_empty() {
		let header_type = if i == 0 { 0 } else { 1_u8 };

		let size = std::cmp::min(65025_u64, reader.len() as u64);

		if i != 0 {
			if let Some(s) = start.checked_add(size) {
				start = s
			} else {
				return Err(LoftyError::TooMuchData);
			}
		}

		let mut content = vec![0; size as usize];
		reader.read_exact(&mut content)?;

		let end = start + size;

		pages.push(Page {
			content,
			header_type,
			abgp: 0,
			serial: 0, // Retrieved later
			seq_num: (i + 1) as u32,
			checksum: 0, // Calculated later
			start,
			end,
		});

		i += 1;
	}

	Ok(pages)
}

pub(self) fn verify_signature(page: &Page, sig: &[u8]) -> Result<()> {
	let sig_len = sig.len();

	if page.content.len() < sig_len || &page.content[..sig_len] != sig {
		return Err(LoftyError::Ogg("File missing magic signature"));
	}

	Ok(())
}

pub(self) fn find_last_page<R>(data: &mut R) -> Result<Page>
where
	R: Read + Seek,
{
	let mut last_page = Page::read(data, true)?;

	while let Ok(page) = Page::read(data, true) {
		last_page = page
	}

	Ok(last_page)
}
