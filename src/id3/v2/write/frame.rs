use crate::error::{ID3v2Error, ID3v2ErrorKind, Result};
use crate::id3::v2::frame::{FrameFlags, FrameRef, FrameValue};
use crate::id3::v2::util::synch_u32;

use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};

pub(in crate::id3::v2) fn create_items<'a, W>(
	writer: &mut W,
	frames: &mut dyn Iterator<Item = FrameRef<'a>>,
) -> Result<()>
where
	W: Write,
{
	for frame in frames {
		verify_frame(&frame)?;
		let value = frame.value.as_bytes()?;

		write_frame(writer, frame.id, frame.flags, &value)?;
	}

	Ok(())
}

fn verify_frame(frame: &FrameRef<'_>) -> Result<()> {
	match (frame.id, frame.value.as_ref()) {
		("APIC", FrameValue::Picture { .. })
		| ("USLT", FrameValue::UnSyncText(_))
		| ("COMM", FrameValue::Comment(_))
		| ("TXXX", FrameValue::UserText(_))
		| ("WXXX", FrameValue::UserURL(_))
		| (_, FrameValue::Binary(_))
		| ("WFED" | "GRP1" | "MVNM" | "MVIN", FrameValue::Text { .. }) => Ok(()),
		(id, FrameValue::Text { .. }) if id.starts_with('T') => Ok(()),
		(id, FrameValue::URL(_)) if id.starts_with('W') => Ok(()),
		(id, frame_value) => Err(ID3v2Error::new(ID3v2ErrorKind::BadFrame(
			id.to_string(),
			match frame_value {
				FrameValue::Comment(_) => "Comment",
				FrameValue::UnSyncText(_) => "UnSyncText",
				FrameValue::Text { .. } => "Text",
				FrameValue::UserText(_) => "UserText",
				FrameValue::URL(_) => "URL",
				FrameValue::UserURL(_) => "UserURL",
				FrameValue::Picture { .. } => "Picture",
				FrameValue::Popularimeter(_) => "Popularimeter",
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
	if flags.encryption.0 {
		write_encrypted(writer, name, value, flags)?;
		return Ok(());
	}

	let len = value.len() as u32;
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

	writer.write_all(value)?;

	Ok(())
}

fn write_encrypted<W>(writer: &mut W, name: &str, value: &[u8], flags: FrameFlags) -> Result<()>
where
	W: Write,
{
	let method_symbol = flags.encryption.1;
	let data_length_indicator = flags.data_length_indicator;

	if method_symbol > 0x80 {
		return Err(ID3v2Error::new(ID3v2ErrorKind::Other(
			"Attempted to write an encrypted frame with an invalid method symbol (> 0x80)",
		))
		.into());
	}

	if data_length_indicator.0 && data_length_indicator.1 > 0 {
		write_frame_header(writer, name, (value.len() + 1) as u32, flags)?;
		writer.write_u32::<BigEndian>(synch_u32(data_length_indicator.1)?)?;
		writer.write_u8(method_symbol)?;
		writer.write_all(value)?;

		return Ok(());
	}

	Err(ID3v2Error::new(ID3v2ErrorKind::Other(
		"Attempted to write an encrypted frame without a data length indicator",
	))
	.into())
}

fn write_frame_header<W>(writer: &mut W, name: &str, len: u32, flags: FrameFlags) -> Result<()>
where
	W: Write,
{
	writer.write_all(name.as_bytes())?;
	writer.write_u32::<BigEndian>(synch_u32(len)?)?;
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
