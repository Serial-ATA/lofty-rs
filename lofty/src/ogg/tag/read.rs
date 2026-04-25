use crate::config::{ParseOptions, ParsingMode};
use crate::error::SizeMismatchError;
use crate::ogg::tag::VorbisComments;
use crate::ogg::tag::error::VorbisCommentsParseError;
use crate::picture::error::PictureParseError;
use crate::picture::{MimeType, Picture, PictureInformation, PictureType};
use crate::tag::Accessor;
use crate::util::text::{utf8_decode, utf8_decode_str, utf16_decode};

use std::borrow::Cow;
use std::error::Error;
use std::io::Read;
use std::string::FromUtf8Error;

use byteorder::{LittleEndian, ReadBytesExt};
use data_encoding::BASE64;
use ogg_pager::{Packets, PageHeader};

pub type OGGTags = (Option<VorbisComments>, PageHeader, Packets);

pub(crate) fn read_comments<R>(
	data: &mut R,
	mut len: u64,
	parse_options: ParseOptions,
) -> Result<VorbisComments, VorbisCommentsParseError>
where
	R: Read,
{
	use crate::macros::try_vec;

	let parse_mode = parse_options.parsing_mode;

	let vendor_len = data.read_u32::<LittleEndian>()?;
	if u64::from(vendor_len) > len {
		return Err(SizeMismatchError.into());
	}

	let mut vendor_bytes = try_vec![0; vendor_len as usize]?;
	data.read_exact(&mut vendor_bytes)?;

	len -= u64::from(vendor_len);

	let vendor;
	match utf8_decode(vendor_bytes) {
		Ok(v) => vendor = v,
		Err(e) => {
			// The actions following this are not spec-compliant in the slightest, so
			// we need to short circuit if strict.
			if parse_mode == ParsingMode::Strict {
				return Err(e.into());
			}

			log::warn!("Possibly corrupt vendor string, attempting to recover");

			let Some(utf8_err) = e.source().and_then(|e| e.downcast_ref::<FromUtf8Error>()) else {
				return Err(e.into());
			};
			let s = utf8_err
				.as_bytes()
				.iter()
				.map(|c| u16::from(*c))
				.collect::<Vec<_>>();

			match utf16_decode(&s) {
				Ok(v) => {
					log::warn!("Vendor string recovered as: '{v}'");
					vendor = v;
				},
				Err(e) => return Err(e.into()),
			}
		},
	}

	let number_of_items = data.read_u32::<LittleEndian>()?;
	if number_of_items > (len >> 2) as u32 {
		return Err(SizeMismatchError.into());
	}

	let mut tag = VorbisComments {
		vendor,
		items: Vec::with_capacity(number_of_items as usize),
		pictures: Vec::new(),
	};

	for _ in 0..number_of_items {
		let comment_len = data.read_u32::<LittleEndian>()?;
		if u64::from(comment_len) > len {
			return Err(SizeMismatchError.into());
		}

		let mut comment_bytes = try_vec![0; comment_len as usize]?;
		data.read_exact(&mut comment_bytes)?;

		len -= u64::from(comment_len);

		// KEY=VALUE
		let mut comment_split = comment_bytes.splitn(2, |b| *b == b'=');

		let Some(key) = comment_split.next() else {
			continue;
		};

		// Make sure there was a separator present, otherwise just move on
		let Some(value) = comment_split.next() else {
			log::warn!("No separator found in field, discarding");
			continue;
		};

		match key {
			k if k.eq_ignore_ascii_case(b"METADATA_BLOCK_PICTURE") => {
				if !parse_options.read_cover_art {
					continue;
				}

				match Picture::from_flac_bytes(value, true, parse_mode) {
					Ok(picture) => tag.pictures.push(picture),
					Err(e) => {
						if parse_mode == ParsingMode::Strict {
							return Err(e.into());
						}

						log::warn!("Failed to decode FLAC picture, discarding field");
						continue;
					},
				}
			},
			k if k.eq_ignore_ascii_case(b"COVERART") => {
				if !parse_options.read_cover_art {
					continue;
				}

				// `COVERART` is an old deprecated image storage format. We have to convert it
				// to a `METADATA_BLOCK_PICTURE` for it to be useful.
				//
				// <https://wiki.xiph.org/VorbisComment#Conversion_to_METADATA_BLOCK_PICTURE>
				log::warn!(
					"Found deprecated `COVERART` field, attempting to convert to \
					 `METADATA_BLOCK_PICTURE`"
				);

				let picture_data = BASE64.decode(value);

				match picture_data {
					Ok(picture_data) => {
						let mime_type = Picture::mimetype_from_bin(&picture_data)
							.unwrap_or_else(|_| MimeType::Unknown(String::from("image/")));

						let picture = Picture {
							pic_type: PictureType::Other,
							mime_type: Some(mime_type),
							description: None,
							data: Cow::from(picture_data),
						};

						tag.pictures.push((picture, PictureInformation::default()))
					},
					Err(e) => {
						if parse_mode == ParsingMode::Strict {
							return Err(PictureParseError::from(e).into());
						}

						log::warn!("Failed to decode FLAC picture, discarding field");
						continue;
					},
				}
			},
			// Support the case of TRACKNUMBER / DISCNUMBER being equal to current/total
			k if (k.eq_ignore_ascii_case(b"TRACKNUMBER")
				|| k.eq_ignore_ascii_case(b"DISCNUMBER")) =>
			{
				match utf8_decode_str(value) {
					Ok(value) => {
						let key = if k.eq_ignore_ascii_case(b"TRACKNUMBER") {
							String::from("TRACKNUMBER")
						} else {
							String::from("DISCNUMBER")
						};

						if !parse_options.implicit_conversions {
							tag.items.push((key, value.to_owned()));
							continue;
						}

						// try to parse as current/total
						let mut value_split = value.splitn(2, '/');
						let current: Option<u32> = value_split.next().and_then(|b| b.parse().ok());
						let total: Option<u32> = value_split.next().and_then(|b| b.parse().ok());

						match key.as_str() {
							"TRACKNUMBER" => {
								if let Some(n) = total {
									tag.set_track_total(n);
								}
								if let Some(n) = current {
									tag.set_track(n);
								} else {
									// Probably some other format, like a vinyl track number (A1, B1, etc.).
									// Just leave it up to the caller to deal with.
									tag.items.push((key, value.to_owned()));
								}
							},
							"DISCNUMBER" => {
								if let Some(n) = total {
									tag.set_disk_total(n);
								}
								if let Some(n) = current {
									tag.set_disk(n);
								} else {
									// Probably some other format, like a vinyl track number (A1, B1, etc.).
									// Just leave it up to the caller to deal with.
									tag.items.push((key, value.to_owned()));
								}
							},
							_ => {},
						}
					},
					Err(e) => {
						if parse_mode == ParsingMode::Strict {
							return Err(e.into());
						}

						log::warn!("Non UTF-8 value found, discarding field {key:?}");
						continue;
					},
				}
			},
			k if valid_vorbis_comments_key(k) => {
				// SAFETY: We just verified that all of the bytes fall within the subset of ASCII
				let key = unsafe { String::from_utf8_unchecked(k.to_vec()) };

				match utf8_decode_str(value) {
					Ok(value) => tag.items.push((key, value.to_owned())),
					Err(e) => {
						if parse_mode == ParsingMode::Strict {
							return Err(e.into());
						}

						log::warn!("Non UTF-8 value found, discarding field {key:?}");
						continue;
					},
				}
			},
			_ => {
				if parse_mode == ParsingMode::Strict {
					return Err(VorbisCommentsParseError::message(format!(
						"invalid item key: {}",
						key.escape_ascii()
					)));
				}

				// Otherwise just discard invalid keys...
			},
		}
	}

	Ok(tag)
}

pub(super) fn valid_vorbis_comments_key(key: &[u8]) -> bool {
	// The valid range is 0x20..=0x7D not including 0x3D
	key.iter().all(|c| (b' '..=b'}').contains(c) && *c != b'=')
}
