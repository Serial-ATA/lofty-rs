use super::find_last_page;
use crate::error::{LoftyError, Result};
use crate::types::properties::FileProperties;

use std::io::{Read, Seek};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub(in crate::logic::ogg) fn read_properties<R>(
	data: &mut R,
	first_page: &Page,
) -> Result<FileProperties>
where
	R: Read + Seek,
{
	let first_page_abgp = first_page.abgp;

	// Skip identification header and version
	let first_page_content = &mut &first_page.content[11..];

	let channels = first_page_content.read_u8()?;
	let sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	let _bitrate_max = first_page_content.read_u32::<LittleEndian>()?;
	let bitrate_nominal = first_page_content.read_u32::<LittleEndian>()?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	last_page_abgp.checked_sub(first_page_abgp).map_or_else(
		|| Err(LoftyError::Vorbis("File contains incorrect PCM values")),
		|frame_count| {
			let length = frame_count * 1000 / u64::from(sample_rate);
			let duration = Duration::from_millis(length as u64);
			let bitrate = bitrate_nominal / 1000;

			Ok(FileProperties::new(
				duration,
				Some(bitrate),
				Some(sample_rate),
				Some(channels),
			))
		},
	)
}
