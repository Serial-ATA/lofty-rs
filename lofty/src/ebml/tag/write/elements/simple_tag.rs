use crate::ebml::element_reader::ElementIdent;
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement, write_element};
use crate::ebml::{ElementId, Language, SimpleTag, TagValue};
use crate::io::FileLike;

// Segment\Tags\Tag\SimpleTag
impl WriteableElement for SimpleTag<'_> {
	const ID: ElementId = ElementId(ElementIdent::SimpleTag as _);

	fn write_element<F: FileLike>(
		&self,
		ctx: ElementWriterCtx,
		writer: &mut F,
	) -> crate::error::Result<()> {
		let mut element_children = Vec::new();
		write_element(
			ctx,
			ElementIdent::TagName.into(),
			&self.name.as_ref(),
			&mut element_children,
		)?;

		match &self.language {
			Language::Iso639_2(iso_639_2) => write_element(
				ctx,
				ElementIdent::TagLanguage.into(),
				&iso_639_2.as_str(),
				&mut element_children,
			)?,
			Language::Bcp47(bcp47) => write_element(
				ctx,
				ElementIdent::TagLanguageBCP47.into(),
				&bcp47.as_str(),
				&mut element_children,
			)?,
		}

		write_element(
			ctx,
			ElementIdent::TagDefault.into(),
			&self.default,
			&mut element_children,
		)?;

		if let Some(value) = self.value.as_ref() {
			match value {
				TagValue::String(s) => write_element(
					ctx,
					ElementIdent::TagString.into(),
					&s.as_ref(),
					&mut element_children,
				)?,
				TagValue::Binary(b) => write_element(
					ctx,
					ElementIdent::TagBinary.into(),
					&b.as_ref(),
					&mut element_children,
				)?,
			}
		}

		write_element(ctx, Self::ID, &element_children.as_slice(), writer)?;

		Ok(())
	}
}
