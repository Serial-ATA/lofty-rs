use super::constants::{
	BE_64BIT_SIGNED_INTEGER, BE_SIGNED_INTEGER, BE_UNSIGNED_INTEGER, BMP, JPEG, PNG, RESERVED,
	UTF16, UTF8,
};
use super::{Atom, AtomData, AtomIdent, Ilst};
use crate::error::{ErrorKind, LoftyError, Result};
use crate::id3::v1::constants::GENRES;
use crate::id3::v2::util::text_utils::utf16_decode;
use crate::macros::try_vec;
use crate::mp4::atom_info::AtomInfo;
use crate::mp4::read::skip_unneeded;
use crate::picture::{MimeType, Picture, PictureType};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::ReadBytesExt;

pub(in crate::mp4) fn parse_ilst<R>(reader: &mut R, len: u64) -> Result<Ilst>
where
	R: Read,
{
	let mut contents = try_vec![0; len as usize];
	reader.read_exact(&mut contents)?;

	let mut cursor = Cursor::new(contents);

	let mut tag = Ilst::default();

	while let Ok(atom) = AtomInfo::read(&mut cursor) {
		let ident = match atom.ident {
			AtomIdent::Fourcc(ref fourcc) => match fourcc {
				b"free" | b"skip" => {
					skip_unneeded(&mut cursor, atom.extended, atom.len)?;
					continue;
				},
				b"covr" => {
					handle_covr(&mut cursor, &mut tag)?;
					continue;
				},
				// Upgrade this to a \xa9gen atom
				b"gnre" => {
					let content = parse_data(&mut cursor)?;

					if let Some(AtomData::Unknown {
						code: BE_UNSIGNED_INTEGER | 0,
						data,
					}) = content
					{
						if data.len() >= 2 {
							let index = data[1] as usize;

							if index > 0 && index <= GENRES.len() {
								tag.atoms.push(Atom {
									ident: AtomIdent::Fourcc(*b"\xa9gen"),
									data: AtomData::UTF8(String::from(GENRES[index - 1])),
								})
							}
						}
					}

					continue;
				},
				// Special case the "Album ID", as it has the code "BE signed integer" (21), but
				// must be interpreted as a "BE 64-bit Signed Integer" (74)
				b"plID" => {
					if let Some((code, content)) = parse_data_inner(&mut cursor)? {
						if (code == BE_SIGNED_INTEGER || code == BE_64BIT_SIGNED_INTEGER)
							&& content.len() == 8
						{
							tag.atoms.push(Atom {
								ident: AtomIdent::Fourcc(*b"plID"),
								data: AtomData::Unknown {
									code,
									data: content,
								},
							})
						}
					}

					continue;
				},
				_ => atom.ident,
			},
			ident => ident,
		};

		if let Some(data) = parse_data(&mut cursor)? {
			tag.atoms.push(Atom { ident, data })
		}
	}

	Ok(tag)
}

fn parse_data<R>(data: &mut R) -> Result<Option<AtomData>>
where
	R: Read + Seek,
{
	if let Some((flags, content)) = parse_data_inner(data)? {
		// https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW35
		let value = match flags {
			UTF8 => AtomData::UTF8(String::from_utf8(content)?),
			UTF16 => AtomData::UTF16(utf16_decode(&*content, u16::from_be_bytes)?),
			BE_SIGNED_INTEGER => AtomData::SignedInteger(parse_int(&content)?),
			BE_UNSIGNED_INTEGER => AtomData::UnsignedInteger(parse_uint(&content)?),
			code => AtomData::Unknown {
				code,
				data: content,
			},
		};

		return Ok(Some(value));
	}

	Ok(None)
}

fn parse_data_inner<R>(data: &mut R) -> Result<Option<(u32, Vec<u8>)>>
where
	R: Read + Seek,
{
	let atom = AtomInfo::read(data)?;

	match atom.ident {
		AtomIdent::Fourcc(ref name) if name == b"data" => {},
		_ => {
			return Err(LoftyError::new(ErrorKind::BadAtom(
				"Expected atom \"data\" to follow name",
			)))
		},
	}

	// We don't care about the version
	let _version = data.read_u8()?;

	let mut flags = [0; 3];
	data.read_exact(&mut flags)?;

	let flags = u32::from_be_bytes([0, flags[0], flags[1], flags[2]]);

	// We don't care about the locale
	data.seek(SeekFrom::Current(4))?;

	let content_len = (atom.len - 16) as usize;
	if content_len == 0 {
		// We won't add empty atoms
		return Ok(None);
	}

	let mut content = try_vec![0; content_len];
	data.read_exact(&mut content)?;

	Ok(Some((flags, content)))
}

fn parse_uint(bytes: &[u8]) -> Result<u32> {
	Ok(match bytes.len() {
		1 => u32::from(bytes[0]),
		2 => u32::from(u16::from_be_bytes([bytes[0], bytes[1]])),
		3 => u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
		4 => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
		_ => {
			return Err(LoftyError::new(ErrorKind::BadAtom(
				"Unexpected atom size for type \"BE unsigned integer\"",
			)))
		},
	})
}

fn parse_int(bytes: &[u8]) -> Result<i32> {
	Ok(match bytes.len() {
		1 => i32::from(bytes[0]),
		2 => i32::from(i16::from_be_bytes([bytes[0], bytes[1]])),
		3 => i32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
		4 => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
		_ => {
			return Err(LoftyError::new(ErrorKind::BadAtom(
				"Unexpected atom size for type \"BE signed integer\"",
			)))
		},
	})
}

fn handle_covr(reader: &mut Cursor<Vec<u8>>, tag: &mut Ilst) -> Result<()> {
	if let Some(value) = parse_data(reader)? {
		let (mime_type, data) = match value {
			AtomData::Unknown { code, data } => match code {
				// Type 0 is implicit
				RESERVED => (MimeType::None, data),
				// GIF is deprecated
				12 => (MimeType::Gif, data),
				JPEG => (MimeType::Jpeg, data),
				PNG => (MimeType::Png, data),
				BMP => (MimeType::Bmp, data),
				_ => {
					return Err(LoftyError::new(ErrorKind::BadAtom(
						"\"covr\" atom has an unknown type",
					)))
				},
			},
			_ => {
				return Err(LoftyError::new(ErrorKind::BadAtom(
					"\"covr\" atom has an unknown type",
				)))
			},
		};

		tag.atoms.push(Atom {
			ident: AtomIdent::Fourcc(*b"covr"),
			data: AtomData::Picture(Picture {
				pic_type: PictureType::Other,
				mime_type,
				description: None,
				data: Cow::from(data),
			}),
		});
	}

	Ok(())
}
