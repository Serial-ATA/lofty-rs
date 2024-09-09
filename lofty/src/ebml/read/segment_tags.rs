use crate::config::ParseOptions;
use crate::ebml::element_reader::{ElementChildIterator, ElementIdent, ElementReaderYield};
use crate::ebml::{EbmlTag, Language, SimpleTag, TagValue, TargetType};
use crate::error::Result;

use crate::macros::decode_err;
use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	tag: &mut EbmlTag,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::Tag, _size)) => {
				read_tag(&mut children_reader.children(), tag)?
			},
			ElementReaderYield::Eof => break,
			_ => unimplemented!("Unhandled child element in \\Ebml\\Segment\\Tags: {child:?}"),
		}
	}

	Ok(())
}

fn read_tag<R>(children_reader: &mut ElementChildIterator<'_, R>, _tag: &mut EbmlTag) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		let ElementReaderYield::Master((master, _size)) = child else {
			match child {
				ElementReaderYield::Eof => break,
				_ => {
					unreachable!("Unhandled child element in \\Ebml\\Segment\\Tags\\Tag: {child:?}")
				},
			}
		};

		match master {
			ElementIdent::Targets => {
				let _ = read_targets(&mut children_reader.children())?;
			},
			ElementIdent::SimpleTag => {
				let _ = read_simple_tag(&mut children_reader.children())?;
			},
			_ => {
				unimplemented!("Unhandled child element in \\Ebml\\Segment\\Tags\\Tag: {master:?}");
			},
		}
	}

	Ok(())
}

struct Target {
	target_type_value: TargetType,
	target_type: Option<String>,
	track_uid: Vec<u64>,
	edition_uid: Vec<u64>,
	chapter_uid: Vec<u64>,
	attachment_uid: Vec<u64>,
}

fn read_targets<R>(children_reader: &mut ElementChildIterator<'_, R>) -> Result<Target>
where
	R: Read + Seek,
{
	let mut target_type_value = None;
	let mut target_type = None;
	let mut track_uid = Vec::new();
	let mut edition_uid = Vec::new();
	let mut chapter_uid = Vec::new();
	let mut attachment_uid = Vec::new();

	while let Some(child) = children_reader.next()? {
		let ElementReaderYield::Child((child, size)) = child else {
			match child {
				ElementReaderYield::Eof => break,
				_ => unreachable!(
					"Unhandled child element in \\Ebml\\Segment\\Tags\\Tag\\Targets: {child:?}"
				),
			}
		};

		match child.ident {
			ElementIdent::TargetTypeValue => {
				target_type_value = Some(children_reader.read_unsigned_int(size.value())?);
			},
			ElementIdent::TargetType => {
				target_type = Some(children_reader.read_string(size.value())?);
			},
			ElementIdent::TagTrackUID => {
				track_uid.push(children_reader.read_unsigned_int(size.value())?);
			},
			ElementIdent::TagEditionUID => {
				edition_uid.push(children_reader.read_unsigned_int(size.value())?);
			},
			ElementIdent::TagChapterUID => {
				chapter_uid.push(children_reader.read_unsigned_int(size.value())?);
			},
			ElementIdent::TagAttachmentUID => {
				attachment_uid.push(children_reader.read_unsigned_int(size.value())?);
			},
			_ => {
				unreachable!(
					"Unhandled child element in \\Ebml\\Segment\\Tags\\Tag\\Targets: {child:?}"
				)
			},
		}
	}

	let target_type_value = match target_type_value {
		// Casting the `u64` to `u8` is safe because the value is checked to be within
		// the range of `TargetType` anyway.
		Some(value) => TargetType::try_from(value as u8)?,
		// The spec defines TargetType 50 (Album) as the default value, as it is the most
		// common grouping level.
		None => TargetType::Album,
	};

	Ok(Target {
		target_type_value,
		target_type,
		track_uid,
		edition_uid,
		chapter_uid,
		attachment_uid,
	})
}

fn read_simple_tag<R>(children_reader: &mut ElementChildIterator<'_, R>) -> Result<SimpleTag>
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
					"Unhandled child element in \\Ebml\\Segment\\Tags\\Tag\\SimpleTag: {child:?}"
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

				value = Some(TagValue::String(children_reader.read_string(size.value())?));
			},
			ElementIdent::TagBinary => {
				if value.is_some() {
					log::warn!("Duplicate value found in SimpleTag, ignoring");
					children_reader.skip(size.value())?;
					continue;
				}

				value = Some(TagValue::Binary(children_reader.read_binary(size.value())?));
			},
			_ => {
				unreachable!(
					"Unhandled child element in \\Ebml\\Segment\\Tags\\Tag\\SimpleTag: {child:?}"
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
		name,
		language,
		default,
		value,
	})
}
