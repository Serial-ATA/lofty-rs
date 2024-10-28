use crate::ebml::tag::write::{write_element, ElementWriterCtx, WriteableElement};
use crate::ebml::{ElementId, SimpleTag, TagRef};
use crate::io::FileLike;

use std::borrow::Cow;
use std::io::Cursor;

impl<'a, I> WriteableElement for TagRef<'a, I>
where
	I: Iterator<Item = Cow<'a, SimpleTag<'a>>>,
{
	const ID: ElementId = ElementId(0x7373);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Cursor::new(Vec::new());
		self.targets.write_element(ctx, &mut element_children)?;

		// TODO
		// for simple_tag in self.simple_tags {
		// 	simple_tag.write_element(ctx, &mut element_children)?;
		// }

		write_element(
			ctx,
			Self::ID,
			&element_children.get_ref().as_slice(),
			writer,
		)?;

		Ok(())
	}
}
