use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{err, try_vec};
use crate::tag::{ItemKey, TagType};
use crate::util::text::utf8_decode;

use std::borrow::Cow;
use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(super) const FOURCC_LEN: u64 = 4;
pub(super) const IDENTIFIER_LEN: u64 = 4;
pub(super) const ATOM_HEADER_LEN: u64 = FOURCC_LEN + IDENTIFIER_LEN;

/// Represents an `MP4` atom identifier
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum AtomIdent<'a> {
	/// A four byte identifier
	///
	/// Many FOURCCs start with `0xA9` (©), and should be human-readable.
	Fourcc([u8; 4]),
	/// A freeform identifier
	///
	/// # Example
	///
	/// ```text
	/// ----:com.apple.iTunes:SUBTITLE
	/// ─┬── ────────┬─────── ───┬────
	///  ╰freeform identifier    ╰name
	///              |
	///              ╰mean
	/// ```
	Freeform {
		/// A string using a reverse DNS naming convention
		mean: Cow<'a, str>,
		/// A string identifying the atom
		name: Cow<'a, str>,
	},
}

impl<'a> AtomIdent<'a> {
	/// Obtains a borrowed instance
	pub fn as_borrowed(&'a self) -> Self {
		match self {
			Self::Fourcc(fourcc) => Self::Fourcc(*fourcc),
			Self::Freeform { mean, name } => Self::Freeform {
				mean: Cow::Borrowed(mean),
				name: Cow::Borrowed(name),
			},
		}
	}

	/// Obtains an owned instance
	pub fn into_owned(self) -> AtomIdent<'static> {
		match self {
			Self::Fourcc(fourcc) => AtomIdent::Fourcc(fourcc),
			Self::Freeform { mean, name } => AtomIdent::Freeform {
				mean: Cow::Owned(mean.into_owned()),
				name: Cow::Owned(name.into_owned()),
			},
		}
	}
}

impl<'a> TryFrom<&'a ItemKey> for AtomIdent<'a> {
	type Error = LoftyError;

	fn try_from(value: &'a ItemKey) -> std::result::Result<Self, Self::Error> {
		if let Some(mapped_key) = value.map_key(TagType::Mp4Ilst) {
			if mapped_key.starts_with("----") {
				let mut split = mapped_key.split(':');

				split.next();

				let mean = split.next();
				let name = split.next();

				if let (Some(mean), Some(name)) = (mean, name) {
					return Ok(AtomIdent::Freeform {
						mean: Cow::Borrowed(mean),
						name: Cow::Borrowed(name),
					});
				}
			} else {
				let fourcc = mapped_key.chars().map(|c| c as u8).collect::<Vec<_>>();

				if let Ok(fourcc) = TryInto::<[u8; 4]>::try_into(fourcc) {
					return Ok(AtomIdent::Fourcc(fourcc));
				}
			}
		}

		err!(TextDecode(
			"ItemKey does not map to a freeform or fourcc identifier"
		))
	}
}

impl TryFrom<ItemKey> for AtomIdent<'static> {
	type Error = LoftyError;

	fn try_from(value: ItemKey) -> std::result::Result<Self, Self::Error> {
		let ret: AtomIdent<'_> = (&value).try_into()?;
		Ok(ret.into_owned())
	}
}

#[derive(Debug)]
pub(crate) struct AtomInfo {
	pub(crate) start: u64,
	pub(crate) len: u64,
	pub(crate) extended: bool,
	pub(crate) ident: AtomIdent<'static>,
}

// The spec permits any characters to be used in atom identifiers. This doesn't
// leave us any room for error detection.
//
// TagLib has decided on a character set to consider valid, so we will do the same:
// <https://github.com/taglib/taglib/issues/1077#issuecomment-1440385838>
fn is_valid_identifier_byte(b: u8) -> bool {
	(b' '..=b'~').contains(&b) || b == b'\xA9'
}

impl AtomInfo {
	pub(crate) fn read<R>(
		data: &mut R,
		mut reader_size: u64,
		parse_mode: ParsingMode,
	) -> Result<Option<Self>>
	where
		R: Read + Seek,
	{
		let start = data.stream_position()?;

		let len_raw = u64::from(data.read_u32::<BigEndian>()?);

		let mut identifier = [0; IDENTIFIER_LEN as usize];
		data.read_exact(&mut identifier)?;

		if !identifier.iter().copied().all(is_valid_identifier_byte) {
			// The atom identifier contains invalid characters
			//
			// Seek to the end, since we can't recover from this
			data.seek(SeekFrom::End(0))?;

			match parse_mode {
				ParsingMode::Strict => {
					err!(BadAtom("Encountered an atom with invalid characters"));
				},
				ParsingMode::BestAttempt | ParsingMode::Relaxed => {
					log::warn!("Encountered an atom with invalid characters, stopping");
					return Ok(None);
				},
			}
		}

		let (len, extended) = match len_raw {
			// The atom extends to the end of the file
			0 => {
				let pos = data.stream_position()?;
				let end = data.seek(SeekFrom::End(0))?;

				data.seek(SeekFrom::Start(pos))?;

				(end - pos, false)
			},
			// There's an extended length
			1 => (data.read_u64::<BigEndian>()?, true),
			_ => (len_raw, false),
		};

		if len < ATOM_HEADER_LEN {
			// Seek to the end, since we can't recover from this
			data.seek(SeekFrom::End(0))?;

			err!(BadAtom("Found an invalid length (< 8)"));
		}

		// `len` includes itself and the identifier
		if (len - ATOM_HEADER_LEN) > reader_size {
			log::warn!("Encountered an atom with an invalid length, stopping");

			// As with all formats, there's a good chance certain software won't know how to actually use padding.
			// If the file ends with an incorrectly sized padding atom, we can just ignore it.
			let skippable = (parse_mode != ParsingMode::Strict && identifier == *b"free")
				|| parse_mode == ParsingMode::Relaxed;
			if skippable {
				// Seek to the end, as we cannot gather anything else from the file
				data.seek(SeekFrom::End(0))?;
				return Ok(None);
			}

			err!(SizeMismatch);
		}

		let atom_ident;
		if identifier == *b"----" {
			// Encountered a freeform identifier
			reader_size -= ATOM_HEADER_LEN;
			if reader_size < ATOM_HEADER_LEN {
				err!(BadAtom("Found an incomplete freeform identifier"));
			}

			atom_ident = parse_freeform(data, len - ATOM_HEADER_LEN, parse_mode)?;
		} else {
			atom_ident = AtomIdent::Fourcc(identifier);
		}

		Ok(Some(Self {
			start,
			len,
			extended,
			ident: atom_ident,
		}))
	}

	pub(crate) fn header_size(&self) -> u64 {
		if !self.extended {
			return ATOM_HEADER_LEN;
		}

		ATOM_HEADER_LEN + 8
	}
}

fn parse_freeform<R>(
	data: &mut R,
	atom_len: u64,
	parse_mode: ParsingMode,
) -> Result<AtomIdent<'static>>
where
	R: Read + Seek,
{
	// ---- header + mean header + name header = 24
	const MINIMUM_FREEFORM_LEN: u64 = ATOM_HEADER_LEN * 3;

	if atom_len < MINIMUM_FREEFORM_LEN {
		err!(BadAtom("Found an incomplete freeform identifier"));
	}

	let mut atom_len = atom_len;
	let mean = freeform_chunk(data, b"mean", &mut atom_len, parse_mode)?;
	let name = freeform_chunk(data, b"name", &mut atom_len, parse_mode)?;

	Ok(AtomIdent::Freeform {
		mean: mean.into(),
		name: name.into(),
	})
}

fn freeform_chunk<R>(
	data: &mut R,
	name: &[u8],
	reader_size: &mut u64,
	parse_mode: ParsingMode,
) -> Result<String>
where
	R: Read + Seek,
{
	let atom = AtomInfo::read(data, *reader_size, parse_mode)?;

	match atom {
		Some(AtomInfo {
			ident: AtomIdent::Fourcc(ref fourcc),
			len,
			..
		}) if fourcc == name => {
			if len < 12 {
				err!(BadAtom("Found an incomplete freeform identifier chunk"));
			}

			if len >= *reader_size {
				err!(SizeMismatch);
			}

			// Version (1)
			// Flags (3)
			data.seek(SeekFrom::Current(4))?;

			// Already read the size (4) + identifier (4) + version/flags (4)
			let mut content = try_vec![0; (len - 12) as usize];
			data.read_exact(&mut content)?;

			*reader_size -= len;

			utf8_decode(content).map_err(|_| {
				LoftyError::new(ErrorKind::BadAtom(
					"Found a non UTF-8 string while reading freeform identifier",
				))
			})
		},
		_ => err!(BadAtom(
			"Found freeform identifier \"----\" with no trailing \"mean\" or \"name\" atoms"
		)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Verify that we could create freeform AtomIdent constants
	#[allow(dead_code)]
	const FREEFORM_ATOM_IDENT: AtomIdent<'_> = AtomIdent::Freeform {
		mean: Cow::Borrowed("mean"),
		name: Cow::Borrowed("name"),
	};
}
