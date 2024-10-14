use crate::config::ParseOptions;
use crate::ebml::element_reader::{ElementChildIterator, ElementIdent, ElementReaderYield};
use crate::ebml::{Language, MatroskaTag, SimpleTag, Tag, TagValue, Target, TargetType};
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	tag: &mut MatroskaTag,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::Tag, _size)) => {
				let tag_element = read_tag(&mut children_reader.children())?;
				tag.tags.push(tag_element);
			},
			ElementReaderYield::Eof => break,
			_ => unimplemented!("Unhandled child element in \\Segment\\Tags: {child:?}"),
		}
	}

	Ok(())
}

fn read_tag<R>(children_reader: &mut ElementChildIterator<'_, R>) -> Result<Tag<'static>>
where
	R: Read + Seek,
{
	let mut target = None;
	let mut simple_tags = Vec::new();

	while let Some(child) = children_reader.next()? {
		let ElementReaderYield::Master((master, _size)) = child else {
			match child {
				ElementReaderYield::Eof => break,
				_ => {
					unreachable!("Unhandled child element in \\Segment\\Tags\\Tag: {child:?}")
				},
			}
		};

		match master {
			ElementIdent::Targets => {
				if target.is_some() {
					decode_err!(
						@BAIL Ebml,
						"Duplicate Targets element found in \\Segment\\Tags\\Tag"
					);
				}

				target = Some(read_targets(&mut children_reader.children())?);
			},
			ElementIdent::SimpleTag => {
				simple_tags.push(read_simple_tag(&mut children_reader.children())?)
			},
			_ => {
				unimplemented!("Unhandled child element in \\Segment\\Tags\\Tag: {master:?}");
			},
		}
	}

	let Some(target) = target else {
		decode_err!(@BAIL Ebml, "\\Segment\\Tags\\Tag is missing the required `Targets` element");
	};

	Ok(Tag {
		target: Some(target),
		simple_tags,
	})
}

fn read_targets<R>(children_reader: &mut ElementChildIterator<'_, R>) -> Result<Target>
where
	R: Read + Seek,
{
	let mut target = Target::default();

	while let Some(child) = children_reader.next()? {
		let ElementReaderYield::Child((child, size)) = child else {
			match child {
				ElementReaderYield::Eof => break,
				_ => unreachable!(
					"Unhandled child element in \\Segment\\Tags\\Tag\\Targets: {child:?}"
				),
			}
		};

		match child.ident {
			ElementIdent::TargetTypeValue => {
				let value = children_reader.read_unsigned_int(size.value())?;

				// Casting the `u64` to `u8` is safe because the value is checked to be within
				// the range of `TargetType` anyway.
				let target_type = TargetType::try_from(value as u8)?;
				target.target_type = target_type;
			},
			ElementIdent::TargetType => {
				target.name = Some(children_reader.read_string(size.value())?);
			},
			ElementIdent::TagTrackUID => {
				let mut track_uids = target.track_uids.unwrap_or_default();
				track_uids.push(children_reader.read_unsigned_int(size.value())?);
				target.track_uids = Some(track_uids);
			},
			ElementIdent::TagEditionUID => {
				let mut edition_uids = target.edition_uids.unwrap_or_default();
				edition_uids.push(children_reader.read_unsigned_int(size.value())?);
				target.edition_uids = Some(edition_uids);
			},
			ElementIdent::TagChapterUID => {
				let mut chapter_uids = target.chapter_uids.unwrap_or_default();
				chapter_uids.push(children_reader.read_unsigned_int(size.value())?);
				target.chapter_uids = Some(chapter_uids);
			},
			ElementIdent::TagAttachmentUID => {
				let mut attachment_uids = target.attachment_uids.unwrap_or_default();
				attachment_uids.push(children_reader.read_unsigned_int(size.value())?);
				target.attachment_uids = Some(attachment_uids);
			},
			_ => {
				unreachable!("Unhandled child element in \\Segment\\Tags\\Tag\\Targets: {child:?}")
			},
		}
	}

	Ok(target)
}

fn read_simple_tag<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
) -> Result<SimpleTag<'static>>
where
	R: Read + Seek,
{
	let mut name = None;
	let mut language = None;
	let mut default = false;
	let mut value = None;

	while let Some(child) = children_reader.next()? {
		let ElementReaderYield::Child((child, size)) = child else {
			match child {
				ElementReaderYield::Eof => break,
				_ => unreachable!(
					"Unhandled child element in \\Segment\\Tags\\Tag\\SimpleTag: {child:?}"
				),
			}
		};

		match child.ident {
			ElementIdent::TagName => {
				name = Some(children_reader.read_string(size.value())?);
			},
			ElementIdent::TagLanguage => {
				if language.is_some() {
					log::warn!("Duplicate language found in SimpleTag, ignoring");
					children_reader.skip(size.value())?;
					continue;
				}

				language = Some(Language::Iso639_2(
					children_reader.read_string(size.value())?,
				));
			},
			ElementIdent::TagLanguageBCP47 => {
				if language.is_some() {
					log::warn!("Duplicate language found in SimpleTag, ignoring");
					children_reader.skip(size.value())?;
					continue;
				}

				language = Some(Language::Bcp47(children_reader.read_string(size.value())?));
			},
			ElementIdent::TagDefault => {
				default = children_reader.read_flag(size.value())?;
			},
			ElementIdent::TagString => {
				if value.is_some() {
					log::warn!("Duplicate value found in SimpleTag, ignoring");
					children_reader.skip(size.value())?;
					continue;
				}

				value = Some(TagValue::from(children_reader.read_string(size.value())?));
			},
			ElementIdent::TagBinary => {
				if value.is_some() {
					log::warn!("Duplicate value found in SimpleTag, ignoring");
					children_reader.skip(size.value())?;
					continue;
				}

				value = Some(TagValue::from(children_reader.read_binary(size.value())?));
			},
			_ => {
				unreachable!(
					"Unhandled child element in \\Segment\\Tags\\Tag\\SimpleTag: {child:?}"
				);
			},
		}
	}

	let Some(name) = name else {
		decode_err!(
			@BAIL Ebml,
			"SimpleTag is missing the required TagName element"
		);
	};

	Ok(SimpleTag {
		name: name.into(),
		language,
		default,
		value,
	})
}
