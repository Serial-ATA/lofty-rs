use crate::config::WriteOptions;
use crate::error::{FileEncodingError, FileParseError, TagEncodingError, TooMuchDataError};
use crate::file::FileType;
use crate::io::{Truncate, VerifiedFile};
use crate::macros::try_vec;
use crate::ogg::constants::{OPUSTAGS, VORBIS_COMMENT_HEAD};
use crate::ogg::tag::error::VorbisCommentsEncodingError;
use crate::ogg::tag::{VorbisCommentsRef, create_vorbis_comments_ref};
use crate::ogg::verify_signature;
use crate::picture::{Picture, PictureInformation};
use crate::tag::Tag;
use crate::util::io::FileLike;

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ogg_pager::{CONTAINS_FIRST_PAGE_OF_BITSTREAM, crc32, paginate};

pub(crate) fn write_to<F>(
	file: VerifiedFile<'_, F>,
	tag: &Tag,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
{
	let (vendor, items, pictures) = create_vorbis_comments_ref(tag);

	let mut comments_ref = VorbisCommentsRef {
		vendor: Cow::from(vendor),
		items,
		pictures,
	};

	write(file, &mut comments_ref, write_options)
}

// A raw OGG page: the 27-byte header, the segment table, and the body
struct RawPage {
	header: [u8; 27],
	segments: Vec<u8>,
	body: Vec<u8>,
}

impl RawPage {
	// Parses a single page from the front of `data`, advancing it past the page
	fn parse(data: &mut &[u8]) -> Result<Self, FileParseError> {
		if data.len() < 27 {
			return Err(FileParseError::message(None, "truncated page header"));
		}
		if &data[..4] != b"OggS" {
			return Err(FileParseError::message(None, "missing page magic"));
		}

		let nsegs = data[26] as usize;
		if nsegs == 0 || data.len() < 27 + nsegs {
			return Err(FileParseError::message(None, "invalid segment table"));
		}

		let segments = data[27..27 + nsegs].to_vec();
		let body_len: usize = segments.iter().map(|&b| b as usize).sum();
		let total = 27 + nsegs + body_len;
		if total > data.len() {
			return Err(FileParseError::message(None, "truncated page body"));
		}

		let header = data[..27].try_into().unwrap();
		let body = data[27 + nsegs..total].to_vec();
		*data = &data[total..];

		Ok(Self {
			header,
			segments,
			body,
		})
	}

	fn to_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(27 + self.segments.len() + self.body.len());
		bytes.extend_from_slice(&self.header);
		bytes.extend_from_slice(&self.segments);
		bytes.extend_from_slice(&self.body);
		bytes
	}
}

// Renumber a page and recompute its checksum
fn fixup_page(raw: &mut [u8], seq: u32) {
	raw[18..22].copy_from_slice(&seq.to_le_bytes());
	raw[22..26].fill(0);
	let checksum = crc32(raw);
	raw[22..26].copy_from_slice(&checksum.to_le_bytes());
}

pub(in crate::ogg) fn write<'a, F, II, IP>(
	file: VerifiedFile<'_, F>,
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	_write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	// TODO: Would be nice if we didn't have to read just to seek and reread immediately
	let format = file.format();
	let (header_packet_count, comment_signature) = match format {
		FileType::Opus => (2, Some(OPUSTAGS)),
		FileType::Vorbis => (3, Some(VORBIS_COMMENT_HEAD)),
		FileType::Speex => (2, None),
		_ => unreachable!("file type verified beforehand"),
	};

	let mut file = file.into_inner();

	// Read the whole file and work at page granularity.
	//
	// NOTE: The header packets must not be read at packet granularity (e.g. via
	// `Packets::read_count`): if the last header packet ends mid-page, the
	// reader is left in the middle of a page, the remaining content no longer
	// starts on a page boundary, and the audio pages can no longer be parsed.
	let mut file_content = Vec::new();
	file.read_to_end(&mut file_content)?;

	let mut pages = Vec::new();
	let mut rest = &file_content[..];
	while !rest.is_empty() {
		pages.push(RawPage::parse(&mut rest)?);
	}
	drop(file_content);

	if pages.is_empty() {
		return Err(FileParseError::message(None, "no pages found").into());
	}

	let stream_serial = u32::from_le_bytes(pages[0].header[14..18].try_into().unwrap());

	// Reassemble the first `header_packet_count` packets from the pages.
	//
	// `last_header_page` is the page on which the last header packet ends and
	// `audio_start_seg` is the index of the first segment on that page which
	// belongs to audio data, i.e. the last header packet ended mid-page.
	let mut header_packets: Vec<Vec<u8>> = Vec::with_capacity(header_packet_count as usize);
	let mut current_packet: Vec<u8> = Vec::new();
	let mut last_header_page = 0usize;
	let mut audio_start_seg = 0usize;

	'pages: for (page_idx, page) in pages.iter().enumerate() {
		let mut body_off = 0usize;
		for (seg_idx, &lacing) in page.segments.iter().enumerate() {
			let n = lacing as usize;
			current_packet.extend_from_slice(&page.body[body_off..body_off + n]);
			body_off += n;

			if lacing < 255 {
				header_packets.push(std::mem::take(&mut current_packet));

				if header_packets.len() == header_packet_count as usize {
					last_header_page = page_idx;
					audio_start_seg = seg_idx + 1;
					break 'pages;
				}
			}
		}
	}

	if header_packets.len() != header_packet_count as usize {
		return Err(FileParseError::message(None, "missing header packets").into());
	}

	let comment_packet = &header_packets[1];

	if let Some(comment_signature) = comment_signature {
		verify_signature(comment_packet, comment_signature)?;
	}

	let comment_signature = comment_signature.unwrap_or_default();

	// Retain the file's vendor string
	let md_reader = &mut &comment_packet[comment_signature.len()..];

	let vendor_len = md_reader.read_u32::<LittleEndian>()?;
	let mut vendor = try_vec![0; vendor_len as usize]?;
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

	let add_framing_bit = format == FileType::Vorbis;
	let new_metadata_packet = create_metadata_packet(tag, comment_signature, add_framing_bit)
		.map_err(TagEncodingError::from)?;

	// Replace the old comment packet
	header_packets[1] = new_metadata_packet;

	// Re-paginate the header packets into fresh pages
	let header_pages = paginate(
		header_packets.iter().map(Vec::as_slice),
		stream_serial,
		0,
		CONTAINS_FIRST_PAGE_OF_BITSTREAM,
	)
	.map_err(|e| FileEncodingError::new(format, e.into()))?;

	// The audio pages are written as they are renumbered and re-checksummed:
	// first the rest of the page on which the last header packet ended (if it
	// ended mid-page), then all subsequent pages.
	let mut next_seq = header_pages.len() as u32;

	file.rewind()?;
	file.truncate(0)?;

	for mut page in header_pages {
		page.gen_crc();
		file.write_all(&page.as_bytes())?;
	}

	{
		let page = &pages[last_header_page];
		if audio_start_seg < page.segments.len() {
			// The last header packet ended mid-page: re-emit the remainder of
			// the page as the first audio page, with the header packets'
			// segments removed.
			let segments = &page.segments[audio_start_seg..];
			let body_len: usize = segments.iter().map(|&b| b as usize).sum();
			let body = &page.body[page.body.len() - body_len..];

			let mut raw = Vec::with_capacity(27 + segments.len() + body_len);
			raw.extend_from_slice(&page.header);
			raw.extend_from_slice(segments);
			raw.extend_from_slice(body);

			// The page now has fewer segments than the original
			raw[26] = segments.len() as u8;

			// The first packet on this page is now a fresh audio packet, so
			// the page neither continues a packet from the previous page nor
			// contains the first page of the bitstream.
			raw[5] &= !0x03;

			fixup_page(&mut raw, next_seq);
			file.write_all(&raw)?;
			next_seq += 1;
		}
	}

	for page in &pages[last_header_page + 1..] {
		let mut raw = page.to_bytes();
		fixup_page(&mut raw, next_seq);
		file.write_all(&raw)?;
		next_seq += 1;
	}

	Ok(())
}

pub(in crate::ogg) fn create_metadata_packet<'a, II, IP>(
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	comment_signature: &[u8],
	add_framing_bit: bool,
) -> Result<Vec<u8>, VorbisCommentsEncodingError>
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
) -> Result<(), VorbisCommentsEncodingError> {
	for (k, v) in items {
		if v.is_empty() {
			continue;
		}

		let comment = format!("{k}={v}");
		let comment_bytes = comment.as_bytes();

		let Ok(bytes_len) = u32::try_from(comment_bytes.len()) else {
			return Err(TooMuchDataError.into());
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
) -> Result<(), VorbisCommentsEncodingError> {
	const PICTURE_KEY: &str = "METADATA_BLOCK_PICTURE=";

	for (pic, info) in pictures {
		let picture = pic.as_flac_bytes(info, true);

		let Ok(bytes_len) = u32::try_from(picture.len() + PICTURE_KEY.len()) else {
			return Err(TooMuchDataError.into());
		};

		*count += 1;

		packet.write_u32::<LittleEndian>(bytes_len)?;
		packet.write_all(PICTURE_KEY.as_bytes())?;
		packet.write_all(&picture)?;
	}

	Ok(())
}
