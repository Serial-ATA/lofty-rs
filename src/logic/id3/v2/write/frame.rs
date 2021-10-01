use crate::error::{LoftyError, Result};
use crate::logic::id3::synch_u32;
use crate::logic::id3::v2::frame::{Id3v2Frame, LanguageSpecificFrame};
use crate::logic::id3::v2::util::text_utils::{encode_text, TextEncoding};
use crate::types::item::{ItemKey, ItemValue, TagItem, TagItemFlags};
use crate::types::tag::TagType;

use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};

enum FrameType<'a> {
	EncodedText(TextEncoding),
	LanguageDependent(&'a LanguageSpecificFrame),
	UserDefined(TextEncoding, &'a str),
	Other,
}

pub(in crate::logic::id3::v2) fn create_items<W>(writer: &mut W, items: &[TagItem]) -> Result<()>
where
	W: Write,
{
	// Get rid of any invalid keys
	let items = items.iter().filter(|i| {
		(match i.key() {
			ItemKey::Id3v2Specific(Id3v2Frame::Text(name, _)) => {
				name.starts_with('T') && name.is_ascii() && name.len() == 4
			}
			ItemKey::Id3v2Specific(Id3v2Frame::URL(name)) => {
				name.starts_with('W') && name.is_ascii() && name.len() == 4
			}
			ItemKey::Id3v2Specific(id3v2_frame) => {
				std::mem::discriminant(&Id3v2Frame::Outdated(String::new()))
					!= std::mem::discriminant(id3v2_frame)
			}
			ItemKey::Unknown(_) => false,
			key => key.map_key(&TagType::Id3v2).is_some(),
		}) && matches!(
			i.value(),
			ItemValue::Text(_) | ItemValue::Locator(_) | ItemValue::Binary(_)
		)
	});

	// Get rid of any invalid keys
	for item in items {
		let value = match item.value() {
			ItemValue::Text(text) => text.as_bytes(),
			ItemValue::Locator(locator) => locator.as_bytes(),
			ItemValue::Binary(binary) => binary,
			_ => unreachable!(),
		};

		let flags = item.flags();

		match item.key() {
			ItemKey::Id3v2Specific(frame) => match frame {
				Id3v2Frame::Comment(details) => write_frame(
					writer,
					&FrameType::LanguageDependent(details),
					"COMM",
					flags,
					0,
					value,
				)?,
				Id3v2Frame::UnSyncText(details) => write_frame(
					writer,
					&FrameType::LanguageDependent(details),
					"USLT",
					flags,
					0,
					value,
				)?,
				Id3v2Frame::Text(name, encoding) => write_frame(
					writer,
					&FrameType::EncodedText(*encoding),
					name,
					flags,
					// Encoding
					1,
					value,
				)?,
				Id3v2Frame::UserText(encoding, descriptor) => write_frame(
					writer,
					&FrameType::UserDefined(*encoding, descriptor),
					"TXXX",
					flags,
					// Encoding + descriptor + null terminator
					2 + descriptor.len() as u32,
					value,
				)?,
				Id3v2Frame::URL(name) => {
					write_frame(writer, &FrameType::Other, name, flags, 0, value)?
				}
				Id3v2Frame::UserURL(encoding, descriptor) => write_frame(
					writer,
					&FrameType::UserDefined(*encoding, descriptor),
					"WXXX",
					flags,
					// Encoding + descriptor + null terminator
					2 + descriptor.len() as u32,
					value,
				)?,
				Id3v2Frame::SyncText => {
					write_frame(writer, &FrameType::Other, "SYLT", flags, 0, value)?
				}
				Id3v2Frame::EncapsulatedObject => {
					write_frame(writer, &FrameType::Other, "GEOB", flags, 0, value)?
				}
				_ => {}
			},
			key => {
				let key = key.map_key(&TagType::Id3v2).unwrap();

				if key.starts_with('T') {
					write_frame(
						writer,
						&FrameType::EncodedText(TextEncoding::UTF8),
						key,
						flags,
						// Encoding
						1,
						value,
					)?;
				} else {
					write_frame(writer, &FrameType::Other, key, flags, 0, value)?;
				}
			}
		}
	}

	Ok(())
}

fn write_frame_header<W>(writer: &mut W, name: &str, len: u32, flags: &TagItemFlags) -> Result<()>
where
	W: Write,
{
	writer.write_all(name.as_bytes())?;
	writer.write_u32::<BigEndian>(synch_u32(len)?)?;
	writer.write_u16::<BigEndian>(get_flags(flags))?;

	Ok(())
}

fn get_flags(tag_flags: &TagItemFlags) -> u16 {
	let mut flags = 0;

	if tag_flags == &TagItemFlags::default() {
		return flags;
	}

	if tag_flags.tag_alter_preservation {
		flags |= 0x4000
	}

	if tag_flags.file_alter_preservation {
		flags |= 0x2000
	}

	if tag_flags.read_only {
		flags |= 0x1000
	}

	if tag_flags.grouping_identity.0 {
		flags |= 0x0040
	}

	if tag_flags.compression {
		flags |= 0x0008
	}

	if tag_flags.encryption.0 {
		flags |= 0x0004
	}

	if tag_flags.unsynchronisation {
		flags |= 0x0002
	}

	if tag_flags.data_length_indicator.0 {
		flags |= 0x0001
	}

	flags
}

fn write_frame<W>(
	writer: &mut W,
	frame_type: &FrameType,
	name: &str,
	flags: &TagItemFlags,
	// Any additional bytes, such as encoding or language code
	additional_len: u32,
	value: &[u8],
) -> Result<()>
where
	W: Write,
{
	if flags.encryption.0 {
		write_encrypted(writer, name, value, flags)?;
		return Ok(());
	}

	let len = value.len() as u32 + additional_len;
	let is_grouping_identity = flags.grouping_identity.0;

	write_frame_header(
		writer,
		name,
		if is_grouping_identity { len + 1 } else { len },
		flags,
	)?;

	if is_grouping_identity {
		writer.write_u8(flags.grouping_identity.1)?;
	}

	match frame_type {
		FrameType::EncodedText(encoding) => {
			writer.write_u8(*encoding as u8)?;
			writer.write_all(value)?;
		}
		FrameType::LanguageDependent(details) => {
			writer.write_u8(details.encoding as u8)?;

			if details.language.len() == 3 {
				writer.write_all(details.language.as_bytes())?;
			} else {
				return Err(LoftyError::Id3v2(
					"Attempted to write a LanguageSpecificFrame with an invalid language String \
					 length (!= 3)",
				));
			}

			if let Some(ref descriptor) = details.description {
				writer.write_all(&encode_text(descriptor, details.encoding, true))?;
			} else {
				writer.write_u8(0)?;
			}

			writer.write_all(value)?;
		}
		FrameType::UserDefined(encoding, descriptor) => {
			writer.write_u8(*encoding as u8)?;
			writer.write_all(&encode_text(descriptor, *encoding, true))?;
			writer.write_all(value)?;
		}
		FrameType::Other => writer.write_all(value)?,
	}

	Ok(())
}

fn write_encrypted<W>(writer: &mut W, name: &str, value: &[u8], flags: &TagItemFlags) -> Result<()>
where
	W: Write,
{
	let method_symbol = flags.encryption.1;
	let data_length_indicator = flags.data_length_indicator;

	if method_symbol > 0x80 {
		return Err(LoftyError::Id3v2(
			"Attempted to write an encrypted frame with an invalid method symbol (> 0x80)",
		));
	}

	if data_length_indicator.0 && data_length_indicator.1 > 0 {
		write_frame_header(writer, name, (value.len() + 1) as u32, flags)?;
		writer.write_u32::<BigEndian>(synch_u32(data_length_indicator.1)?)?;
		writer.write_u8(method_symbol)?;
		writer.write_all(value)?;

		return Ok(());
	}

	Err(LoftyError::Id3v2(
		"Attempted to write an encrypted frame without a data length indicator",
	))
}
