// *********************
// Reference Conversions
// *********************

use crate::error::Result;
use crate::mp4::{Atom, AtomData, AtomIdent, Ilst};

use std::fs::File;
use std::io::Write;

impl Ilst {
	pub(crate) fn as_ref(&self) -> IlstRef<'_, impl IntoIterator<Item = &AtomData>> {
		IlstRef {
			atoms: Box::new(self.atoms.iter().map(Atom::as_ref)),
		}
	}
}

pub(crate) struct IlstRef<'a, I> {
	pub(super) atoms: Box<dyn Iterator<Item = AtomRef<'a, I>> + 'a>,
}

impl<'a, I: 'a> IlstRef<'a, I>
where
	I: IntoIterator<Item = &'a AtomData>,
{
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		super::write::write_to(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let temp = super::write::build_ilst(&mut self.atoms)?;
		writer.write_all(&*temp)?;

		Ok(())
	}
}

impl Atom {
	pub(super) fn as_ref(&self) -> AtomRef<'_, impl IntoIterator<Item = &AtomData>> {
		AtomRef {
			ident: (&self.ident).into(),
			data: (&self.data).into_iter(),
		}
	}
}

pub(crate) struct AtomRef<'a, I> {
	pub(crate) ident: AtomIdentRef<'a>,
	pub(crate) data: I,
}

pub(crate) enum AtomIdentRef<'a> {
	Fourcc([u8; 4]),
	Freeform { mean: &'a str, name: &'a str },
}

impl<'a> Into<AtomIdentRef<'a>> for &'a AtomIdent {
	fn into(self) -> AtomIdentRef<'a> {
		match self {
			AtomIdent::Fourcc(fourcc) => AtomIdentRef::Fourcc(*fourcc),
			AtomIdent::Freeform { mean, name } => AtomIdentRef::Freeform { mean, name },
		}
	}
}

impl<'a> From<AtomIdentRef<'a>> for AtomIdent {
	fn from(input: AtomIdentRef<'a>) -> Self {
		match input {
			AtomIdentRef::Fourcc(fourcc) => AtomIdent::Fourcc(fourcc),
			AtomIdentRef::Freeform { mean, name } => AtomIdent::Freeform {
				mean: mean.to_string(),
				name: name.to_string(),
			},
		}
	}
}
