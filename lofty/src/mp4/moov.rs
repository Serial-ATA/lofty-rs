use super::atom_info::{AtomIdent, AtomInfo};
use super::ilst::Ilst;
use super::ilst::read::parse_ilst;
use super::read::{AtomReader, find_child_atom, meta_is_full, skip_atom};
use crate::config::ParseOptions;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

pub(crate) struct Moov {
	// Represents the trak.mdia atom
	pub(crate) traks: Vec<AtomInfo>,
	// Represents a parsed moov.udta.meta.ilst
	pub(crate) ilst: Option<Ilst>,
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

			skip_atom(reader, atom.extended, atom.len)?;
		}

		moov.ok_or_else(|| decode_err!(Mp4, "No \"moov\" atom found"))
	}

	pub(super) fn parse<R>(reader: &mut AtomReader<R>, parse_options: ParseOptions) -> Result<Self>
	where
		R: Read + Seek,
	{
		let mut traks = Vec::new();
		let mut ilst = None;

		while let Ok(Some(atom)) = reader.next() {
			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"trak" if parse_options.read_properties => {
						// All we need from here is trak.mdia
						if let Some(mdia) =
							find_child_atom(reader, atom.len, *b"mdia", parse_options.parsing_mode)?
						{
							skip_atom(reader, mdia.extended, mdia.len)?;
							traks.push(mdia);
						}
					},
					b"udta" if parse_options.read_tags => {
						let ilst_parsed = ilst_from_udta(reader, parse_options, atom.len - 8)?;
						if let Some(ilst_parsed) = ilst_parsed {
							let Some(mut existing_ilst) = ilst else {
								ilst = Some(ilst_parsed);
								continue;
							};

							log::warn!("Multiple `ilst` atoms found, combining them");
							for atom in ilst_parsed.atoms {
								existing_ilst.insert(atom);
							}

							ilst = Some(existing_ilst);
						}
					},
					_ => skip_atom(reader, atom.extended, atom.len)?,
				}

				continue;
			}

			skip_atom(reader, atom.extended, atom.len)?
		}

		Ok(Self { traks, ilst })
	}
}

fn ilst_from_udta<R>(
	reader: &mut AtomReader<R>,
	parse_options: ParseOptions,
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
		skip_atom(reader, atom.extended, atom.len)?;
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
		skip_atom(reader, atom.extended, atom.len)?;
	}

	if found_ilst {
		return parse_ilst(reader, parse_options, ilst_atom_size - 8).map(Some);
	}

	Ok(None)
}
