use super::tag::VorbisComments;
use super::verify_signature;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{decode_err, err, parse_mode_choice};
use crate::picture::{MimeType, Picture, PictureInformation, PictureType};
use crate::probe::ParsingMode;

use std::borrow::Cow;
use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use data_encoding::BASE64;
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

		// KEY=VALUE
		let mut comment_split = comment_bytes.splitn(2, |b| *b == b'=');

		let key = match comment_split.next() {
			Some(k) => k,
			None => continue,
		};

		// Make sure there was a separator present, otherwise just move on
		let Some(value) = comment_split.next() else {
			log::warn!("No separator found, discarding field");
			continue;
		};

		match key {
			k if k.eq_ignore_ascii_case(b"METADATA_BLOCK_PICTURE") => {
				match Picture::from_flac_bytes(value, true, parse_mode) {
					Ok(picture) => tag.pictures.push(picture),
					Err(e) => {
						if parse_mode == ParsingMode::Strict {
							return Err(e);
						}

						log::warn!("Failed to decode FLAC picture, discarding field");
						continue;
					},
				}
			},
			k if k.eq_ignore_ascii_case(b"COVERART") => {
				// `COVERART` is an old deprecated image storage format. We have to convert it
				// to a `METADATA_BLOCK_PICTURE` for it to be useful.
				//
				// <https://wiki.xiph.org/VorbisComment#Conversion_to_METADATA_BLOCK_PICTURE>
				let picture_data = BASE64.decode(value);

				match picture_data {
					Ok(picture_data) => {
						let mime_type = Picture::mimetype_from_bin(&picture_data)
							.unwrap_or_else(|_| MimeType::Unknown(String::from("image/")));

						let picture = Picture {
							pic_type: PictureType::Other,
							mime_type,
							description: None,
							data: Cow::from(picture_data),
						};

						tag.pictures.push((picture, PictureInformation::default()))
					},
					Err(_) => {
						if parse_mode == ParsingMode::Strict {
							return Err(LoftyError::new(ErrorKind::NotAPicture));
						}

						log::warn!("Failed to decode FLAC picture, discarding field");
						continue;
					},
				}
			},
			// The valid range is 0x20..=0x7D not including 0x3D
			k if k.iter().all(|c| (b' '..=b'}').contains(c) && *c != b'=') => {
				// SAFETY: We just verified that all of the bytes fall within the subset of ASCII
				let key = unsafe { String::from_utf8_unchecked(k.to_vec()) };

				match String::from_utf8(value.to_vec()) {
					Ok(value) => tag.items.push((key, value)),
					Err(e) => {
						if parse_mode == ParsingMode::Strict {
							return Err(LoftyError::new(ErrorKind::StringFromUtf8(e)));
						}

						log::warn!("Non UTF-8 value found, discarding field");
						continue;
					},
				}
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
