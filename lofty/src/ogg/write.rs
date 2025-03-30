use super::verify_signature;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::file::FileType;
use crate::macros::{decode_err, err, try_vec};
use crate::ogg::constants::{OPUSTAGS, VORBIS_COMMENT_HEAD};
use crate::ogg::tag::{VorbisCommentsRef, create_vorbis_comments_ref};
use crate::picture::{Picture, PictureInformation};
use crate::tag::{Tag, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ogg_pager::{CONTAINS_FIRST_PAGE_OF_BITSTREAM, Packets, Page, PageHeader};

#[derive(PartialEq, Copy, Clone)]
pub(crate) enum OGGFormat {
	Opus,
	Vorbis,
	Speex,
}

impl OGGFormat {
	pub(crate) fn comment_signature(self) -> Option<&'static [u8]> {
		match self {
			OGGFormat::Opus => Some(OPUSTAGS),
			OGGFormat::Vorbis => Some(VORBIS_COMMENT_HEAD),
			OGGFormat::Speex => None,
		}
	}

	pub(super) fn from_filetype(file_type: FileType) -> (Self, isize) {
		match file_type {
			FileType::Opus => (OGGFormat::Opus, 2),
			FileType::Vorbis => (OGGFormat::Vorbis, 3),
			FileType::Speex => (OGGFormat::Speex, 2),
			_ => unreachable!("You forgot to add support for FileType::{:?}!", file_type),
		}
	}
}

pub(crate) fn write_to<F>(
	file: &mut F,
	tag: &Tag,
	file_type: FileType,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	if tag.tag_type() != TagType::VorbisComments {
		err!(UnsupportedTag);
	}

	let (vendor, items, pictures) = create_vorbis_comments_ref(tag);

	let mut comments_ref = VorbisCommentsRef {
		vendor: Cow::from(vendor),
		items,
		pictures,
	};

	let (format, header_packet_count) = OGGFormat::from_filetype(file_type);

	write(
		file,
		&mut comments_ref,
		format,
		header_packet_count,
		write_options,
	)
}

pub(super) fn write<'a, F, II, IP>(
	file: &mut F,
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	format: OGGFormat,
	header_packet_count: isize,
	_write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	// TODO: Would be nice if we didn't have to read just to seek and reread immediately

	// Read the first page header to get the stream serial number
	let start = file.stream_position()?;
	let first_page_header = PageHeader::read(file)?;

	let stream_serial = first_page_header.stream_serial;

	file.seek(SeekFrom::Start(start))?;
	let mut packets = Packets::read_count(file, header_packet_count)?;

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

	let vendor_str;
	match String::from_utf8(vendor) {
		Ok(s) => vendor_str = Cow::Owned(s),
		Err(_) => {
			// TODO: Error on strict?
			log::warn!("OGG vendor string is not valid UTF-8, not re-using");
			vendor_str = Cow::Borrowed("");
		},
	}

	tag.vendor = vendor_str;

	let add_framing_bit = format == OGGFormat::Vorbis;
	let new_metadata_packet = create_metadata_packet(tag, comment_signature, add_framing_bit)?;

	// Replace the old comment packet
	packets.set(1, new_metadata_packet);

	file.rewind()?;
	file.truncate(0)?;

	let pages_written =
		packets.write_to(file, stream_serial, 0, CONTAINS_FIRST_PAGE_OF_BITSTREAM)? as u32;

	// Correct all remaining page sequence numbers
	let mut pages_reader = Cursor::new(&remaining_file_content[..]);
	let mut idx = 0;
	while let Ok(mut page) = Page::read(&mut pages_reader) {
		let header = page.header_mut();
		header.sequence_number = pages_written + idx;
		page.gen_crc();
		file.write_all(&page.as_bytes())?;

		idx += 1;
	}

	Ok(())
}

pub(super) fn create_metadata_packet<'a, II, IP>(
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	comment_signature: &[u8],
	add_framing_bit: bool,
) -> Result<Vec<u8>>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	let mut new_comment_packet = Cursor::new(Vec::new());

	let vendor_bytes = tag.vendor.as_bytes();
	new_comment_packet.write_all(comment_signature)?;
	new_comment_packet.write_u32::<LittleEndian>(vendor_bytes.len() as u32)?;
	new_comment_packet.write_all(vendor_bytes)?;

	// Zero out the item count for later
	let item_count_pos = new_comment_packet.stream_position()?;
	new_comment_packet.write_u32::<LittleEndian>(0)?;

	let mut count = 0;
	create_comments(&mut new_comment_packet, &mut count, &mut tag.items)?;
	create_pictures(&mut new_comment_packet, &mut count, &mut tag.pictures)?;

	// Seek back and write the item count
	new_comment_packet.seek(SeekFrom::Start(item_count_pos))?;
	new_comment_packet.write_u32::<LittleEndian>(count)?;

	if add_framing_bit {
		// OGG Vorbis makes use of a "framing bit" to
		// separate the header packets
		//
		// https://xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-590004
		new_comment_packet.get_mut().push(1);
	}

	Ok(new_comment_packet.into_inner())
}

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
		let comment_bytes = comment.as_bytes();

		let Ok(bytes_len) = u32::try_from(comment_bytes.len()) else {
			err!(TooMuchData);
		};

		*count += 1;

		packet.write_u32::<LittleEndian>(bytes_len)?;
		packet.write_all(comment_bytes)?;
	}

	Ok(())
}

fn create_pictures(
	packet: &mut impl Write,
	count: &mut u32,
	pictures: &mut dyn Iterator<Item = (&Picture, PictureInformation)>,
) -> Result<()> {
	const PICTURE_KEY: &str = "METADATA_BLOCK_PICTURE=";

	for (pic, info) in pictures {
		let picture = pic.as_flac_bytes(info, true);

		let Ok(bytes_len) = u32::try_from(picture.len() + PICTURE_KEY.len()) else {
			err!(TooMuchData);
		};

		*count += 1;

		packet.write_u32::<LittleEndian>(bytes_len)?;
		packet.write_all(PICTURE_KEY.as_bytes())?;
		packet.write_all(&picture)?;
	}

	Ok(())
}
