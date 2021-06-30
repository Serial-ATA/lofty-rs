use std::io::{Read, Seek};

use ogg_pager::Page;

use crate::{LoftyError, Result};

pub(crate) mod constants;
pub(crate) mod read;
pub(crate) mod write;

pub fn page_from_packet(packet: &mut [u8]) -> Result<Vec<Page>> {
	let mut pages: Vec<Page> = Vec::new();

	let reader = &mut &packet[..];

	let mut start = 0_usize;
	let mut i = 0;

	while !reader.is_empty() {
		let header_type = if i == 0 { 0 } else { 1_u8 };

		let size = std::cmp::min(65025, reader.len());

		if i != 0 {
			if let Some(s) = start.checked_add(size) {
				start = s
			} else {
				return Err(LoftyError::TooMuchData);
			}
		}

		let mut content = vec![0; size];
		reader.read_exact(&mut content)?;

		let end = start + size;

		pages.push(Page {
			content,
			header_type,
			abgp: 0,
			serial: 0, // Retrieved later
			seq_num: (i + 1) as u32,
			checksum: 0, // Calculated later
			start,
			end,
		});

		i += 1;
	}

	Ok(pages)
}

pub(self) fn reach_metadata<T>(mut data: T, sig: &[u8]) -> Result<()>
where
	T: Read + Seek,
{
	let first_page = Page::read(&mut data)?;

	let head = first_page.content;
	let (ident, head) = head.split_at(sig.len());

	if ident != sig {
		return Err(LoftyError::InvalidData("OGG file missing magic signature"));
	}

	if head[10] != 0 {
		let mut channel_mapping_info = [0; 1];
		data.read_exact(&mut channel_mapping_info)?;

		let mut channel_mapping = vec![0; channel_mapping_info[0] as usize];
		data.read_exact(&mut channel_mapping)?;
	}

	Ok(())
}

// Verify the 2nd page contains the comment header
pub(self) fn is_metadata(page: &Page, sig: &[u8]) -> Result<()> {
	let sig_len = sig.len();

	if page.content.len() < sig_len || &page.content[0..sig_len] != sig {
		return Err(LoftyError::InvalidData(
			"OGG file missing the mandatory comment header",
		));
	}

	Ok(())
}
