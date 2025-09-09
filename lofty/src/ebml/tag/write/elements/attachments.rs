use crate::ebml::element_reader::ElementIdent;
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{AttachedFile, ElementId};
use crate::io::FileLike;

use std::borrow::Cow;
use std::io::Cursor;

pub struct Attachments<'a>(pub Vec<Cow<'a, AttachedFile<'a>>>);

// Segment\Tags
impl WriteableElement for Attachments<'_> {
	const ID: ElementId = ElementId(ElementIdent::Attachments as _);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Cursor::new(Vec::new());
		for file in &self.0 {
			file.write_element(ctx, &mut element_children)?;
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
