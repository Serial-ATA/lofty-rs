use crate::error::{FileDecodingError, Result};
use crate::file::FileType;
use crate::traits::SeekStreamLen;

use std::io::{Read, Seek, SeekFrom};
use std::ops::Neg;

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Copy, Clone)]
pub(crate) struct ApeHeader {
	pub(crate) size: u32,
	#[cfg(feature = "ape")]
	pub(crate) item_count: u32,
}

pub(crate) fn read_ape_header<R>(data: &mut R, footer: bool) -> Result<ApeHeader>
where
	R: Read + Seek,
{
	let version = data.read_u32::<LittleEndian>()?;

	let mut size = data.read_u32::<LittleEndian>()?;

	if size < 32 {
		// If the size is < 32, something went wrong during encoding
		// The size includes the footer and all items
		return Err(
			FileDecodingError::new(FileType::APE, "APE tag has an invalid size (< 32)").into(),
		);
	}

	#[cfg(feature = "ape")]
	let item_count = data.read_u32::<LittleEndian>()?;

	#[cfg(not(feature = "ape"))]
	data.seek(SeekFrom::Current(4))?;

	if footer {
		// No point in reading the rest of the footer, just seek back to the end of the header
		data.seek(SeekFrom::Current(i64::from(size - 12).neg()))?;
	} else {
		// There are 12 bytes remaining in the header
		// Flags (4)
		// Reserved (8)
		data.seek(SeekFrom::Current(12))?;
	}

	// Version 1 doesn't include a header
	if version == 2000 {
		size = size.saturating_add(32);
	}

	#[allow(unstable_name_collisions)]
	if u64::from(size) > data.stream_len()? {
		return Err(FileDecodingError::new(
			FileType::APE,
			"APE tag has an invalid size (> file size)",
		)
		.into());
	}

	Ok(ApeHeader {
		size,
		#[cfg(feature = "ape")]
		item_count,
	})
}
