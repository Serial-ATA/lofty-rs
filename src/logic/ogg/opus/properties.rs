use super::{find_last_page, OpusProperties};
use crate::error::{LoftyError, Result};

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub(in crate::logic::ogg) fn read_properties<R>(
	data: &mut R,
	first_page: &Page,
) -> Result<OpusProperties>
where
	R: Read + Seek,
{
	let stream_len = {
		let current = data.seek(SeekFrom::Current(0))?;
		let end = data.seek(SeekFrom::End(0))?;
		data.seek(SeekFrom::Start(current))?;

		end - first_page.start
	};

	let first_page_abgp = first_page.abgp;

	// Skip identification header
	let first_page_content = &mut &first_page.content[8..];

	let version = first_page_content.read_u8()?;
	let channels = first_page_content.read_u8()?;
	let pre_skip = first_page_content.read_u16::<LittleEndian>()?;
	let input_sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	// Subtract the identification and metadata packet length from the total
	let audio_size = stream_len - data.seek(SeekFrom::Current(0))?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	last_page_abgp
		.checked_sub(first_page_abgp + u64::from(pre_skip))
		.map_or_else(
			|| Err(LoftyError::Opus("File contains incorrect PCM values")),
			|frame_count| {
				let length = frame_count * 1000 / 48000;
				let duration = Duration::from_millis(length as u64);
				let bitrate = (audio_size * 8 / length) as u32;

				Ok(OpusProperties {
					duration,
					bitrate,
					channels,
					version,
					input_sample_rate,
				})
			},
		)
}
