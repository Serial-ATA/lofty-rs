use super::{find_last_page, VorbisProperties};
use crate::error::{LoftyError, Result};

use std::io::{Read, Seek};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub(in crate::logic::ogg) fn read_properties<R>(
	data: &mut R,
	first_page: &Page,
) -> Result<VorbisProperties>
where
	R: Read + Seek,
{
	let first_page_abgp = first_page.abgp;

	// Skip identification header
	let first_page_content = &mut &first_page.content[7..];

	let version = first_page_content.read_u32::<LittleEndian>()?;

	let channels = first_page_content.read_u8()?;
	let sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	let bitrate_maximum = first_page_content.read_u32::<LittleEndian>()?;
	let bitrate_nominal = first_page_content.read_u32::<LittleEndian>()?;
	let bitrate_minimum = first_page_content.read_u32::<LittleEndian>()?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	last_page_abgp.checked_sub(first_page_abgp).map_or_else(
		|| Err(LoftyError::Vorbis("File contains incorrect PCM values")),
		|frame_count| {
			let length = frame_count * 1000 / u64::from(sample_rate);
			let duration = Duration::from_millis(length as u64);
			let bitrate = bitrate_nominal / 1000;

			Ok(VorbisProperties {
				duration,
				bitrate,
				sample_rate,
				channels,
				version,
				bitrate_maximum,
				bitrate_nominal,
				bitrate_minimum,
			})
		},
	)
}
