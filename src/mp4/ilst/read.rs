use super::constants::{
	BE_64BIT_SIGNED_INTEGER, BE_SIGNED_INTEGER, BE_UNSIGNED_INTEGER, BMP, JPEG, PNG, RESERVED,
	UTF16, UTF8,
};
use super::{Atom, AtomData, AtomIdent, Ilst};
use crate::error::Result;
use crate::id3::v1::constants::GENRES;
use crate::id3::v2::util::text_utils::utf16_decode;
use crate::macros::{err, try_vec};
use crate::mp4::atom_info::AtomInfo;
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::mp4::read::{skip_unneeded, AtomReader};
use crate::picture::{MimeType, Picture, PictureType};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

pub(in crate::mp4) fn parse_ilst<R>(reader: &mut AtomReader<R>, len: u64) -> Result<Ilst>
where
	R: Read + Seek,
{
	let mut contents = try_vec![0; len as usize];
	reader.read_exact(&mut contents)?;

	let mut cursor = Cursor::new(contents);

	let mut ilst_reader = AtomReader::new(&mut cursor)?;

	let mut tag = Ilst::default();

	while let Ok(atom) = ilst_reader.next() {
		if let AtomIdent::Fourcc(ref fourcc) = atom.ident {
			match fourcc {
				b"free" | b"skip" => {
					skip_unneeded(&mut ilst_reader, atom.extended, atom.len)?;
					continue;
				},
				b"covr" => {
					handle_covr(&mut ilst_reader, &mut tag, &atom)?;
					continue;
				},
				// Upgrade this to a \xa9gen atom
				b"gnre" => {
					if let Some(atom_data) = parse_data_inner(&mut ilst_reader, &atom)? {
						let mut data = Vec::new();

						for (flags, content) in atom_data {
							if (flags == BE_SIGNED_INTEGER || flags == 0) && content.len() >= 2 {
								let index = content[1] as usize;
								if index > 0 && index <= GENRES.len() {
									data.push(AtomData::UTF8(String::from(GENRES[index - 1])));
								}
							}
						}

						tag.atoms.push(Atom {
							ident: AtomIdent::Fourcc(*b"\xa9gen"),
							data: AtomDataStorage::Multiple(data),
						})
					}

					continue;
				},
				// Special case the "Album ID", as it has the code "BE signed integer" (21), but
				// must be interpreted as a "BE 64-bit Signed Integer" (74)
				b"plID" => {
					if let Some(atom_data) = parse_data_inner(&mut ilst_reader, &atom)? {
						let mut data = Vec::new();

						for (code, content) in atom_data {
							if (code == BE_SIGNED_INTEGER || code == BE_64BIT_SIGNED_INTEGER)
								&& content.len() == 8
							{
								data.push(AtomData::Unknown {
									code,
									data: content,
								})
							}
						}

						tag.atoms.push(Atom {
							ident: AtomIdent::Fourcc(*b"plID"),
							data: AtomDataStorage::Multiple(data),
						})
					}

					continue;
				},
				_ => {},
			}
		}

		parse_data(&mut ilst_reader, &mut tag, atom)?;
	}

	Ok(tag)
}

fn parse_data<R>(reader: &mut AtomReader<R>, tag: &mut Ilst, atom_info: AtomInfo) -> Result<()>
where
	R: Read + Seek,
{
	if let Some(mut atom_data) = parse_data_inner(reader, &atom_info)? {
		// Most atoms we encounter are only going to have 1 value, so store them as such
		if atom_data.len() == 1 {
			let (flags, content) = atom_data.remove(0);
			let data = interpret_atom_content(flags, content)?;

			tag.atoms.push(Atom {
				ident: atom_info.ident,
				data: AtomDataStorage::Single(data),
			});

			return Ok(());
		}

		let mut data = Vec::new();
		for (flags, content) in atom_data {
			let value = interpret_atom_content(flags, content)?;
			data.push(value);
		}

		tag.atoms.push(Atom {
			ident: atom_info.ident,
			data: AtomDataStorage::Multiple(data),
		});
	}

	Ok(())
}

fn parse_data_inner<R>(
	reader: &mut AtomReader<R>,
	atom_info: &AtomInfo,
) -> Result<Option<Vec<(u32, Vec<u8>)>>>
where
	R: Read + Seek,
{
	// An atom can contain multiple data atoms
	let mut ret = Vec::new();

	let to_read = (atom_info.start + atom_info.len) - reader.position()?;
	let mut pos = 0;
	while pos < to_read {
		let data_atom = reader.next()?;
		match data_atom.ident {
			AtomIdent::Fourcc(ref name) if name == b"data" => {},
			_ => err!(BadAtom("Expected atom \"data\" to follow name")),
		}

		// We don't care about the version
		let _version = reader.read_u8()?;

		let mut flags = [0; 3];
		reader.read_exact(&mut flags)?;

		let flags = u32::from_be_bytes([0, flags[0], flags[1], flags[2]]);

		// We don't care about the locale
		reader.seek(SeekFrom::Current(4))?;

		let content_len = (data_atom.len - 16) as usize;
		if content_len == 0 {
			// We won't add empty atoms
			return Ok(None);
		}

		let mut content = try_vec![0; content_len];
		reader.read_exact(&mut content)?;

		pos += data_atom.len;
		ret.push((flags, content));
	}

	let ret = if ret.is_empty() { None } else { Some(ret) };
	Ok(ret)
}

fn parse_uint(bytes: &[u8]) -> Result<u32> {
	Ok(match bytes.len() {
		1 => u32::from(bytes[0]),
		2 => u32::from(u16::from_be_bytes([bytes[0], bytes[1]])),
		3 => u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
		4 => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
		_ => err!(BadAtom(
			"Unexpected atom size for type \"BE unsigned integer\""
		)),
	})
}

fn parse_int(bytes: &[u8]) -> Result<i32> {
	Ok(match bytes.len() {
		1 => i32::from(bytes[0]),
		2 => i32::from(i16::from_be_bytes([bytes[0], bytes[1]])),
		3 => i32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
		4 => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
		_ => err!(BadAtom(
			"Unexpected atom size for type \"BE signed integer\""
		)),
	})
}

fn handle_covr<R>(reader: &mut AtomReader<R>, tag: &mut Ilst, atom_info: &AtomInfo) -> Result<()>
where
	R: Read + Seek,
{
	if let Some(atom_data) = parse_data_inner(reader, atom_info)? {
		let mut data = Vec::new();

		let len = atom_data.len();
		for (flags, value) in atom_data {
			let mime_type = match flags {
				// Type 0 is implicit
				RESERVED => MimeType::None,
				// GIF is deprecated
				12 => MimeType::Gif,
				JPEG => MimeType::Jpeg,
				PNG => MimeType::Png,
				BMP => MimeType::Bmp,
				_ => err!(BadAtom("\"covr\" atom has an unknown type")),
			};

			let picture_data = AtomData::Picture(Picture {
				pic_type: PictureType::Other,
				mime_type,
				description: None,
				data: Cow::from(value),
			});

			if len == 1 {
				tag.atoms.push(Atom {
					ident: AtomIdent::Fourcc(*b"covr"),
					data: AtomDataStorage::Single(picture_data),
				});

				return Ok(());
			}

			data.push(picture_data);
		}

		tag.atoms.push(Atom {
			ident: AtomIdent::Fourcc(*b"covr"),
			data: AtomDataStorage::Multiple(data),
		});
	}

	Ok(())
}

fn interpret_atom_content(flags: u32, content: Vec<u8>) -> Result<AtomData> {
	// https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW35
	Ok(match flags {
		UTF8 => AtomData::UTF8(String::from_utf8(content)?),
		UTF16 => AtomData::UTF16(utf16_decode(&content, u16::from_be_bytes)?),
		BE_SIGNED_INTEGER => AtomData::SignedInteger(parse_int(&content)?),
		BE_UNSIGNED_INTEGER => AtomData::UnsignedInteger(parse_uint(&content)?),
		code => AtomData::Unknown {
			code,
			data: content,
		},
	})
}
