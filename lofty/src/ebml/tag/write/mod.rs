mod elements;
mod type_encodings;

use super::MatroskaTagRef;
use crate::config::WriteOptions;
use crate::ebml::{ElementId, VInt};
use crate::error::{LoftyError, Result};
use crate::io::{FileLike, Truncate};

use std::io::Write;

use type_encodings::ElementEncodable;

#[derive(Copy, Clone)]
pub(crate) struct ElementWriterCtx {
	pub(crate) max_id_len: u8,
	pub(crate) max_size_len: u8,
}

impl Default for ElementWriterCtx {
	fn default() -> Self {
		Self {
			max_id_len: 4,
			max_size_len: 8,
		}
	}
}

pub(crate) trait EbmlWriteExt: Write + Sized {
	fn write_id(&mut self, ctx: ElementWriterCtx, id: ElementId) -> Result<()> {
		id.write_to(Some(ctx.max_id_len), self)?;
		Ok(())
	}

	fn write_size(&mut self, ctx: ElementWriterCtx, size: VInt<u64>) -> Result<()> {
		VInt::<u64>::write_to(size.value(), None, Some(ctx.max_size_len), self)?;
		Ok(())
	}
}

impl<T> EbmlWriteExt for T where T: Write {}

pub(crate) trait WriteableElement {
	const ID: ElementId;

	fn write_element<F: FileLike>(&self, ctx: ElementWriterCtx, writer: &mut F) -> Result<()>;
}

pub(crate) fn write_element<W: Write, E: ElementEncodable>(
	ctx: ElementWriterCtx,
	id: ElementId,
	element: &E,
	writer: &mut W,
) -> Result<()> {
	writer.write_id(ctx, id)?;
	element.write_to(ctx, writer)?;

	Ok(())
}

pub(crate) fn write_to<'a, F>(
	_file: &mut F,
	_tag_ref: &mut MatroskaTagRef<'a>,
	_write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
{
	todo!("Writing Segment\\Tags")
}
