#[cfg(feature = "vorbis_comments")]
use super::tag::VorbisComments;
use super::verify_signature;
use crate::error::Result;
use crate::macros::err;
#[cfg(feature = "vorbis_comments")]
use crate::picture::Picture;

use std::io::{Read, Seek, SeekFrom};

#[cfg(feature = "vorbis_comments")]
use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

#[cfg(feature = "vorbis_comments")]
pub type OGGTags = (Option<VorbisComments>, Page);

#[cfg(not(feature = "vorbis_comments"))]
pub type OGGTags = (Option<()>, Page);

#[cfg(feature = "vorbis_comments")]
pub(crate) fn read_comments<R>(data: &mut R, mut len: u64, tag: &mut VorbisComments) -> Result<()>
where
	R: Read,
{
	use crate::error::FileDecodingError;
	use crate::macros::try_vec;

	let vendor_len = data.read_u32::<LittleEndian>()?;
	if u64::from(vendor_len) > len {
		err!(TooMuchData);
	}

	let mut vendor = try_vec![0; vendor_len as usize];
	data.read_exact(&mut vendor)?;

	len -= u64::from(vendor_len);

	let vendor = match String::from_utf8(vendor) {
		Ok(v) => v,
		Err(e) => {
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
				Err(_) => {
					return Err(FileDecodingError::from_description(
						"OGG: File has an invalid vendor string",
					)
					.into())
				},
			}
		},
	};

	tag.vendor = vendor;

	let comments_total_len = data.read_u32::<LittleEndian>()?;

	for _ in 0..comments_total_len {
		let comment_len = data.read_u32::<LittleEndian>()?;
		if u64::from(comment_len) > len {
			// TODO: Maybe add ErrorKind::SizeMismatch?
			err!(TooMuchData);
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
				"METADATA_BLOCK_PICTURE" => tag
					.pictures
					.push(Picture::from_flac_bytes(value.as_bytes(), true)?),
				// The valid range is 0x20..=0x7D not including 0x3D
				k if k.chars().all(|c| (' '..='}').contains(&c) && c != '=') => {
					tag.items.push((k.to_string(), value.to_string()))
				},
				_ => {}, // Discard invalid keys
			}
		}
	}

	Ok(())
}

pub(crate) fn read_from<T>(data: &mut T, header_sig: &[u8], comment_sig: &[u8]) -> Result<OGGTags>
where
	T: Read + Seek,
{
	let first_page = Page::read(data, false)?;
	verify_signature(&first_page, header_sig)?;

	let md_page = Page::read(data, false)?;
	verify_signature(&md_page, comment_sig)?;

	let mut md_pages: Vec<u8> = Vec::new();

	md_pages.extend_from_slice(&md_page.content()[comment_sig.len()..]);

	while let Ok(page) = Page::read(data, false) {
		if md_pages.len() > 125_829_120 {
			err!(TooMuchData);
		}

		if page.header_type() & 0x01 == 1 {
			md_pages.extend_from_slice(page.content());
		} else {
			data.seek(SeekFrom::Start(page.start))?;
			break;
		}
	}

	#[cfg(feature = "vorbis_comments")]
	{
		let mut tag = VorbisComments::default();

		let reader = &mut &md_pages[..];
		read_comments(reader, reader.len() as u64, &mut tag)?;

		Ok((Some(tag), first_page))
	}

	#[cfg(not(feature = "vorbis_comments"))]
	Ok((None, first_page))
}
