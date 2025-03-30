//! A simple OGG page reader

mod crc;
mod error;
mod header;
mod packets;
mod paginate;

use std::io::{Read, Seek};

pub use crc::crc32;
pub use error::{PageError, Result};
pub use header::{PAGE_HEADER_SIZE, PageHeader};
pub use packets::{Packets, PacketsIter};
pub use paginate::paginate;

const CONTINUED_PACKET: u8 = 0x01;
pub(crate) const MAX_WRITTEN_SEGMENT_COUNT: usize = 32;
pub(crate) const MAX_WRITTEN_CONTENT_SIZE: usize = MAX_WRITTEN_SEGMENT_COUNT * 255;

/// The maximum page content size
// NOTE: An OGG page can have up to 255 segments, or ~64KB. We cap it at 32 segments, or ~8KB when writing.
pub const MAX_CONTENT_SIZE: usize = MAX_WRITTEN_CONTENT_SIZE * 4;
/// The maximum number of segments a page can contain
pub const MAX_SEGMENT_COUNT: usize = 255;
/// The packet contains the first page of the logical bitstream
pub const CONTAINS_FIRST_PAGE_OF_BITSTREAM: u8 = 0x02;
/// The packet contains the last page of the logical bitstream
pub const CONTAINS_LAST_PAGE_OF_BITSTREAM: u8 = 0x04;

/// An OGG page
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Page {
	content: Vec<u8>,
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
	#[must_use]
	pub fn as_bytes(&self) -> Vec<u8> {
		let segment_table = &self.header.segments;
		let num_segments = segment_table.len();
		let mut bytes =
			Vec::with_capacity(PAGE_HEADER_SIZE + num_segments + self.header.content_size());

		bytes.extend(b"OggS");
		bytes.push(0); // Version
		bytes.push(self.header.header_type_flag);
		bytes.extend(self.header.abgp.to_le_bytes());
		bytes.extend(self.header.stream_serial.to_le_bytes());
		bytes.extend(self.header.sequence_number.to_le_bytes());
		bytes.extend(self.header.checksum.to_le_bytes());
		bytes.push(num_segments as u8);
		bytes.extend(segment_table);
		bytes.extend(self.content.iter());

		bytes
	}

	/// Attempts to get a Page from a reader
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	/// * [`PageError`]
	pub fn read<V>(data: &mut V) -> Result<Self>
	where
		V: Read + Seek,
	{
		let header = PageHeader::read(data)?;

		let mut content = vec![0; header.content_size()];
		data.read_exact(&mut content)?;

		let end = data.stream_position()?;

		Ok(Page {
			content,
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

	/// Returns the page's content
	pub fn content(&self) -> &[u8] {
		self.content.as_slice()
	}

	/// Consumes the page and returns its content
	#[must_use]
	pub fn take_content(self) -> Vec<u8> {
		self.content
	}
}

#[cfg(test)]
mod tests {
	use crate::{Page, PageHeader, paginate};
	use std::io::Cursor;

	pub fn segment_table(length: usize) -> Vec<u8> {
		if length == 0 {
			return vec![1, 0];
		}

		let last_len = (length % 255) as u8;
		let needed = (length / 255) + 1;

		let mut segments = Vec::with_capacity(needed);

		for i in 0..needed {
			if i + 1 < needed {
				segments.push(255);
			} else {
				segments.push(last_len);
			}
		}

		segments
	}

	#[test]
	fn opus_ident_header() {
		let expected = Page {
			content: vec![
				0x4F, 0x70, 0x75, 0x73, 0x48, 0x65, 0x61, 0x64, 0x01, 0x02, 0x38, 0x01, 0x80, 0xBB,
				0, 0, 0, 0, 0,
			],
			header: PageHeader {
				start: 0,
				header_type_flag: 2,
				abgp: 0,
				stream_serial: 1_759_377_061,
				sequence_number: 0,
				segments: vec![19],
				checksum: 3_579_522_525,
			},
			end: 47,
		};

		let content = std::fs::read("test_assets/opus_ident_header.page").unwrap();

		let page = Page::read(&mut Cursor::new(content)).unwrap();

		assert_eq!(expected, page);
	}

	#[test]
	fn paginate_large() {
		let packet = std::fs::read("test_assets/large_comment_packet.page").unwrap();

		let pages = paginate([packet.as_slice()], 1234, 0, 0).unwrap();

		let len = pages.len();

		assert_eq!(len, 130);
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

			if i + 1 == len {
				let segments = &header.segments[..header.segments.len()];
				for s in &segments[..segments.len() - 1] {
					assert_eq!(*s, 255);
				}

				assert_eq!(segments.last(), Some(&171));
			} else {
				assert_eq!(header.segments, vec![255; super::MAX_WRITTEN_SEGMENT_COUNT]);
			}
		}
	}

	#[test]
	fn paginate_large_perfectly_divisible() {
		// Create 20 max-size pages.
		// This means we will need to make another page with a single zero segment table.
		const PAGES_TO_WRITE: usize = 20;
		const PACKET_SIZE: usize = (super::MAX_WRITTEN_SEGMENT_COUNT * 255) * PAGES_TO_WRITE;

		let packet = vec![0; PACKET_SIZE];

		let pages = paginate([packet.as_slice()], 1234, 0, 0).unwrap();

		let len = pages.len();
		assert_eq!(len, PAGES_TO_WRITE + 1);

		for (i, page) in pages.iter().enumerate() {
			if i + 1 == len {
				break;
			}

			assert!(page.header.segments.iter().all(|c| *c == 255));
		}

		let last = pages.last().unwrap();
		assert_eq!(last.header.segments.len(), 1);
		assert_eq!(*last.header.segments.first().unwrap(), 0);

		let mut total_size = 0;
		for page in pages {
			total_size += page
				.header
				.segments
				.iter()
				.map(|&b| usize::from(b))
				.sum::<usize>();
		}

		assert_eq!(total_size, PACKET_SIZE);
	}

	#[test]
	fn paginate_perfectly_divisible_terminate() {
		// Our segment table will have 17 max-size segments, it should be terminated with a 0 to
		// indicate the end of our packet.
		const SEGMENTS: usize = 17;
		const PACKET_SIZE: usize = SEGMENTS * 255;

		let packet = vec![0; PACKET_SIZE];

		let pages = paginate([packet.as_slice()], 1234, 0, 0).unwrap();

		let len = pages.len();
		assert_eq!(len, 1);

		let page = &pages[0];

		// + 1 for the terminating 0
		assert_eq!(page.header.segments.len(), SEGMENTS + 1);

		let correct_number_of_segments = page
			.header
			.segments
			.iter()
			.take(SEGMENTS)
			.all(|&b| b == 255);
		assert!(correct_number_of_segments);

		assert_eq!(*page.header.segments.last().unwrap(), 0);
	}
}
