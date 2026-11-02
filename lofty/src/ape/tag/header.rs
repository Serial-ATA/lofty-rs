use crate::ape::tag::error::ApeTagParseError;
use crate::error::SizeMismatchError;
use crate::util::io::SeekStreamLen;

use std::io::{Read, Seek, SeekFrom};
use std::ops::Neg;

use byteorder::{LittleEndian, ReadBytesExt};

// https://wiki.hydrogenaudio.org/index.php?title=Ape_Tags_Flags
pub(super) const CONTAINS_HEADER: u32 = 1 << 31;
pub(super) const CONTAINS_FOOTER: u32 = 1 << 30;
pub(super) const IS_HEADER: u32 = 1 << 29;

#[derive(Copy, Clone)]
pub(crate) struct ApeHeader {
	pub(crate) size: u32,
	pub(crate) item_count: u32,
}

/// Attempt to read an APE tag header/footer at the current position
///
/// If the parse is successful, this leaves the reader at the start of the tag items
pub(crate) fn read_ape_header<R>(
	reader: &mut R,
	footer: bool,
) -> Result<ApeHeader, ApeTagParseError>
where
	R: Read + Seek,
{
	let version = reader.read_u32::<LittleEndian>()?;

	let mut size = reader.read_u32::<LittleEndian>()?;

	if size < 32 {
		// If the size is < 32, something went wrong during encoding
		// The size includes the footer and all items
		return Err(SizeMismatchError.into());
	}

	let item_count = reader.read_u32::<LittleEndian>()?;
	let flags = reader.read_u32::<LittleEndian>()?;
	let _reserved = reader.read_u64::<LittleEndian>()?;

	if footer {
		// Seek back to the end of the header
		reader.seek(SeekFrom::Current(i64::from(size).neg()))?;

		// Version 1 doesn't include a header
		if version == 2000 && flags & CONTAINS_HEADER != 0 {
			size = size.saturating_add(32);
		}
	} else if version == 2000 && flags & CONTAINS_FOOTER != 0 {
		size = size.saturating_add(32);
	}

	if u64::from(size) > reader.stream_len_hack()? {
		return Err(SizeMismatchError.into());
	}

	Ok(ApeHeader { size, item_count })
}
