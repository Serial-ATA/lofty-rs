//! A simple OGG page reader

mod crc;
mod error;
mod header;
mod packets;
mod paginate;

use std::io::{Read, Seek, SeekFrom};

pub use crc::crc32;
pub use error::{PageError, Result};
pub use header::PageHeader;
pub use packets::{Packets, PacketsIter};
pub use paginate::paginate;

const CONTINUED_PACKET: u8 = 0x01;

/// The maximum page content size
pub const MAX_CONTENT_SIZE: usize = 65025;
/// The packet contains the first page of the logical bitstream
pub const CONTAINS_FIRST_PAGE_OF_BITSTREAM: u8 = 0x02;
/// The packet contains the last page of the logical bitstream
pub const CONTAINS_LAST_PAGE_OF_BITSTREAM: u8 = 0x04;

/// An OGG page
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Page {
	content: Vec<u8>,
	segments: Vec<u8>,
	header: PageHeader,
	/// The position in the stream the page ended
	pub end: u64,
}

impl Page {
	/// Returns a reference to the `Page`'s header
	pub fn header(&self) -> &PageHeader {
		&self.header
	}

	/// Returns a mutable reference to the `Page`'s header
	pub fn header_mut(&mut self) -> &mut PageHeader {
		&mut self.header
	}

	/// Convert the Page to bytes for writing
	///
	/// NOTE: This will write the checksum as is. It is likely [`Page::gen_crc`] will have
	/// to be used prior.
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(27 + self.segments.len() + self.content.len());

		bytes.extend(b"OggS");
		bytes.push(0); // Version
		bytes.push(self.header.header_type_flag);
		bytes.extend(self.header.abgp.to_le_bytes());
		bytes.extend(self.header.stream_serial.to_le_bytes());
		bytes.extend(self.header.sequence_number.to_le_bytes());
		bytes.extend(self.header.checksum.to_le_bytes());
		bytes.push(self.segments.len() as u8);
		bytes.extend(&self.segments);
		bytes.extend(self.content.iter());

		bytes
	}

	/// Attempts to get a Page from a reader
	///
	/// Use `skip_content` to only read the header, and skip over the content.
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	/// * [`PageError`]
	pub fn read<V>(data: &mut V, skip_content: bool) -> Result<Self>
	where
		V: Read + Seek,
	{
		let (header, segments) = PageHeader::read(data)?;

		let mut content: Vec<u8> = Vec::new();
		let content_len: u16 = segments.iter().map(|&b| u16::from(b)).sum();

		if skip_content {
			data.seek(SeekFrom::Current(i64::from(content_len)))?;
		} else {
			content = vec![0; content_len as usize];
			data.read_exact(&mut content)?;
		}

		let end = data.stream_position()?;

		Ok(Page {
			content,
			segments,
			header,
			end,
		})
	}

	/// Generates the CRC checksum of the page
	pub fn gen_crc(&mut self) {
		// The value is computed over the entire header (with the CRC field in the header set to zero) and then continued over the page
		self.header.checksum = 0;
		self.header.checksum = crc::crc32(&self.as_bytes());
	}

	/// Extends the Page's content, returning another Page if too much data was provided
	///
	/// This will do nothing if `content` is greater than the max page size. In this case,
	/// [`paginate()`] should be used.
	pub fn extend(&mut self, content: &[u8]) -> Option<Page> {
		let self_len = self.content.len();
		let content_len = content.len();

		if self_len + content_len <= MAX_CONTENT_SIZE {
			self.content.extend(content.iter());
			self.end += content_len as u64;

			return None;
		}

		if content_len <= MAX_CONTENT_SIZE {
			let remaining = 65025 - self_len;

			self.content.extend(content[0..remaining].iter());
			self.header.header_type_flag = 0;
			self.header.abgp = 1_u64.wrapping_neg(); // -1 in two's complement indicates that no packets finish on this page
			self.end += remaining as u64;

			let mut p = Page {
				content: content[remaining..].to_vec(),
				segments: segment_table(remaining),
				header: PageHeader {
					start: self.end,
					header_type_flag: 1,
					abgp: 0,
					stream_serial: self.header.stream_serial,
					sequence_number: self.header.sequence_number + 1,
					checksum: 0,
				},
				end: self.header().start + content.len() as u64,
			};

			p.gen_crc();

			return Some(p);
		}

		None
	}

	/// Returns the page's content
	pub fn content(&self) -> &[u8] {
		self.content.as_slice()
	}

	/// Consumes the page and returns it's content
	pub fn take_content(self) -> Vec<u8> {
		self.content
	}

	/// Returns the page's segment table
	pub fn segment_table(&self) -> Vec<u8> {
		segment_table(self.content.len())
	}
}

/// Creates a segment table based on the length
pub fn segment_table(length: usize) -> Vec<u8> {
	if length == 0 {
		return vec![1, 0];
	}

	let last_len = (length % 255) as u8;
	let needed = (length / 255) + 1;

	let mut segments = Vec::with_capacity(needed);

	for i in 0..needed {
		if i + 1 < needed {
			segments.push(255)
		} else {
			segments.push(last_len)
		}
	}

	segments
}

#[cfg(test)]
mod tests {
	use crate::{paginate, segment_table, Page, PageHeader};
	use std::io::Cursor;

	#[test]
	fn opus_ident_header() {
		let expected = Page {
			content: vec![
				0x4F, 0x70, 0x75, 0x73, 0x48, 0x65, 0x61, 0x64, 0x01, 0x02, 0x38, 0x01, 0x80, 0xBB,
				0, 0, 0, 0, 0,
			],
			segments: vec![19],
			header: PageHeader {
				start: 0,
				header_type_flag: 2,
				abgp: 0,
				stream_serial: 1759377061,
				sequence_number: 0,
				checksum: 3579522525,
			},
			end: 47,
		};

		let content = std::fs::read("test_assets/opus_ident_header.page").unwrap();

		let page = Page::read(&mut Cursor::new(content), false).unwrap();

		assert_eq!(expected, page);
	}

	#[test]
	fn paginate_large() {
		let packet = std::fs::read("test_assets/large_comment_packet.page").unwrap();

		let pages = paginate([packet.as_slice()], 1234, 0, 0).unwrap();

		let len = pages.len();

		assert_eq!(len, 17);
		let last_page_content = pages.last().unwrap().content();

		assert_eq!(
			last_page_content.len() % 255,
			*segment_table(last_page_content.len()).last().unwrap() as usize
		);

		for (i, page) in pages.into_iter().enumerate() {
			let header = page.header();

			assert_eq!(header.stream_serial, 1234);

			if i + 1 == len {
				assert_eq!(header.abgp, 0);
			} else {
				// -1
				assert_eq!(header.abgp, u64::MAX);
			}

			assert_eq!(header.sequence_number, i as u32);

			if i == 0 {
				assert_eq!(header.header_type_flag, 0);
			} else {
				assert_eq!(header.header_type_flag, 1);
			}
		}
	}
}
