use crate::ebml::ElementId;
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::io::FileLike;

use crate::macros::try_vec;

pub struct Void(pub u32);

// \(-\)Void
impl WriteableElement for Void {
	const ID: ElementId = ElementId(0xEC);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let content = try_vec![0; self.0 as usize];

		write_element(ctx, Self::ID, &content.as_slice(), writer)?;

		Ok(())
	}
}
