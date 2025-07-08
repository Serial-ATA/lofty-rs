use crate::ebml::tag::write::{EbmlWriteExt, ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{ElementId, TargetDescriptor, TargetType, VInt};
use crate::io::FileLike;

const TargetTypeValue_ID: ElementId = ElementId(0x68CA);
const TargetType_ID: ElementId = ElementId(0x63CA);
const TagTrackUID_ID: ElementId = ElementId(0x63C5);
const TagEditionUID_ID: ElementId = ElementId(0x63C9);
const TagChapterUID_ID: ElementId = ElementId(0x63C4);
const TagAttachmentUID_ID: ElementId = ElementId(0x63C6);

impl WriteableElement for TargetDescriptor<'_> {
	const ID: ElementId = ElementId(0x63C0);

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
				TargetTypeValue_ID,
				&[].as_slice(),
				&mut element_children,
			)?;
		} else {
			let vint = VInt::<u64>::try_from(target_type as u64)?;
			write_element(ctx, TargetTypeValue_ID, &vint, &mut element_children)?;
		}

		if let TargetDescriptor::Full(target) = self {
			if let Some(name) = &target.name {
				write_element(ctx, TargetType_ID, &name.as_str(), &mut element_children)?;
			}

			if let Some(track_uids) = &target.track_uids {
				for &uid in track_uids {
					let vint = VInt::<u64>::try_from(uid)?;
					write_element(ctx, TagTrackUID_ID, &vint, &mut element_children)?;
				}
			}

			if let Some(edition_uids) = &target.edition_uids {
				for &uid in edition_uids {
					let vint = VInt::<u64>::try_from(uid)?;
					write_element(ctx, TagEditionUID_ID, &vint, &mut element_children)?;
				}
			}

			if let Some(chapter_uids) = &target.chapter_uids {
				for &uid in chapter_uids {
					let vint = VInt::<u64>::try_from(uid)?;
					write_element(ctx, TagChapterUID_ID, &vint, &mut element_children)?;
				}
			}

			if let Some(attachment_uids) = &target.attachment_uids {
				for &uid in attachment_uids {
					let vint = VInt::<u64>::try_from(uid)?;
					write_element(ctx, TagAttachmentUID_ID, &vint, &mut element_children)?;
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
			.write_element(
				ElementWriterCtx {
					max_id_len: 4,
					max_size_len: 8,
				},
				&mut buf,
			)
			.unwrap();

		let expected = vec![0x63, 0xC0, 0x80];

		assert_eq!(buf.into_inner(), expected);
	}
}
