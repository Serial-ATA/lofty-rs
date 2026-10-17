mod atom_reader;

use super::Mp4File;
use super::atom_info::{AtomIdent, AtomInfo};
use super::moov::Moov;
use super::properties::Mp4Properties;
use crate::config::{ParseOptions, ParsingMode};
use crate::error::{NotEnoughDataError, SizeMismatchError, TooMuchDataError, UnknownFormatError};
use crate::mp4::error::{AtomParseError, Mp4ParseError};
use crate::util::io::SeekStreamLen;
use crate::util::text::utf8_decode_str;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(super) use atom_reader::AtomReader;

pub(in crate::mp4) fn verify_mp4<R>(reader: &mut AtomReader<R>) -> Result<String, Mp4ParseError>
where
	R: Read + Seek,
{
	let Some(atom) = reader.next()? else {
		return Err(UnknownFormatError.into());
	};

	if atom.ident != AtomIdent::Fourcc(*b"ftyp") {
		return Err(UnknownFormatError.into());
	}

	// size + identifier + major brand
	// There *should* be more, but this is all we need from it
	if atom.len < 12 {
		return Err(NotEnoughDataError::new(Some(12)).into());
	}

	let mut major_brand = [0u8; 4];
	reader.read_exact(&mut major_brand)?;

	reader.seek(SeekFrom::Current((atom.len - 12) as i64))?;

	let major_brand = utf8_decode_str(&major_brand)
		.map(ToOwned::to_owned)
		.map_err(|_| Mp4ParseError::message("malformed ftyp major brand"))?;

	log::debug!("Verified to be an MP4 file. Major brand: {}", major_brand);
	Ok(major_brand)
}

pub(crate) fn read_from<R>(
	data: &mut R,
	parse_options: ParseOptions,
) -> Result<Mp4File, Mp4ParseError>
where
	R: Read + Seek,
{
	let mut reader = AtomReader::new(data, parse_options.parsing_mode)?;
	let file_length = reader.stream_len_hack()?;

	let ftyp = verify_mp4(&mut reader)?;

	// Find the `moov` atom and restrict the reader to its length
	let moov_info = Moov::find(&mut reader)?;
	reader.reset_bounds(moov_info.start + 8, moov_info.len - 8);

	let moov = Moov::parse(&mut reader, parse_options)?;

	let mut properties = if parse_options.read_properties {
		// Remove the length restriction
		reader.reset_bounds(0, file_length);
		super::properties::read_properties(
			&mut reader,
			&moov.traks,
			file_length,
			parse_options.parsing_mode,
		)?
	} else {
		Mp4Properties::default()
	};
	properties.ftyp = ftyp;

	Ok(Mp4File {
		ilst_tag: moov.ilst,
		properties,
	})
}

/// Seeks the reader to the end of the atom
///
/// This should be used immediately after [`AtomInfo::read`] to skip an unwanted atom.
///
/// NOTES:
///
/// * This makes the assumption that the reader is at the end of the atom's header.
/// * This makes the assumption that the `len` is the *full atom length*, not just that of the content.
pub(super) fn skip_atom<R>(reader: &mut R, extended: bool, len: u64) -> Result<(), AtomParseError>
where
	R: Read + Seek,
{
	// The reader is already past the header: 8 bytes, or 16 with a 64-bit size
	let header_len = if extended { 16 } else { 8 };
	let Some(skip_len) = len.checked_sub(header_len) else {
		return Err(SizeMismatchError.into());
	};
	log::trace!("Attempting to skip {} bytes", skip_len);

	if !extended {
		reader.seek(SeekFrom::Current(i64::from(len as u32) - 8))?;
		return Ok(());
	}

	let pos = reader.stream_position()?;

	let Some(pos) = pos.checked_add(skip_len) else {
		return Err(TooMuchDataError.into());
	};

	reader.seek(SeekFrom::Start(pos))?;

	Ok(())
}

/// Finds the first child atom with the given fourcc
///
/// * `len` is the length of the parent atom
/// * `expected` is the fourcc of the child atom to find
pub(super) fn find_child_atom<R>(
	reader: &mut R,
	mut len: u64,
	expected: [u8; 4],
	parse_mode: ParsingMode,
) -> Result<Option<AtomInfo>, AtomParseError>
where
	R: Read + Seek,
{
	let mut ret = None;

	while len > 8 {
		let Some(atom) = AtomInfo::read(reader, len, parse_mode)? else {
			break;
		};

		match atom.ident {
			AtomIdent::Fourcc(fourcc) if fourcc == expected => {
				ret = Some(atom);
				break;
			},
			_ => {
				skip_atom(reader, atom.extended, atom.len)?;
				len = len.saturating_sub(atom.len);
			},
		}
	}

	Ok(ret)
}

// Creates a tree of nested atoms
pub(super) fn atom_tree<R>(
	reader: &mut R,
	mut len: u64,
	up_to: &[u8],
	parse_mode: ParsingMode,
) -> Result<(Option<usize>, Vec<AtomInfo>), Mp4ParseError>
where
	R: Read + Seek,
{
	let mut found_idx = None;
	let mut buf = Vec::new();

	let mut i = 0;

	while len > 8 {
		let Some(atom) = AtomInfo::read(reader, len, parse_mode)? else {
			break;
		};

		skip_atom(reader, atom.extended, atom.len)?;
		len = len.saturating_sub(atom.len);

		if let AtomIdent::Fourcc(ref fourcc) = atom.ident {
			if fourcc == up_to && found_idx.is_none() {
				found_idx = Some(i);
			}

			buf.push(atom);
			i += 1;
		}
	}

	Ok((found_idx, buf))
}

/// Check if the `meta` atom is "full"
///
/// A full `meta` atom should have the following:
///
/// * Size (4)
/// * Ident (4) ("meta")
/// * Version (1)
/// * Flags (3)
///
/// However, some encoders write it as a normal atom:
///
/// * Size (4)
/// * Ident (4) ("meta")
pub(super) fn meta_is_full<R>(reader: &mut R) -> Result<bool, AtomParseError>
where
	R: Read + Seek,
{
	// If this is a normal atom, this'll be the size of the next atom
	let _version_flags = reader.read_u32::<BigEndian>()?;

	// Check if the next four bytes is one of the nested `meta` atoms
	let mut possible_ident = [0; 4];
	reader.read_exact(&mut possible_ident)?;

	match &possible_ident {
		b"hdlr" | b"ilst" | b"mhdr" | b"ctry" | b"lang" => {
			log::warn!("File contains a non-full 'meta' atom");

			reader.seek(SeekFrom::Current(-8))?;
			Ok(false)
		},
		_ => {
			reader.seek(SeekFrom::Current(-4))?;
			Ok(true)
		},
	}
}

#[cfg(test)]
mod tests {
	use super::read_from;
	use crate::config::ParseOptions;
	use crate::tag::Accessor;

	use std::io::Cursor;

	fn atom(ident: [u8; 4], data: &[u8]) -> Vec<u8> {
		let mut v = ((data.len() + 8) as u32).to_be_bytes().to_vec();
		v.extend(ident);
		v.extend(data);
		v
	}

	// An atom with a 64-bit size field, as written for any `mdat` over 4 GiB
	fn extended_atom(ident: [u8; 4], data: &[u8]) -> Vec<u8> {
		let mut v = 1u32.to_be_bytes().to_vec();
		v.extend(ident);
		v.extend(((data.len() + 16) as u64).to_be_bytes());
		v.extend(data);
		v
	}

	fn moov_with_title() -> Vec<u8> {
		let mut meta = vec![0; 4]; // version + flags
		meta.extend(atom(
			*b"hdlr",
			&[&[0; 8][..], b"mdirappl", &[0; 9]].concat(),
		));
		meta.extend(atom(
			*b"ilst",
			&atom(*b"\xA9nam", &atom(*b"data", b"\0\0\0\x01\0\0\0\0Title")),
		));
		atom(*b"moov", &atom(*b"udta", &atom(*b"meta", &meta)))
	}

	fn read_title(file: Vec<u8>) -> Option<String> {
		let options = ParseOptions::new().read_properties(false);
		let mp4 = read_from(&mut Cursor::new(file), options).unwrap();
		mp4.ilst_tag.and_then(|t| t.title().map(|t| t.to_string()))
	}

	#[test_log::test]
	fn extended_atom_before_moov_is_skipped_correctly() {
		let mut file = atom(*b"ftyp", b"isom\0\0\x02\0isomiso2mp41");
		file.extend(extended_atom(*b"mdat", &[0; 8]));
		file.extend(moov_with_title());

		assert_eq!(read_title(file).as_deref(), Some("Title"));
	}
}
