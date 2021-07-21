use crate::{FileProperties, Result, LoftyError};

use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;
use std::time::Duration;

pub(in crate::components) fn read_properties<R>(
	data: &mut R,
	first_page: Page,
	stream_len: u64,
) -> Result<FileProperties>
where
	R: Read + Seek,
{
	let first_page_abgp = first_page.abgp as i64;

	let mut cursor = Cursor::new(&*first_page.content);

	// Skip identification header and version
	cursor.seek(SeekFrom::Start(11))?;

	let channels = cursor.read_u8()?;
	let pre_skip = cursor.read_u16::<LittleEndian>()?;
	let sample_rate = cursor.read_u32::<LittleEndian>()?;

	let _first_comment_page = Page::read(data, true)?;

    // Skip over the metadata packet
    loop {
        let page = Page::read(data, true)?;

        if page.header_type != 1 {
            data.seek(SeekFrom::Start(page.start as u64))?;
            break
        }
    }

	// Subtract the identification and metadata packet length from the total
	let audio_size = stream_len - data.seek(SeekFrom::Current(0))?;

    let next_page = Page::read(data, true)?;

    // Find the last page
    let mut pages: Vec<Page> = vec![next_page];

    let last_page = loop {
        if let Ok(current) = Page::read(data, true) {
            pages.push(current)
        } else {
            // Safe to unwrap since the Vec starts off with a Page
            break pages.pop().unwrap()
        }
    };

    let last_page_abgp = last_page.abgp as i64;

    let frame_count = last_page_abgp - first_page_abgp - pre_skip as i64;

    if frame_count < 0 {
        return Err(LoftyError::InvalidData("OGG file contains incorrect PCM values"))
    }

    let length = frame_count * 1000 / 48000;
    let duration = Duration::from_millis(length as u64);
    let bitrate = (audio_size * 8 / length) as u32;

	Ok(FileProperties {
        duration,
        bitrate: Some(bitrate),
        sample_rate: Some(sample_rate),
        channels: Some(channels)
    })
}
