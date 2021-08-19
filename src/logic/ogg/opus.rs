use super::find_last_page;
use crate::error::{LoftyError, Result};
use crate::logic::ogg::constants::{OPUSHEAD, OPUSTAGS};
use crate::types::file::AudioFile;
use crate::types::file::{FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

/// An OGG Opus file
pub struct OpusFile {
	/// The file's audio properties
	properties: FileProperties,
	/// The file vendor's name
	vendor: String,
	/// The vorbis comments contained in the file
	///
	/// NOTE: While a metadata packet is required, it isn't required to actually have any data.
	vorbis_comments: Tag,
}

impl Into<TaggedFile> for OpusFile {
	fn into(self) -> TaggedFile {
		TaggedFile {
			ty: FileType::Opus,
			properties: self.properties,
			tags: vec![self.vorbis_comments],
		}
	}
}

impl AudioFile for OpusFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let file_information = super::read::read_from(reader, OPUSHEAD, OPUSTAGS)?;

		Ok(Self {
			properties: file_information.2,
			vendor: file_information.0,
			vorbis_comments: file_information.1,
		})
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		true
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		if tag_type != &TagType::VorbisComments {
			return false;
		}

		true
	}
}

pub(crate) fn read_properties<R>(
	data: &mut R,
	first_page: &Page,
	stream_len: u64,
) -> Result<FileProperties>
where
	R: Read + Seek,
{
	let first_page_abgp = first_page.abgp;

	// Skip identification header and version
	let first_page_content = &mut &first_page.content[9..];

	let channels = first_page_content.read_u8()?;
	let pre_skip = first_page_content.read_u16::<LittleEndian>()?;
	let sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	// Subtract the identification and metadata packet length from the total
	let audio_size = stream_len - data.seek(SeekFrom::Current(0))?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	last_page_abgp
		.checked_sub(first_page_abgp + u64::from(pre_skip))
		.map_or_else(
			|| Err(LoftyError::Opus("File contains incorrect PCM values")),
			|frame_count| {
				let length = frame_count * 1000 / 48000;
				let duration = Duration::from_millis(length as u64);
				let bitrate = (audio_size * 8 / length) as u32;

				Ok(FileProperties::new(
					duration,
					Some(bitrate),
					Some(sample_rate),
					Some(channels),
				))
			},
		)
}

pub fn write_to(data: &mut File, writer: &mut Vec<u8>, ser: u32, pages: &mut [Page]) -> Result<()> {
	let reached_md_end: bool;
	let mut remaining = Vec::new();

	loop {
		let p = Page::read(data, true)?;

		if p.header_type != 1 {
			data.seek(SeekFrom::Start(p.start as u64))?;
			reached_md_end = true;
			break;
		}
	}

	if !reached_md_end {
		return Err(LoftyError::Opus("File ends with comment header"));
	}

	data.read_to_end(&mut remaining)?;

	for mut p in pages.iter_mut() {
		p.serial = ser;
		p.gen_crc();

		writer.write_all(&*p.as_bytes())?;
	}

	writer.write_all(&*remaining)?;

	Ok(())
}
