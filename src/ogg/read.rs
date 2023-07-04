use super::tag::VorbisComments;
use super::verify_signature;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{decode_err, err, parse_mode_choice};
use crate::picture::Picture;
use crate::probe::ParsingMode;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::{Packets, PageHeader};

pub type OGGTags = (Option<VorbisComments>, PageHeader, Packets);

pub(crate) fn read_comments<R>(
	data: &mut R,
	mut len: u64,
	parse_mode: ParsingMode,
) -> Result<VorbisComments>
where
	R: Read,
{
	use crate::macros::try_vec;

	let vendor_len = data.read_u32::<LittleEndian>()?;
	if u64::from(vendor_len) > len {
		err!(SizeMismatch);
	}

	let mut vendor = try_vec![0; vendor_len as usize];
	data.read_exact(&mut vendor)?;

	len -= u64::from(vendor_len);

	let vendor = match String::from_utf8(vendor) {
		Ok(v) => v,
		Err(e) => {
			// The actions following this are not spec-compliant in the slightest, so
			// we need to short circuit if strict.
			if parse_mode == ParsingMode::Strict {
				return Err(LoftyError::new(ErrorKind::StringFromUtf8(e)));
			}

			// Some vendor strings have invalid mixed UTF-8 and UTF-16 encodings.
			// This seems to work, while preserving the string opposed to using
			// the replacement character
			let s = e
				.as_bytes()
				.iter()
				.map(|c| u16::from(*c))
				.collect::<Vec<_>>();

			match String::from_utf16(&s) {
				Ok(vendor) => vendor,
				Err(_) => decode_err!(@BAIL "OGG: File has an invalid vendor string"),
			}
		},
	};

	let comments_total_len = data.read_u32::<LittleEndian>()?;

	let mut tag = VorbisComments {
		vendor,
		items: Vec::with_capacity(comments_total_len as usize),
		pictures: Vec::new(),
	};

	for _ in 0..comments_total_len {
		let comment_len = data.read_u32::<LittleEndian>()?;
		if u64::from(comment_len) > len {
			err!(SizeMismatch);
		}

		let mut comment_bytes = try_vec![0; comment_len as usize];
		data.read_exact(&mut comment_bytes)?;

		len -= u64::from(comment_len);

		let comment = String::from_utf8(comment_bytes)?;
		let mut comment_split = comment.splitn(2, '=');

		let key = match comment_split.next() {
			Some(k) => k,
			None => continue,
		};

		// Make sure there was a separator present, otherwise just move on
		if let Some(value) = comment_split.next() {
			match key {
				k if k.eq_ignore_ascii_case("METADATA_BLOCK_PICTURE") => {
					let picture;
					match Picture::from_flac_bytes(value.as_bytes(), true) {
						Ok(pic) => picture = pic,
						Err(e) => {
							parse_mode_choice!(
								parse_mode,
								RELAXED: continue,
								DEFAULT: return Err(e)
							)
						},
					}

					tag.pictures.push(picture)
				},
				// The valid range is 0x20..=0x7D not including 0x3D
				k if k.chars().all(|c| (' '..='}').contains(&c) && c != '=') => {
					tag.items.push((k.to_string(), value.to_string()))
				},
				_ => {
					parse_mode_choice!(
						parse_mode,
						STRICT: decode_err!(@BAIL "OGG: Vorbis comments contain an invalid key"),
						// Otherwise discard invalid keys
					)
				},
			}
		}
	}

	Ok(tag)
}

pub(crate) fn read_from<T>(
	data: &mut T,
	header_sig: &[u8],
	comment_sig: &[u8],
	packets_to_read: isize,
	parse_mode: ParsingMode,
) -> Result<OGGTags>
where
	T: Read + Seek,
{
	debug_assert!(packets_to_read >= 2);

	// TODO: Would be nice if we didn't have to read just to seek and reread immediately
	let start = data.stream_position()?;
	let first_page_header = PageHeader::read(data)?;

	data.seek(SeekFrom::Start(start))?;

	// Read the header packets
	let packets = Packets::read_count(data, packets_to_read)?;

	let identification_packet = packets
		.get(0)
		.ok_or_else(|| decode_err!("OGG: Expected identification packet"))?;
	verify_signature(identification_packet, header_sig)?;

	let mut metadata_packet = packets
		.get(1)
		.ok_or_else(|| decode_err!("OGG: Expected comment packet"))?;
	verify_signature(metadata_packet, comment_sig)?;

	// Remove the signature from the packet
	metadata_packet = &metadata_packet[comment_sig.len()..];

	let reader = &mut metadata_packet;
	let tag = read_comments(reader, reader.len() as u64, parse_mode)?;

	Ok((Some(tag), first_page_header, packets))
}
