use crate::ebml::tag::write::{write_element, ElementWriterCtx, WriteableElement};
use crate::ebml::{ElementId, Language, SimpleTag, TagValue};
use crate::io::FileLike;

const TagName_ID: ElementId = ElementId(0x45A3);
const TagLanguage_ID: ElementId = ElementId(0x447A);
const TagLanguageBcp47_ID: ElementId = ElementId(0x447B);
const TagDefault_ID: ElementId = ElementId(0x4484);
const TagString_ID: ElementId = ElementId(0x4487);
const TagBinary_ID: ElementId = ElementId(0x4485);

impl WriteableElement for SimpleTag<'_> {
	const ID: ElementId = ElementId(0x67C8);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Vec::new();
		write_element(ctx, TagName_ID, &self.name.as_ref(), &mut element_children)?;

		match &self.language {
			Language::Iso639_2(iso_639_2) => write_element(
				ctx,
				TagLanguage_ID,
				&iso_639_2.as_str(),
				&mut element_children,
			)?,
			Language::Bcp47(bcp47) => write_element(
				ctx,
				TagLanguageBcp47_ID,
				&bcp47.as_str(),
				&mut element_children,
			)?,
		}

		write_element(ctx, TagDefault_ID, &self.default, &mut element_children)?;

		if let Some(value) = self.value.as_ref() {
			match value {
				TagValue::String(s) => {
					write_element(ctx, TagString_ID, &s.as_ref(), &mut element_children)?
				},
				TagValue::Binary(b) => {
					write_element(ctx, TagBinary_ID, &b.as_ref(), &mut element_children)?
				},
			}
		}

		write_element(ctx, Self::ID, &element_children.as_slice(), writer)?;

		Ok(())
	}
}
