use super::FrameFlags;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::id3::v2::util::upgrade::{upgrade_v2, upgrade_v3};
use crate::id3::v2::FrameId;

use std::borrow::Cow;
use std::io::Read;

pub(crate) fn parse_v2_header<R>(
	reader: &mut R,
) -> Result<Option<(FrameId<'static>, u32, FrameFlags)>>
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

	let id_bytes = &header[..3];
	let id_str = std::str::from_utf8(id_bytes)
		.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadFrameId(id_bytes.to_vec())))
		.map(|id_str| {
			upgrade_v2(id_str).map_or_else(|| Cow::Owned(id_str.to_owned()), Cow::Borrowed)
		})?;
	let id = FrameId::new_cow(id_str)?;

	let size = u32::from_be_bytes([0, header[3], header[4], header[5]]);

	// V2 doesn't store flags
	Ok(Some((id, size, FrameFlags::default())))
}

pub(crate) fn parse_header<R>(
	reader: &mut R,
	synchsafe: bool,
) -> Result<Option<(FrameId<'static>, u32, FrameFlags)>>
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

	// For some reason, some apps make v3 tags with v2 frame IDs.
	// The actual frame header is v3 though
	let mut id_end = 4;
	let mut invalid_v2_frame = false;
	if header[3] == 0 && !synchsafe {
		invalid_v2_frame = true;
		id_end = 3;
	}

	let id_bytes = &header[..id_end];
	let id_str = std::str::from_utf8(id_bytes)
		.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadFrameId(id_bytes.to_vec())))?;

	let mut size = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);

	// Now upgrade the FrameId
	let id = if invalid_v2_frame {
		if let Some(id) = upgrade_v2(id_str) {
			Cow::Borrowed(id)
		} else {
			Cow::Owned(id_str.to_owned())
		}
	} else if !synchsafe {
		upgrade_v3(id_str).map_or_else(|| Cow::Owned(id_str.to_owned()), Cow::Borrowed)
	} else {
		Cow::Owned(id_str.to_owned())
	};
	let frame_id = FrameId::new_cow(id)?;

	// unsynch the frame size if necessary
	if synchsafe {
		size = size.unsynch();
	}

	let flags = u16::from_be_bytes([header[8], header[9]]);
	let flags = parse_flags(flags, synchsafe);

	Ok(Some((frame_id, size, flags)))
}

pub(crate) fn parse_flags(flags: u16, v4: bool) -> FrameFlags {
	FrameFlags {
		tag_alter_preservation: if v4 {
			flags & 0x4000 == 0x4000
		} else {
			flags & 0x8000 == 0x8000
		},
		file_alter_preservation: if v4 {
			flags & 0x2000 == 0x2000
		} else {
			flags & 0x4000 == 0x4000
		},
		read_only: if v4 {
			flags & 0x1000 == 0x1000
		} else {
			flags & 0x2000 == 0x2000
		},
		grouping_identity: ((v4 && flags & 0x0040 == 0x0040) || (flags & 0x0020 == 0x0020))
			.then_some(0),
		compression: if v4 {
			flags & 0x0008 == 0x0008
		} else {
			flags & 0x0080 == 0x0080
		},
		encryption: ((v4 && flags & 0x0004 == 0x0004) || flags & 0x0040 == 0x0040).then_some(0),
		unsynchronisation: if v4 { flags & 0x0002 == 0x0002 } else { false },
		data_length_indicator: (v4 && flags & 0x0001 == 0x0001).then_some(0),
	}
}
