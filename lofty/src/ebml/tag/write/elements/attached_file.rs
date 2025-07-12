use crate::ebml::element_reader::ElementIdent;
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{AttachedFile, ElementId};
use crate::io::FileLike;

// Segment\Attachments\AttachedFile
impl WriteableElement for AttachedFile<'_> {
	const ID: ElementId = ElementId(ElementIdent::AttachedFile as _);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		self.validate()?;

		let mut element_children = Vec::new();
		if let Some(description) = &self.description {
			write_element(
				ctx,
				ElementIdent::FileDescription.into(),
				&description.as_ref(),
				&mut element_children,
			)?;
		}

		write_element(
			ctx,
			ElementIdent::FileName.into(),
			&self.file_name.as_ref(),
			&mut element_children,
		)?;

		write_element(
			ctx,
			ElementIdent::FileMediaType.into(),
			&self.mime_type.as_str(),
			&mut element_children,
		)?;

		write_element(
			ctx,
			ElementIdent::FileData.into(),
			&self.file_data.as_ref(),
			&mut element_children,
		)?;

		write_element(
			ctx,
			ElementIdent::FileUID.into(),
			&self.uid,
			&mut element_children,
		)?;

		if let Some(referral) = &self.referral {
			write_element(
				ctx,
				ElementIdent::FileReferral.into(),
				&referral.as_ref(),
				&mut element_children,
			)?;
		}

		if let Some(start_time) = &self.used_start_time {
			write_element(
				ctx,
				ElementIdent::FileUsedStartTime.into(),
				start_time,
				&mut element_children,
			)?;
		}

		if let Some(end_time) = &self.used_end_time {
			write_element(
				ctx,
				ElementIdent::FileUsedEndTime.into(),
				end_time,
				&mut element_children,
			)?;
		}

		write_element(ctx, Self::ID, &element_children.as_slice(), writer)?;

		Ok(())
	}
}
