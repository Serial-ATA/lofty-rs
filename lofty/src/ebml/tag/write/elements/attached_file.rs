use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{AttachedFile, ElementId, VInt};
use crate::io::FileLike;

const FileDescription_ID: ElementId = ElementId(0x467E);
const FileName_ID: ElementId = ElementId(0x466E);
const FileMediaType_ID: ElementId = ElementId(0x4660);
const FileData_ID: ElementId = ElementId(0x465C);
const FileUID_ID: ElementId = ElementId(0x46AE);
const FileReferral_ID: ElementId = ElementId(0x4675);
const FileUsedStartTime_ID: ElementId = ElementId(0x4661);
const FileUsedEndTime_ID: ElementId = ElementId(0x4662);

// Segment\Attachments\AttachedFile
impl WriteableElement for AttachedFile<'_> {
	const ID: ElementId = ElementId(0x61A7);

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
				FileDescription_ID,
				&description.as_ref(),
				&mut element_children,
			)?;
		}

		write_element(
			ctx,
			FileName_ID,
			&self.file_name.as_ref(),
			&mut element_children,
		)?;

		write_element(
			ctx,
			FileMediaType_ID,
			&self.mime_type.as_str(),
			&mut element_children,
		)?;

		write_element(
			ctx,
			FileData_ID,
			&self.file_data.as_ref(),
			&mut element_children,
		)?;

		let uid = VInt::<u64>::try_from(self.uid)?;
		write_element(ctx, FileUID_ID, &uid, &mut element_children)?;

		if let Some(referral) = &self.referral {
			write_element(
				ctx,
				FileReferral_ID,
				&referral.as_ref(),
				&mut element_children,
			)?;
		}

		if let Some(start_time) = &self.used_start_time {
			let vint = VInt::<u64>::try_from(*start_time)?;
			write_element(ctx, FileUsedStartTime_ID, &vint, &mut element_children)?;
		}

		if let Some(end_time) = &self.used_end_time {
			let vint = VInt::<u64>::try_from(*end_time)?;
			write_element(ctx, FileUsedEndTime_ID, &vint, &mut element_children)?;
		}

		write_element(ctx, Self::ID, &element_children.as_slice(), writer)?;

		Ok(())
	}
}
