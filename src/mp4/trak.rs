use super::atom_info::{AtomIdent, AtomInfo};
use super::read::{skip_unneeded, AtomReader};
use crate::error::Result;

use std::io::{Read, Seek, SeekFrom};

pub(crate) struct Trak {
	pub(crate) mdia: Option<AtomInfo>,
}

impl Trak {
	pub(super) fn parse<R>(reader: &mut AtomReader<R>, trak: &AtomInfo) -> Result<Self>
	where
		R: Read + Seek,
	{
		let mut mdia = None;

		let mut read = 8;

		while read < trak.len {
			let atom = reader.next()?;

			if atom.ident == AtomIdent::Fourcc(*b"mdia") {
				mdia = Some(atom);
				reader.seek(SeekFrom::Current((trak.len - read - 8) as i64))?;
				break;
			}

			skip_unneeded(reader, atom.extended, atom.len)?;
			read += atom.len;
		}

		Ok(Self { mdia })
	}
}
