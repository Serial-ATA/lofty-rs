use super::verify_signature;
use crate::error::Result;
use crate::file::FileType;
use crate::macros::{decode_err, err, try_vec};
use crate::ogg::constants::{OPUSTAGS, VORBIS_COMMENT_HEAD};
use crate::ogg::tag::{create_vorbis_comments_ref, VorbisCommentsRef};
use crate::picture::PictureInformation;
use crate::tag::{Tag, TagType};

use std::convert::TryFrom;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ogg_pager::{Packets, PageHeader, CONTAINS_FIRST_PAGE_OF_BITSTREAM};

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

pub(crate) fn write_to(file: &mut File, tag: &Tag, file_type: FileType) -> Result<()> {
	if tag.tag_type() != TagType::VorbisComments {
		err!(UnsupportedTag);
	}

	let (vendor, items, pictures) = create_vorbis_comments_ref(tag);

	let mut comments_ref = VorbisCommentsRef {
		vendor,
		items,
		pictures,
	};

	let format = match file_type {
		FileType::Opus => OGGFormat::Opus,
		FileType::Vorbis => OGGFormat::Vorbis,
		FileType::Speex => OGGFormat::Speex,
		_ => unreachable!(),
	};

	write(file, &mut comments_ref, format)
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

		let comment = format!("{k}={v}");

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
pub(super) fn write<'a, II, IP>(
	file: &mut File,
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	format: OGGFormat,
) -> Result<()>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a crate::picture::Picture, PictureInformation)>,
{
	// TODO: Would be nice if we didn't have to read just to seek and reread immediately

	// Read the first page header to get the stream serial number
	let start = file.stream_position()?;
	let (first_page_header, _) = PageHeader::read(file)?;

	let stream_serial = first_page_header.stream_serial;

	file.seek(SeekFrom::Start(start))?;
	let mut packets = Packets::read_count(file, 3)?;

	let mut remaining_file_content = Vec::new();
	file.read_to_end(&mut remaining_file_content)?;

	let comment_packet = packets
		.get(1)
		.ok_or_else(|| decode_err!("OGG: Expected metadata packet"))?;

	let comment_signature = format.comment_signature();
	if let Some(comment_signature) = comment_signature {
		verify_signature(comment_packet, comment_signature)?;
	}

	let comment_signature = comment_signature.unwrap_or_default();

	// Retain the file's vendor string
	let md_reader = &mut &comment_packet[comment_signature.len()..];

	let vendor_len = md_reader.read_u32::<LittleEndian>()?;
	let mut vendor = try_vec![0; vendor_len as usize];
	md_reader.read_exact(&mut vendor)?;

	let add_framing_bit = format == OGGFormat::Vorbis;
	let new_metadata_packet =
		create_metadata_packet(tag, comment_signature, &vendor, add_framing_bit)?;

	// Replace the old comment packet
	packets.set(1, new_metadata_packet);

	file.rewind()?;
	file.set_len(0)?;

	packets.write_to(file, stream_serial, 0, CONTAINS_FIRST_PAGE_OF_BITSTREAM)?;

	file.write_all(&remaining_file_content)?;
	Ok(())
}

pub(super) fn create_metadata_packet<'a, II, IP>(
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	comment_signature: &[u8],
	vendor: &[u8],
	add_framing_bit: bool,
) -> Result<Vec<u8>>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a crate::picture::Picture, PictureInformation)>,
{
	const PICTURE_KEY: &str = "METADATA_BLOCK_PICTURE=";

	let mut new_comment_packet = Cursor::new(Vec::new());

	new_comment_packet.write_all(comment_signature)?;
	new_comment_packet.write_u32::<LittleEndian>(vendor.len() as u32)?;
	new_comment_packet.write_all(&vendor)?;

	let item_count_pos = new_comment_packet.stream_position()?;

	new_comment_packet.write_u32::<LittleEndian>(0)?;

	let mut count = 0;
	create_comments(&mut new_comment_packet, &mut count, &mut tag.items)?;

	for (pic, info) in &mut tag.pictures {
		let picture = pic.as_flac_bytes(info, true);

		let bytes_len = picture.len() + PICTURE_KEY.len();

		if u32::try_from(bytes_len as u64).is_ok() {
			count += 1;

			new_comment_packet.write_u32::<LittleEndian>(bytes_len as u32)?;
			new_comment_packet.write_all(PICTURE_KEY.as_bytes())?;
			new_comment_packet.write_all(&picture)?;
		}
	}

	let packet_end = new_comment_packet.stream_position()?;

	new_comment_packet.seek(SeekFrom::Start(item_count_pos))?;
	new_comment_packet.write_u32::<LittleEndian>(count)?;
	new_comment_packet.seek(SeekFrom::Start(packet_end))?;

	if add_framing_bit {
		// OGG Vorbis makes use of a "framing bit" to
		// separate the header packets
		//
		// https://xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-590004
		new_comment_packet.write_u8(1)?;
	}

	Ok(new_comment_packet.into_inner())
}
