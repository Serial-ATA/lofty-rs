use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::{FrameFlags, FrameRef, FrameValue};
use crate::id3::v2::util::synchsafe::SynchsafeInteger;

use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};

pub(in crate::id3::v2) fn create_items<W>(
	writer: &mut W,
	frames: &mut dyn Iterator<Item = FrameRef<'_>>,
) -> Result<()>
where
	W: Write,
{
	for frame in frames {
		verify_frame(&frame)?;
		let value = frame.value.as_bytes()?;

		write_frame(writer, frame.id.as_str(), frame.flags, &value)?;
	}

	Ok(())
}

fn verify_frame(frame: &FrameRef<'_>) -> Result<()> {
	match (frame.id.as_str(), frame.value.as_ref()) {
		("APIC", FrameValue::Picture { .. })
		| ("USLT", FrameValue::UnsynchronizedText(_))
		| ("COMM", FrameValue::Comment(_))
		| ("TXXX", FrameValue::UserText(_))
		| ("WXXX", FrameValue::UserUrl(_))
		| (_, FrameValue::Binary(_))
		| ("UFID", FrameValue::UniqueFileIdentifier(_))
		| ("POPM", FrameValue::Popularimeter(_))
		| ("TIPL" | "TMCL", FrameValue::KeyValue { .. })
		| ("WFED" | "GRP1" | "MVNM" | "MVIN", FrameValue::Text { .. }) => Ok(()),
		(id, FrameValue::Text { .. }) if id.starts_with('T') => Ok(()),
		(id, FrameValue::Url(_)) if id.starts_with('W') => Ok(()),
		(id, frame_value) => Err(Id3v2Error::new(Id3v2ErrorKind::BadFrame(
			id.to_string(),
			match frame_value {
				FrameValue::Comment(_) => "Comment",
				FrameValue::UnsynchronizedText(_) => "UnsynchronizedText",
				FrameValue::Text { .. } => "Text",
				FrameValue::UserText(_) => "UserText",
				FrameValue::Url(_) => "Url",
				FrameValue::UserUrl(_) => "UserUrl",
				FrameValue::Picture { .. } => "Picture",
				FrameValue::Popularimeter(_) => "Popularimeter",
				FrameValue::KeyValue(_) => "KeyValue",
				FrameValue::UniqueFileIdentifier(_) => "UniqueFileIdentifier",
				FrameValue::RelativeVolumeAdjustment(_) => "RelativeVolumeAdjustment",
				FrameValue::Ownership(_) => "Ownership",
				FrameValue::EventTimingCodes(_) => "EventTimingCodes",
				FrameValue::Private(_) => "Private",
				FrameValue::Binary(_) => "Binary",
			},
		))
		.into()),
	}
}

fn write_frame<W>(writer: &mut W, name: &str, flags: FrameFlags, value: &[u8]) -> Result<()>
where
	W: Write,
{
	if flags.encryption.is_some() {
		write_encrypted(writer, name, value, flags)?;
		return Ok(());
	}

	let len = value.len() as u32;
	let is_grouping_identity = flags.grouping_identity.is_some();

	write_frame_header(
		writer,
		name,
		if is_grouping_identity { len + 1 } else { len },
		flags,
	)?;

	if is_grouping_identity {
		// Guaranteed to be `Some` at this point.
		writer.write_u8(flags.grouping_identity.unwrap())?;
	}

	writer.write_all(value)?;

	Ok(())
}

fn write_encrypted<W>(writer: &mut W, name: &str, value: &[u8], flags: FrameFlags) -> Result<()>
where
	W: Write,
{
	// Guaranteed to be `Some` at this point.
	let method_symbol = flags.encryption.unwrap();

	if method_symbol > 0x80 {
		return Err(
			Id3v2Error::new(Id3v2ErrorKind::InvalidEncryptionMethodSymbol(method_symbol)).into(),
		);
	}

	if let Some(len) = flags.data_length_indicator {
		if len > 0 {
			write_frame_header(writer, name, (value.len() + 1) as u32, flags)?;
			writer.write_u32::<BigEndian>(len.synch()?)?;
			writer.write_u8(method_symbol)?;
			writer.write_all(value)?;

			return Ok(());
		}
	}

	Err(Id3v2Error::new(Id3v2ErrorKind::MissingDataLengthIndicator).into())
}

fn write_frame_header<W>(writer: &mut W, name: &str, len: u32, flags: FrameFlags) -> Result<()>
where
	W: Write,
{
	writer.write_all(name.as_bytes())?;
	writer.write_u32::<BigEndian>(len.synch()?)?;
	writer.write_u16::<BigEndian>(get_flags(flags))?;

	Ok(())
}

fn get_flags(tag_flags: FrameFlags) -> u16 {
	let mut flags = 0;

	if tag_flags == FrameFlags::default() {
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

	if tag_flags.grouping_identity.is_some() {
		flags |= 0x0040
	}

	if tag_flags.compression {
		flags |= 0x0008
	}

	if tag_flags.encryption.is_some() {
		flags |= 0x0004
	}

	if tag_flags.unsynchronisation {
		flags |= 0x0002
	}

	if tag_flags.data_length_indicator.is_some() {
		flags |= 0x0001
	}

	flags
}
