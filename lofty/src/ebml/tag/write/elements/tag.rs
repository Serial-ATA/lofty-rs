use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{ElementId, TagRef};
use crate::io::FileLike;

use std::io::Cursor;

// Segment\Tags\Tag
impl WriteableElement for TagRef<'_> {
	const ID: ElementId = ElementId(0x7373);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Cursor::new(Vec::new());
		self.targets.write_element(ctx, &mut element_children)?;

		for simple_tag in &self.simple_tags {
			simple_tag.write_element(ctx, &mut element_children)?;
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
