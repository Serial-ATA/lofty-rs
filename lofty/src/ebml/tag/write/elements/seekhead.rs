use crate::ebml::ElementId;
use crate::ebml::element_reader::ElementIdent;
use crate::ebml::read::segment_seekhead::SeekHead;
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::io::FileLike;

use std::io::Cursor;

// Segment\SeekHead
impl WriteableElement for SeekHead {
	const ID: ElementId = ElementId(ElementIdent::SeekHead as _);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Cursor::new(Vec::new());

		for seek_entry in &self.entries {
			seek_entry.write_element(ctx, &mut element_children)?;
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
