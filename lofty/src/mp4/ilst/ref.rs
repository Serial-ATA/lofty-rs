// *********************
// Reference Conversions
// *********************

use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::mp4::{Atom, AtomData, AtomIdent, Ilst};
use crate::util::io::{FileLike, Length, Truncate};

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
	pub(crate) fn write_to<F>(&mut self, file: &mut F, write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		super::write::write_to(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		_write_options: WriteOptions,
	) -> Result<()> {
		let temp = super::write::build_ilst(&mut self.atoms)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

impl Atom<'_> {
	pub(super) fn as_ref(&self) -> AtomRef<'_, impl IntoIterator<Item = &AtomData>> {
		AtomRef {
			ident: self.ident.as_borrowed(),
			data: (&self.data).into_iter(),
		}
	}
}

pub(crate) struct AtomRef<'a, I> {
	pub(crate) ident: AtomIdent<'a>,
	pub(crate) data: I,
}
