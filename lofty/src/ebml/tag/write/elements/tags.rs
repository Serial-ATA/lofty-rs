use crate::ebml::tag::write::{write_element, ElementWriterCtx, WriteableElement};
use crate::ebml::{ElementId, MatroskaTagRef};
use crate::io::FileLike;

use std::io::Cursor;

// Segment\Tags
impl<'a> WriteableElement for MatroskaTagRef<'a>
{
	const ID: ElementId = ElementId(0x1254_C367);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Cursor::new(Vec::new());
		for tag in &self.tags {
			tag.write_element(ctx, &mut element_children)?;
		}

		write_element(
			ctx,
			Self::ID,
			&element_children.get_ref().as_slice(),
			writer,
		)?;

		Ok(())
	}
}
