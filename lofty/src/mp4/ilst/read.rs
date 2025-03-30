use super::constants::WELL_KNOWN_TYPE_SET;
use super::data_type::DataType;
use super::{Atom, AtomData, AtomIdent, Ilst};
use crate::config::{ParseOptions, ParsingMode};
use crate::error::{LoftyError, Result};
use crate::id3::v1::constants::GENRES;
use crate::macros::{err, try_vec};
use crate::mp4::atom_info::{ATOM_HEADER_LEN, AtomInfo};
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::mp4::read::{AtomReader, skip_atom};
use crate::picture::{MimeType, Picture, PictureType};
use crate::tag::TagExt;
use crate::util::text::{utf8_decode, utf16_decode_bytes};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

pub(in crate::mp4) fn parse_ilst<R>(
	reader: &mut AtomReader<R>,
	parse_options: ParseOptions,
	len: u64,
) -> Result<Ilst>
where
	R: Read + Seek,
{
	let parsing_mode = parse_options.parsing_mode;

	let mut contents = try_vec![0; len as usize];
	reader.read_exact(&mut contents)?;

	let mut cursor = Cursor::new(contents);

	let mut ilst_reader = AtomReader::new(&mut cursor, parsing_mode)?;

	let mut tag = Ilst::default();

	let mut upgraded_gnres = Vec::new();
	while let Ok(Some(atom)) = ilst_reader.next() {
		if let AtomIdent::Fourcc(ref fourcc) = atom.ident {
			match fourcc {
				b"free" | b"skip" => {
					skip_atom(&mut ilst_reader, atom.extended, atom.len)?;
					continue;
				},
				b"covr" => {
					if parse_options.read_cover_art {
						handle_covr(&mut ilst_reader, parsing_mode, &mut tag, &atom)?;
					} else {
						skip_atom(&mut ilst_reader, atom.extended, atom.len)?;
					}

					continue;
				},
				// Upgrade this to a \xa9gen atom
				b"gnre" if parse_options.implicit_conversions => {
					log::warn!("Encountered outdated 'gnre' atom, attempting to upgrade to '©gen'");

					if let Some(atom_data) =
						parse_data_inner(&mut ilst_reader, parsing_mode, &atom)?
					{
						for (_, content) in atom_data {
							if content.len() >= 2 {
								let index = content[1] as usize;
								if index > 0 && index <= GENRES.len() {
									upgraded_gnres
										.push(AtomData::UTF8(String::from(GENRES[index - 1])));
								}
							}
						}
					}

					continue;
				},
				// Just insert these normally, the caller will deal with them (hopefully)
				b"gnre" => {
					log::warn!("Encountered outdated 'gnre' atom");

					if let Some(atom_data) =
						parse_data_inner(&mut ilst_reader, parsing_mode, &atom)?
					{
						let mut data = Vec::new();

						for (code, content) in atom_data {
							data.push(AtomData::Unknown {
								code,
								data: content,
							});
						}

						if let Some(storage) = AtomDataStorage::from_vec(data) {
							tag.atoms.push(Atom {
								ident: AtomIdent::Fourcc(*b"gnre"),
								data: storage,
							})
						}
					}

					continue;
				},
				// Special case the "Album ID", as it has the code "BE signed integer" (21), but
				// must be interpreted as a "BE 64-bit Signed Integer" (74)
				b"plID" => {
					if let Some(atom_data) =
						parse_data_inner(&mut ilst_reader, parsing_mode, &atom)?
					{
						let mut data = Vec::new();

						for (code, content) in atom_data {
							if content.len() == 8 {
								data.push(AtomData::Unknown {
									code,
									data: content,
								})
							}
						}

						if !data.is_empty() {
							let storage = match data.len() {
								1 => AtomDataStorage::Single(data.remove(0)),
								_ => AtomDataStorage::Multiple(data),
							};

							tag.atoms.push(Atom {
								ident: AtomIdent::Fourcc(*b"plID"),
								data: storage,
							})
						}
					}

					continue;
				},
				b"cpil" | b"hdvd" | b"pcst" | b"pgap" | b"shwm" => {
					if let Some(atom_data) =
						parse_data_inner(&mut ilst_reader, parsing_mode, &atom)?
					{
						if let Some((_, content)) = atom_data.first() {
							// Any size integer is technically valid, we'll correct it on write.
							let is_true = content.iter().any(|&b| b != 0);
							let data = AtomData::Bool(is_true);

							tag.atoms.push(Atom {
								ident: AtomIdent::Fourcc(*fourcc),
								data: AtomDataStorage::Single(data),
							})
						}
					}

					continue;
				},
				_ => {},
			}
		}

		parse_data(&mut ilst_reader, parsing_mode, &mut tag, atom)?;
	}

	if parse_options.implicit_conversions && !upgraded_gnres.is_empty() {
		if tag.contains(&AtomIdent::Fourcc(*b"\xa9gen")) {
			log::warn!("Encountered '©gen' atom, discarding upgraded 'gnre' atom(s)");
			return Ok(tag);
		}

		if let Some(storage) = AtomDataStorage::from_vec(upgraded_gnres) {
			tag.atoms.push(Atom {
				ident: AtomIdent::Fourcc(*b"\xa9gen"),
				data: storage,
			})
		}
	}

	Ok(tag)
}

fn parse_data<R>(
	reader: &mut AtomReader<R>,
	parsing_mode: ParsingMode,
	tag: &mut Ilst,
	atom_info: AtomInfo,
) -> Result<()>
where
	R: Read + Seek,
{
	let handle_error = |err: LoftyError, parsing_mode: ParsingMode| -> Result<()> {
		match parsing_mode {
			ParsingMode::Strict => Err(err),
			ParsingMode::BestAttempt | ParsingMode::Relaxed => {
				log::warn!("Skipping atom with invalid content: {}", err);
				Ok(())
			},
		}
	};

	if let Some(mut atom_data) = parse_data_inner(reader, parsing_mode, &atom_info)? {
		// Most atoms we encounter are only going to have 1 value, so store them as such
		if atom_data.len() == 1 {
			let (flags, content) = atom_data.remove(0);
			let data = match interpret_atom_content(flags, content) {
				Ok(data) => data,
				Err(err) => return handle_error(err, parsing_mode),
			};

			tag.atoms.push(Atom {
				ident: atom_info.ident,
				data: AtomDataStorage::Single(data),
			});

			return Ok(());
		}

		let mut data = Vec::new();
		for (flags, content) in atom_data {
			let value = match interpret_atom_content(flags, content) {
				Ok(data) => data,
				Err(err) => return handle_error(err, parsing_mode),
			};

			data.push(value);
		}

		tag.atoms.push(Atom {
			ident: atom_info.ident,
			data: AtomDataStorage::Multiple(data),
		});
	}

	Ok(())
}

const DATA_ATOM_IDENT: AtomIdent<'static> = AtomIdent::Fourcc(*b"data");

fn parse_data_inner<R>(
	reader: &mut AtomReader<R>,
	parsing_mode: ParsingMode,
	atom_info: &AtomInfo,
) -> Result<Option<Vec<(DataType, Vec<u8>)>>>
where
	R: Read + Seek,
{
	// An atom can contain multiple data atoms
	let mut ret = Vec::new();

	let atom_end = atom_info.start + atom_info.len;
	let position = reader.stream_position()?;
	assert!(
		atom_end >= position,
		"uncaught size mismatch, reader position: {position} (expected <= {atom_end})",
	);

	let to_read = atom_end - position;
	let mut pos = 0;
	while pos < to_read {
		let Some(next_atom) = reader.next()? else {
			break;
		};

		if next_atom.len < 16 {
			log::warn!(
				"Expected data atom to be at least 16 bytes, got {}. Stopping",
				next_atom.len
			);
			if parsing_mode == ParsingMode::Strict {
				err!(BadAtom("Data atom is too small"))
			}

			break;
		}

		if next_atom.ident != DATA_ATOM_IDENT {
			if parsing_mode == ParsingMode::Strict {
				err!(BadAtom("Expected atom \"data\" to follow name"))
			}

			log::warn!(
				"Skipping unexpected atom {actual_ident:?}, expected {expected_ident:?}",
				actual_ident = next_atom.ident,
				expected_ident = DATA_ATOM_IDENT
			);

			pos += next_atom.len;
			skip_atom(reader, next_atom.extended, next_atom.len)?;
			continue;
		}

		let Some(data_type) = parse_type_indicator(reader, parsing_mode)? else {
			log::warn!("Skipping atom with unknown type set");
			let remaining_atom_len = next_atom.len - (ATOM_HEADER_LEN + 1);

			reader.seek(SeekFrom::Current(remaining_atom_len as i64))?;
			pos += remaining_atom_len;
			continue;
		};

		// We don't care about the locale
		reader.seek(SeekFrom::Current(4))?;

		let content_len = (next_atom.len - 16) as usize;
		if content_len > 0 {
			let mut content = try_vec![0; content_len];
			reader.read_exact(&mut content)?;
			ret.push((data_type, content));
		} else {
			log::warn!("Skipping empty \"data\" atom");
		}

		pos += next_atom.len;
	}

	let ret = if ret.is_empty() { None } else { Some(ret) };
	Ok(ret)
}

fn parse_type_indicator<R>(
	reader: &mut AtomReader<R>,
	parsing_mode: ParsingMode,
) -> Result<Option<DataType>>
where
	R: Read + Seek,
{
	// The type indicator is formed of four bytes split between two fields. The first byte indicates
	// the set of types from which the type is drawn. The second through fourth byte forms the second
	// field and its interpretation depends upon the value in the first field.

	let type_set = reader.read_u8()?;
	if type_set != WELL_KNOWN_TYPE_SET {
		if parsing_mode == ParsingMode::Strict {
			err!(BadAtom("Unknown type set in data atom"))
		}

		return Ok(None);
	}

	Ok(Some(DataType::from(reader.read_u24()?)))
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

fn handle_covr<R>(
	reader: &mut AtomReader<R>,
	parsing_mode: ParsingMode,
	tag: &mut Ilst,
	atom_info: &AtomInfo,
) -> Result<()>
where
	R: Read + Seek,
{
	if let Some(atom_data) = parse_data_inner(reader, parsing_mode, atom_info)? {
		let mut data = Vec::new();

		let len = atom_data.len();
		for (data_type, value) in atom_data {
			let mime_type = match data_type {
				// Type 0 is implicit
				DataType::Reserved => None,
				// GIF is deprecated
				DataType::Gif => Some(MimeType::Gif),
				DataType::Jpeg => Some(MimeType::Jpeg),
				DataType::Png => Some(MimeType::Png),
				DataType::Bmp => Some(MimeType::Bmp),
				_ => {
					if parsing_mode == ParsingMode::Strict {
						err!(BadAtom("\"covr\" atom has an unknown type"))
					}

					log::warn!(
						"Encountered \"covr\" atom with an unknown type of `{}`, discarding",
						Into::<u32>::into(data_type)
					);
					return Ok(());
				},
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

fn interpret_atom_content(flags: DataType, content: Vec<u8>) -> Result<AtomData> {
	// https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW35
	Ok(match flags {
		DataType::Utf8 => AtomData::UTF8(utf8_decode(content)?),
		DataType::Utf16 => AtomData::UTF16(utf16_decode_bytes(&content, u16::from_be_bytes)?),
		DataType::BeSignedInteger => AtomData::SignedInteger(parse_int(&content)?),
		DataType::BeUnsignedInteger => AtomData::UnsignedInteger(parse_uint(&content)?),
		code => AtomData::Unknown {
			code,
			data: content,
		},
	})
}
