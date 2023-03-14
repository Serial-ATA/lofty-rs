use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{err, try_vec};
use crate::tag::item::ItemKey;
use crate::tag::TagType;

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
		if let Some(mapped_key) = value.map_key(TagType::MP4ilst, true) {
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

pub(crate) struct AtomInfo {
	pub(crate) start: u64,
	pub(crate) len: u64,
	pub(crate) extended: bool,
	pub(crate) ident: AtomIdent<'static>,
}

impl AtomInfo {
	pub(crate) fn read<R>(data: &mut R, mut reader_size: u64) -> Result<Self>
	where
		R: Read + Seek,
	{
		let start = data.stream_position()?;

		let len_raw = u64::from(data.read_u32::<BigEndian>()?);

		let mut identifier = [0; IDENTIFIER_LEN as usize];
		data.read_exact(&mut identifier)?;

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
			data.seek(SeekFrom::Current(-4))?;
			err!(SizeMismatch);
		}

		let mut atom_ident = AtomIdent::Fourcc(identifier);

		// Encountered a freeform identifier
		if &identifier == b"----" {
			reader_size -= ATOM_HEADER_LEN;
			if reader_size < ATOM_HEADER_LEN {
				err!(BadAtom("Found an incomplete freeform identifier"));
			}

			atom_ident = parse_freeform(data, reader_size)?;
		}

		Ok(Self {
			start,
			len,
			extended,
			ident: atom_ident,
		})
	}
}

fn parse_freeform<R>(data: &mut R, reader_size: u64) -> Result<AtomIdent<'static>>
where
	R: Read + Seek,
{
	let mean = freeform_chunk(data, b"mean", reader_size)?;
	let name = freeform_chunk(data, b"name", reader_size - 4)?;

	Ok(AtomIdent::Freeform {
		mean: mean.into(),
		name: name.into(),
	})
}

fn freeform_chunk<R>(data: &mut R, name: &[u8], reader_size: u64) -> Result<String>
where
	R: Read + Seek,
{
	let atom = AtomInfo::read(data, reader_size)?;

	match atom.ident {
		AtomIdent::Fourcc(ref fourcc) if fourcc == name => {
			// Version (1)
			// Flags (3)
			data.seek(SeekFrom::Current(4))?;

			// Already read the size, identifier, and version/flags (12 bytes)
			let mut content = try_vec![0; (atom.len - 12) as usize];
			data.read_exact(&mut content)?;

			String::from_utf8(content).map_err(|_| {
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
