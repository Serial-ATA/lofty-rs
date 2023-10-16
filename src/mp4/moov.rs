use super::atom_info::{AtomIdent, AtomInfo};
use super::ilst::read::parse_ilst;
use super::ilst::Ilst;
use super::read::{meta_is_full, nested_atom, skip_unneeded, AtomReader};
use crate::error::Result;
use crate::macros::decode_err;
use crate::ParsingMode;

use std::io::{Read, Seek};

pub(crate) struct Moov {
	// Represents the trak.mdia atom
	pub(crate) traks: Vec<AtomInfo>,
	// Represents a parsed moov.udta.meta.ilst
	pub(crate) meta: Option<Ilst>,
}

impl Moov {
	pub(super) fn find<R>(reader: &mut AtomReader<R>) -> Result<AtomInfo>
	where
		R: Read + Seek,
	{
		let mut moov = None;

		while let Ok(Some(atom)) = reader.next() {
			if atom.ident == AtomIdent::Fourcc(*b"moov") {
				moov = Some(atom);
				break;
			}

			skip_unneeded(reader, atom.extended, atom.len)?;
		}

		moov.ok_or_else(|| decode_err!(Mp4, "No \"moov\" atom found"))
	}

	pub(super) fn parse<R>(
		reader: &mut AtomReader<R>,
		parse_mode: ParsingMode,
		read_properties: bool,
	) -> Result<Self>
	where
		R: Read + Seek,
	{
		let mut traks = Vec::new();
		let mut meta = None;

		while let Ok(Some(atom)) = reader.next() {
			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"trak" if read_properties => {
						// All we need from here is trak.mdia
						if let Some(mdia) = nested_atom(reader, atom.len, b"mdia", parse_mode)? {
							skip_unneeded(reader, mdia.extended, mdia.len)?;
							traks.push(mdia);
						}
					},
					b"udta" => {
						meta = meta_from_udta(reader, parse_mode, atom.len - 8)?;
					},
					_ => skip_unneeded(reader, atom.extended, atom.len)?,
				}

				continue;
			}

			skip_unneeded(reader, atom.extended, atom.len)?
		}

		Ok(Self { traks, meta })
	}
}

fn meta_from_udta<R>(
	reader: &mut AtomReader<R>,
	parsing_mode: ParsingMode,
	len: u64,
) -> Result<Option<Ilst>>
where
	R: Read + Seek,
{
	let mut read = 8;
	let mut found_meta = false;
	let mut meta_atom_size = 0;

	while read < len {
		let Some(atom) = reader.next()? else {
			break;
		};

		if atom.ident == AtomIdent::Fourcc(*b"meta") {
			found_meta = true;
			meta_atom_size = atom.len;
			break;
		}

		read += atom.len;
		skip_unneeded(reader, atom.extended, atom.len)?;
	}

	if !found_meta {
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

	let mut found_ilst = false;
	let mut ilst_atom_size = 0;

	while read < meta_atom_size {
		let Some(atom) = reader.next()? else {
			break;
		};

		if atom.ident == AtomIdent::Fourcc(*b"ilst") {
			found_ilst = true;
			ilst_atom_size = atom.len;
			break;
		}

		read += atom.len;
		skip_unneeded(reader, atom.extended, atom.len)?;
	}

	if found_ilst {
		return parse_ilst(reader, parsing_mode, ilst_atom_size - 8).map(Some);
	}

	Ok(None)
}
