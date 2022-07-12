use super::atom_info::{AtomIdent, AtomInfo};
use super::read::skip_unneeded;
use super::trak::Trak;
#[cfg(feature = "mp4_ilst")]
use super::{
	ilst::{read::parse_ilst, Ilst},
	read::{meta_is_full, AtomReader},
};
use crate::error::{FileDecodingError, Result};
use crate::file::FileType;

use std::io::{Read, Seek};

pub(crate) struct Moov {
	pub(crate) traks: Vec<Trak>,
	#[cfg(feature = "mp4_ilst")]
	// Represents a parsed moov.udta.meta.ilst since we don't need anything else
	pub(crate) meta: Option<Ilst>,
}

impl Moov {
	pub(super) fn find<R>(reader: &mut AtomReader<R>) -> Result<AtomInfo>
	where
		R: Read + Seek,
	{
		let mut moov = None;

		while let Ok(atom) = reader.next() {
			if atom.ident == AtomIdent::Fourcc(*b"moov") {
				moov = Some(atom);
				break;
			}

			skip_unneeded(reader, atom.extended, atom.len)?;
		}

		if let Some(moov) = moov {
			Ok(moov)
		} else {
			Err(FileDecodingError::new(FileType::MP4, "No \"moov\" atom found").into())
		}
	}

	pub(super) fn parse<R>(reader: &mut AtomReader<R>, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
	{
		let mut traks = Vec::new();
		#[cfg(feature = "mp4_ilst")]
		let mut meta = None;

		while let Ok(atom) = reader.next() {
			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"trak" if read_properties => traks.push(Trak::parse(reader, &atom)?),
					#[cfg(feature = "mp4_ilst")]
					b"udta" => {
						meta = meta_from_udta(reader, atom.len - 8)?;
					},
					_ => skip_unneeded(reader, atom.extended, atom.len)?,
				}

				continue;
			}

			skip_unneeded(reader, atom.extended, atom.len)?
		}

		Ok(Self {
			traks,
			#[cfg(feature = "mp4_ilst")]
			meta,
		})
	}
}

#[cfg(feature = "mp4_ilst")]
fn meta_from_udta<R>(reader: &mut AtomReader<R>, len: u64) -> Result<Option<Ilst>>
where
	R: Read + Seek,
{
	let mut read = 8;
	let mut meta = (false, 0_u64);

	while read < len {
		let atom = reader.next()?;

		if atom.ident == AtomIdent::Fourcc(*b"meta") {
			meta = (true, atom.len);
			break;
		}

		read += atom.len;
		skip_unneeded(reader, atom.extended, atom.len)?;
	}

	if !meta.0 {
		return Ok(None);
	}

	// It's possible for the `meta` atom to be non-full,
	// so we have to check for that case
	let full_meta_atom = meta_is_full(reader)?;

	if full_meta_atom {
		read = 12;
	} else {
		read = 8;
	}

	let mut islt = (false, 0_u64);

	while read < meta.1 {
		let atom = reader.next()?;

		if atom.ident == AtomIdent::Fourcc(*b"ilst") {
			islt = (true, atom.len);
			break;
		}

		read += atom.len;
		skip_unneeded(reader, atom.extended, atom.len)?;
	}

	if islt.0 {
		return parse_ilst(reader, islt.1 - 8).map(Some);
	}

	Ok(None)
}
