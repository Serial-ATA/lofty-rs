use crate::ebml::element_reader::ElementIdent;
use crate::ebml::tag::write::{EbmlWriteExt, ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{DocumentType, ElementId, TargetDescriptor, TargetType, VInt};
use crate::io::FileLike;

// Segment\Tags\Tag\Targets
impl WriteableElement for TargetDescriptor<'_> {
	const ID: ElementId = ElementId(ElementIdent::Targets as _);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		if self.is_empty_candidate() {
			writer.write_id(ctx, Self::ID)?;
			writer.write_size(ctx, VInt::<u64>::ZERO)?;
			return Ok(());
		}

		let mut element_children = Vec::new();

		let target_type = self.target_type();
		if target_type == TargetType::Album {
			write_element(
				ctx,
				ElementIdent::TargetTypeValue.into(),
				&[].as_slice(),
				&mut element_children,
			)?;
		} else {
			write_element(
				ctx,
				ElementIdent::TargetTypeValue.into(),
				&(target_type as u64),
				&mut element_children,
			)?;
		}

		if let TargetDescriptor::Full(target) = self {
			if let Some(name) = &target.name {
				write_element(
					ctx,
					ElementIdent::TargetType.into(),
					&name.as_str(),
					&mut element_children,
				)?;
			}

			// None of these fields are supported in WebM
			if ctx.doc_type == DocumentType::Matroska {
				if let Some(track_uids) = &target.track_uids {
					for &uid in track_uids {
						write_element(
							ctx,
							ElementIdent::TagTrackUID.into(),
							&uid,
							&mut element_children,
						)?;
					}
				}

				if let Some(edition_uids) = &target.edition_uids {
					for &uid in edition_uids {
						write_element(
							ctx,
							ElementIdent::TagEditionUID.into(),
							&uid,
							&mut element_children,
						)?;
					}
				}

				if let Some(chapter_uids) = &target.chapter_uids {
					for &uid in chapter_uids {
						write_element(
							ctx,
							ElementIdent::TagChapterUID.into(),
							&uid,
							&mut element_children,
						)?;
					}
				}

				if let Some(attachment_uids) = &target.attachment_uids {
					for &uid in attachment_uids {
						write_element(
							ctx,
							ElementIdent::TagAttachmentUID.into(),
							&uid,
							&mut element_children,
						)?;
					}
				}
			}
		}

		write_element(ctx, Self::ID, &element_children.as_slice(), writer)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ebml::Target;

	use std::io::Cursor;

	#[test_log::test]
	fn write_empty_default() {
		let target = Target::default();

		let mut buf = Cursor::new(Vec::new());
		let target_descriptor = TargetDescriptor::from(&target);
		target_descriptor
			.write_element(ElementWriterCtx::default(), &mut buf)
			.unwrap();

		let expected = vec![0x63, 0xC0, 0x80];

		assert_eq!(buf.into_inner(), expected);
	}
}
