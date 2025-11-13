use super::FrameFlags;
use crate::config::ParseOptions;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::FrameId;
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::id3::v2::util::upgrade::{upgrade_v2, upgrade_v3};
use crate::util::text::utf8_decode_str;

use std::borrow::Cow;
use std::io::Read;

pub(crate) fn parse_v2_header<R>(
	reader: &mut R,
	size: &mut u32,
) -> Result<Option<(FrameId<'static>, FrameFlags)>>
where
	R: Read,
{
	let mut header = [0; 6];
	match reader.read_exact(&mut header) {
		Ok(_) => {},
		Err(_) => return Ok(None),
	}

	// Assume we just started reading padding
	if header[0] == 0 {
		return Ok(None);
	}

	*size = u32::from_be_bytes([0, header[3], header[4], header[5]]);

	let id_bytes = &header[..3];
	let id_str = std::str::from_utf8(id_bytes)
		.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadFrameId(id_bytes.to_vec())))
		.map(|id_str| {
			upgrade_v2(id_str).map_or_else(|| Cow::Owned(id_str.to_owned()), Cow::Borrowed)
		})?;
	let id = FrameId::new_cow(id_str)?;

	// V2 doesn't store flags
	Ok(Some((id, FrameFlags::default())))
}

pub(crate) fn parse_header<R>(
	reader: &mut R,
	size: &mut u32,
	synchsafe: bool,
	parse_options: ParseOptions,
) -> Result<Option<(FrameId<'static>, FrameFlags)>>
where
	R: Read,
{
	let mut header = [0; 10];
	match reader.read_exact(&mut header) {
		Ok(_) => {},
		Err(_) => return Ok(None),
	}

	// Assume we just started reading padding
	if header[0] == 0 {
		return Ok(None);
	}

	*size = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
	// unsynch the frame size if necessary
	if synchsafe {
		*size = size.unsynch();
	}

	// For some reason, some apps make v3 tags with v2 frame IDs.
	// The actual frame header is v3 though
	let mut id_end = 4;
	let mut invalid_v2_frame = false;
	if header[3] == 0 && !synchsafe {
		log::warn!("Found a v2 frame ID in a v3 tag, attempting to upgrade");

		invalid_v2_frame = true;
		id_end = 3;
	}

	let id_bytes = &header[..id_end];
	let id_str = utf8_decode_str(id_bytes)
		.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadFrameId(id_bytes.to_vec())))?;

	// Now upgrade the FrameId
	let id = if invalid_v2_frame {
		if let Some(id) = upgrade_v2(id_str) {
			Cow::Borrowed(id)
		} else {
			Cow::Owned(id_str.to_owned())
		}
	} else if !synchsafe && parse_options.implicit_conversions {
		upgrade_v3(id_str).map_or_else(|| Cow::Owned(id_str.to_owned()), Cow::Borrowed)
	} else {
		Cow::Owned(id_str.to_owned())
	};
	let frame_id = FrameId::new_cow(id)?;

	let flags = u16::from_be_bytes([header[8], header[9]]);
	let flags = if synchsafe {
		FrameFlags::parse_id3v24(flags)
	} else {
		FrameFlags::parse_id3v23(flags)
	};

	Ok(Some((frame_id, flags)))
}
