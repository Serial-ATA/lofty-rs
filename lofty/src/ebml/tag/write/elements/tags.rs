use crate::ebml::element_reader::ElementIdent;
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{ElementId, TagRef};
use crate::io::FileLike;

use std::io::Cursor;

pub struct Tags<'a>(pub Vec<TagRef<'a>>);

// Segment\Tags
impl WriteableElement for Tags<'_> {
	const ID: ElementId = ElementId(ElementIdent::Tags as _);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Cursor::new(Vec::new());
		for tag in &self.0 {
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
