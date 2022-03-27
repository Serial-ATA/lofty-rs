use super::verify_signature;
use crate::error::{ErrorKind, FileEncodingError, LoftyError, Result};
use crate::file::FileType;
use crate::flac;
use crate::macros::try_vec;
use crate::ogg::constants::{OPUSTAGS, VORBIS_COMMENT_HEAD};
use crate::ogg::tag::{create_vorbis_comments_ref, VorbisCommentsRef};
use crate::picture::PictureInformation;
use crate::tag::{Tag, TagType};

use std::convert::TryFrom;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ogg_pager::Page;

#[derive(PartialEq, Copy, Clone)]
pub(crate) enum OGGFormat {
	Opus,
	Vorbis,
	Speex,
}

impl OGGFormat {
	#[allow(clippy::trivially_copy_pass_by_ref)]
	pub(crate) fn comment_signature(&self) -> Option<&[u8]> {
		match self {
			OGGFormat::Opus => Some(OPUSTAGS),
			OGGFormat::Vorbis => Some(VORBIS_COMMENT_HEAD),
			OGGFormat::Speex => None,
		}
	}
}

pub(in crate) fn write_to(file: &mut File, tag: &Tag, file_type: FileType) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "vorbis_comments")]
		TagType::VorbisComments => {
			let (vendor, items, pictures) = create_vorbis_comments_ref(tag);

			let mut comments_ref = VorbisCommentsRef {
				vendor,
				items,
				pictures,
			};

			if file_type == FileType::FLAC {
				return flac::write::write_to(file, &mut comments_ref);
			}

			let format = match file_type {
				FileType::Opus => OGGFormat::Opus,
				FileType::Vorbis => OGGFormat::Vorbis,
				FileType::Speex => OGGFormat::Speex,
				_ => unreachable!(),
			};

			write(file, &mut comments_ref, format)
		},
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 if file_type == FileType::FLAC => {
			// This tag can *only* be removed in this format
			crate::id3::v2::tag::Id3v2TagRef::empty().write_to(file)
		},
		_ => Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}
}

#[cfg(feature = "vorbis_comments")]
pub(crate) fn create_comments(
	packet: &mut impl Write,
	count: &mut u32,
	items: &mut dyn Iterator<Item = (&str, &str)>,
) -> Result<()> {
	for (k, v) in items {
		if v.is_empty() {
			continue;
		}

		let comment = format!("{}={}", k, v);

		let comment_b = comment.as_bytes();
		let bytes_len = comment_b.len();

		if u32::try_from(bytes_len as u64).is_ok() {
			*count += 1;

			packet.write_all(&(bytes_len as u32).to_le_bytes())?;
			packet.write_all(comment_b)?;
		}
	}

	Ok(())
}

#[cfg(feature = "vorbis_comments")]
pub(super) fn create_pages<'a, II, IP>(
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	writer: &mut Cursor<Vec<u8>>,
	stream_serial: u32,
	add_framing_bit: bool,
) -> Result<Vec<Page>>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a crate::picture::Picture, PictureInformation)>,
{
	const PICTURE_KEY: &str = "METADATA_BLOCK_PICTURE=";

	let item_count_pos = writer.seek(SeekFrom::Current(0))?;

	writer.write_u32::<LittleEndian>(0)?;

	let mut count = 0;
	create_comments(writer, &mut count, &mut tag.items)?;

	for (pic, _) in &mut tag.pictures {
		let picture = pic.as_flac_bytes(PictureInformation::from_picture(pic)?, true);

		let bytes_len = picture.len() + PICTURE_KEY.len();

		if u32::try_from(bytes_len as u64).is_ok() {
			count += 1;

			writer.write_u32::<LittleEndian>(bytes_len as u32)?;
			writer.write_all(PICTURE_KEY.as_bytes())?;
			writer.write_all(&*picture)?;
		}
	}

	if add_framing_bit {
		// OGG Vorbis makes use of a "framing bit" to
		// separate the header packets
		//
		// https://xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-590004
		writer.write_u8(1)?;
	}

	let packet_end = writer.seek(SeekFrom::Current(0))?;

	writer.seek(SeekFrom::Start(item_count_pos))?;
	writer.write_u32::<LittleEndian>(count)?;
	writer.seek(SeekFrom::Start(packet_end))?;

	// Checksum is calculated later
	Ok(ogg_pager::paginate(writer.get_ref(), stream_serial, 0, 0))
}

#[cfg(feature = "vorbis_comments")]
pub(super) fn write<'a, II, IP>(
	data: &mut File,
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	format: OGGFormat,
) -> Result<()>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a crate::picture::Picture, PictureInformation)>,
{
	let first_page = Page::read(data, false)?;

	let ser = first_page.serial;

	let mut writer = Vec::new();
	writer.write_all(&*first_page.as_bytes()?)?;

	let first_md_page = Page::read(data, false)?;

	let comment_signature = format.comment_signature();
	if let Some(comment_signature) = comment_signature {
		verify_signature(&first_md_page, comment_signature)?;
	}

	let comment_signature = comment_signature.unwrap_or_default();

	// Retain the file's vendor string
	let md_reader = &mut &first_md_page.content()[comment_signature.len()..];

	let vendor_len = md_reader.read_u32::<LittleEndian>()?;
	let mut vendor = try_vec![0; vendor_len as usize];
	md_reader.read_exact(&mut vendor)?;

	let mut packet = Cursor::new(Vec::new());

	packet.write_all(comment_signature)?;
	packet.write_u32::<LittleEndian>(vendor_len)?;
	packet.write_all(&vendor)?;

	let needs_framing_bit = format == OGGFormat::Vorbis;
	let mut pages = create_pages(tag, &mut packet, ser, needs_framing_bit)?;

	match format {
		OGGFormat::Vorbis => {
			super::vorbis::write::write_to(
				data,
				&mut writer,
				first_md_page.take_content(),
				&mut pages,
			)?;
		},
		OGGFormat::Opus => {
			replace_packet(data, &mut writer, &mut pages, FileType::Opus)?;
		},
		OGGFormat::Speex => {
			replace_packet(data, &mut writer, &mut pages, FileType::Speex)?;
		},
	}

	data.seek(SeekFrom::Start(0))?;
	data.set_len(first_page.end)?;
	data.write_all(&*writer)?;

	Ok(())
}

fn replace_packet(
	data: &mut File,
	writer: &mut Vec<u8>,
	pages: &mut [Page],
	file_type: FileType,
) -> Result<()> {
	let reached_md_end: bool;

	loop {
		let p = Page::read(data, true)?;

		if p.header_type() & 0x01 != 0x01 {
			data.seek(SeekFrom::Start(p.start))?;
			reached_md_end = true;
			break;
		}
	}

	if !reached_md_end {
		return Err(FileEncodingError::new(file_type, "File ends with comment header").into());
	}

	let mut remaining = Vec::new();
	data.read_to_end(&mut remaining)?;

	for p in pages.iter_mut() {
		p.gen_crc()?;

		writer.write_all(&*p.as_bytes()?)?;
	}

	writer.write_all(&*remaining)?;

	Ok(())
}
