//! A simple OGG page reader

mod crc;
mod error;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub use crc::crc32;
pub use error::{PageError, Result};

const CONTINUED_PACKET: u8 = 0x01;

/// The maximum page content size
pub const MAX_CONTENT_SIZE: usize = 65025;
/// The packet contains the first page of the logical bitstream
pub const CONTAINS_FIRST_PAGE_OF_BITSTREAM: u8 = 0x02;
/// The packet contains the last page of the logical bitstream
pub const CONTAINS_LAST_PAGE_OF_BITSTREAM: u8 = 0x04;

/// An OGG page
#[derive(Clone, PartialEq, Debug)]
pub struct Page {
	content: Vec<u8>,
	header_type: u8,
	/// The page's absolute granule position
	pub abgp: u64,
	/// The page's stream serial number
	pub serial: u32,
	/// The page's sequence number
	pub seq_num: u32,
	checksum: u32,
	/// The position in the stream the page started at
	pub start: u64,
	/// The position in the stream the page ended
	pub end: u64,
	segment_table: Vec<u8>,
}

impl Page {
	/// Create a new `Page`
	///
	/// This will have the following defaults:
	///
	/// * `checksum` = 0
	/// * `start` = 0
	/// * `end` = `content.len()`
	///
	/// # Errors
	///
	/// See [`segment_table`]
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use ogg_pager::CONTAINS_FIRST_PAGE_OF_BITSTREAM;
	///
	/// // Creating the identification header
	/// let ident_header_packet = vec![...];
	/// let stream_serial_number = 2784419176;
	///
	/// let page = Page::new(
	///     CONTAINS_FIRST_PAGE_OF_BITSTREAM,
	///     0,
	///     stream_serial_number,
	///     ident_header_packet,
	/// );
	/// ```
	pub fn new(
		header_type_flag: u8,
		abgp: u64,
		stream_serial: u32,
		sequence_number: u32,
		content: Vec<u8>,
	) -> Result<Self> {
		let len = content.len();
		let segment_table = segment_table(len)?;

		Ok(Self {
			content,
			header_type: header_type_flag,
			abgp,
			serial: stream_serial,
			seq_num: sequence_number,
			checksum: 0,
			start: 0,
			end: len as u64,
			segment_table,
		})
	}

	/// Convert the Page to Vec<u8> for writing
	///
	/// NOTE: This will write the checksum as is. It is likely [Page::gen_crc] will have
	/// to be used prior.
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::new();

		bytes.extend(b"OggS");
		bytes.push(0);
		bytes.extend(self.header_type.to_le_bytes());
		bytes.extend(self.abgp.to_le_bytes());
		bytes.extend(self.serial.to_le_bytes());
		bytes.extend(self.seq_num.to_le_bytes());
		bytes.extend(self.checksum.to_le_bytes());
		bytes.push(self.segment_table.len() as u8);
		bytes.extend(self.segment_table.iter());
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
		let start = data.seek(SeekFrom::Current(0))?;

		let mut sig = [0; 4];
		data.read_exact(&mut sig)?;

		if &sig != b"OggS" {
			return Err(PageError::MissingMagic);
		}

		// Version, always 0
		let version = data.read_u8()?;

		if version != 0 {
			return Err(PageError::InvalidVersion);
		}

		let header_type = data.read_u8()?;

		let abgp = data.read_u64::<LittleEndian>()?;
		let serial = data.read_u32::<LittleEndian>()?;
		let seq_num = data.read_u32::<LittleEndian>()?;
		let checksum = data.read_u32::<LittleEndian>()?;

		let segments = data.read_u8()?;

		if segments < 1 {
			return Err(PageError::BadSegmentCount);
		}

		let mut segment_table = vec![0; segments as usize];
		data.read_exact(&mut segment_table)?;

		let mut content: Vec<u8> = Vec::new();
		let content_len: u16 = segment_table.iter().map(|&b| u16::from(b)).sum();

		if skip_content {
			data.seek(SeekFrom::Current(i64::from(content_len)))?;
		} else {
			content = vec![0; content_len as usize];
			data.read_exact(&mut content)?;
		}

		let end = data.seek(SeekFrom::Current(0))?;

		Ok(Page {
			content,
			header_type,
			abgp,
			serial,
			seq_num,
			checksum,
			start,
			end,
			segment_table,
		})
	}

	/// Generates the CRC checksum of the page
	pub fn gen_crc(&mut self) {
		self.checksum = crc::crc32(&*self.as_bytes());
	}

	/// Extends the Page's content, returning another Page if too much data was provided
	///
	/// This will do nothing if `content` is greater than the max page size. In this case,
	/// [`paginate`] should be used.
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
			self.header_type = 0;
			self.abgp = 1_u64.wrapping_neg(); // -1 in two's complement indicates that no packets finish on this page
			self.end += remaining as u64;

			let mut p = Page {
				content: content[remaining..].to_vec(),
				header_type: 1,
				abgp: 0,
				serial: self.serial,
				seq_num: self.seq_num + 1,
				checksum: 0,
				start: self.end,
				end: self.start + content.len() as u64,
				segment_table: vec![],
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

	/// Returns the page's header type flag
	pub fn header_type(&self) -> u8 {
		self.header_type
	}

	/// Returns the page's checksum
	pub fn checksum(&self) -> u32 {
		self.checksum
	}

	/// Returns the page's segment table
	pub fn segment_table(&self) -> &[u8] {
		self.segment_table.as_slice()
	}
}

#[allow(clippy::eval_order_dependence)]
/// Create pages from a packet
///
/// # Example
///
/// ```rust,ignore
/// use ogg_pager::paginate;
///
/// // Creating the comment header
/// let comment_header_packet = vec![...];
/// let stream_serial_number = 2784419176;
///
/// let pages = paginate(&*comment_header_packet, stream_serial_number, 0, 0);
/// ```
pub fn paginate(packet: &[u8], stream_serial: u32, abgp: u64, flags: u8) -> Vec<Page> {
	let mut pages = Vec::new();

	let mut first_page = true;
	let mut pos = 0;

	for (idx, page) in packet.chunks(MAX_CONTENT_SIZE).enumerate() {
		let p = Page {
			content: page.to_vec(),
			header_type: {
				if first_page {
					if flags & CONTAINS_FIRST_PAGE_OF_BITSTREAM == 0x02 {
						CONTAINS_LAST_PAGE_OF_BITSTREAM
					} else {
						0
					}
				} else {
					CONTINUED_PACKET
				}
			},
			abgp,
			serial: stream_serial,
			seq_num: (idx + 1) as u32,
			checksum: 0,
			start: pos,
			end: {
				pos += page.len() as u64;
				pos
			},
			// Safe to unwrap, since we are working with chunks no bigger than the max page size
			segment_table: segment_table(page.len()).unwrap(),
		};

		first_page = false;
		pages.push(p);
	}

	if flags & CONTAINS_LAST_PAGE_OF_BITSTREAM == 0x04 {
		if let Some(last) = pages.last_mut() {
			last.header_type |= CONTAINS_LAST_PAGE_OF_BITSTREAM;
		}
	}

	if pages.len() > 1 {
		let last_idx = pages.len() - 1;

		for (idx, p) in pages.iter_mut().enumerate() {
			if idx == last_idx {
				break;
			}

			p.abgp = 1_u64.wrapping_neg();
		}
	}

	pages
}

/// Creates a segment table based on the length
///
/// # Errors
///
/// `length` > [`MAX_CONTENT_SIZE`]
pub fn segment_table(length: usize) -> Result<Vec<u8>> {
	let last_len = (length % 255) as u8;

	let mut needed = length / 255;
	if needed != 255 {
		needed += 1
	}

	if needed > 255 {
		return Err(PageError::TooMuchData);
	}

	let mut segments = Vec::with_capacity(needed);

	for i in 0..needed {
		if i + 1 < needed {
			segments.push(255)
		} else {
			segments.push(last_len)
		}
	}

	Ok(segments)
}

#[cfg(test)]
mod tests {
	use crate::{paginate, Page};
	use std::io::Cursor;

	#[test]
	fn opus_ident_header() {
		let expected = Page {
			content: vec![
				0x4F, 0x70, 0x75, 0x73, 0x48, 0x65, 0x61, 0x64, 0x01, 0x02, 0x38, 0x01, 0x80, 0xBB,
				0, 0, 0, 0, 0,
			],
			header_type: 2,
			abgp: 0,
			serial: 1759377061,
			seq_num: 0,
			checksum: 3579522525,
			start: 0,
			end: 47,
			segment_table: vec![0x13],
		};

		let content = std::fs::read("test_assets/opus_ident_header.page").unwrap();

		let page = Page::read(&mut Cursor::new(content), false).unwrap();

		assert_eq!(expected, page);
	}

	#[test]
	fn paginate_large() {
		let packet = std::fs::read("test_assets/large_comment_packet.page").unwrap();

		let pages = paginate(&*packet, 1234, 0, 0);

		let len = pages.len();

		assert_eq!(len, 17);
		assert_eq!(
			len % 255,
			*pages.last().unwrap().segment_table.last().unwrap() as usize
		);

		for (i, page) in pages.into_iter().enumerate() {
			assert_eq!(page.serial, 1234);

			if i + 1 == len {
				assert_eq!(page.abgp, 0);
			} else {
				// -1
				assert_eq!(page.abgp, u64::MAX);
			}

			assert_eq!(page.seq_num, (i + 1) as u32);

			if i == 0 {
				assert_eq!(page.header_type, 0);
			} else {
				assert_eq!(page.header_type, 1);
			}
		}
	}
}
