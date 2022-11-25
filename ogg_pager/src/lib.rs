//! A simple OGG page reader

mod crc;
mod error;
mod header;

use std::io::{Read, Seek, SeekFrom};

pub use crc::crc32;
pub use error::{PageError, Result};
pub use header::PageHeader;

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
	header: PageHeader,
	/// The position in the stream the page ended
	pub end: u64,
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
	/// `content.len()` > [`MAX_CONTENT_SIZE`]
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
	pub fn new(header: PageHeader, content: Vec<u8>) -> Result<Self> {
		let content_len = content.len();

		if content_len > MAX_CONTENT_SIZE {
			return Err(PageError::TooMuchData);
		}

		Ok(Self {
			content,
			header,
			end: content_len as u64,
		})
	}

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
	///
	/// # Errors
	///
	/// See [`segment_table`]
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut segment_table = self.segment_table()?;
		let mut bytes = Vec::with_capacity(27 + segment_table.len() + self.content.len());

		bytes.extend(b"OggS");
		bytes.push(0); // Version
		bytes.push(self.header.header_type_flag);
		bytes.extend(self.header.abgp.to_le_bytes());
		bytes.extend(self.header.stream_serial.to_le_bytes());
		bytes.extend(self.header.sequence_number.to_le_bytes());
		bytes.extend(self.header.checksum.to_le_bytes());
		bytes.push(segment_table.len() as u8);
		bytes.append(&mut segment_table);
		bytes.extend(self.content.iter());

		Ok(bytes)
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
		let (header, segment_table) = PageHeader::read(data)?;

		let mut content: Vec<u8> = Vec::new();
		let content_len: u16 = segment_table.iter().map(|&b| u16::from(b)).sum();

		if skip_content {
			data.seek(SeekFrom::Current(i64::from(content_len)))?;
		} else {
			content = vec![0; content_len as usize];
			data.read_exact(&mut content)?;
		}

		let end = data.stream_position()?;

		Ok(Page {
			content,
			header,
			end,
		})
	}

	/// Generates the CRC checksum of the page
	///
	/// # Errors
	///
	/// See [`Page::as_bytes`]
	pub fn gen_crc(&mut self) -> Result<()> {
		self.header.checksum = crc::crc32(&self.as_bytes()?);
		Ok(())
	}

	/// Extends the Page's content, returning another Page if too much data was provided
	///
	/// This will do nothing if `content` is greater than the max page size. In this case,
	/// [`paginate`] should be used.
	///
	/// # Errors
	///
	/// *Only applicable if a new page is created*:
	///
	/// See [`Page::gen_crc`]
	pub fn extend(&mut self, content: &[u8]) -> Result<Option<Page>> {
		let self_len = self.content.len();
		let content_len = content.len();

		if self_len + content_len <= MAX_CONTENT_SIZE {
			self.content.extend(content.iter());
			self.end += content_len as u64;

			return Ok(None);
		}

		if content_len <= MAX_CONTENT_SIZE {
			let remaining = 65025 - self_len;

			self.content.extend(content[0..remaining].iter());
			self.header.header_type_flag = 0;
			self.header.abgp = 1_u64.wrapping_neg(); // -1 in two's complement indicates that no packets finish on this page
			self.end += remaining as u64;

			let mut p = Page {
				content: content[remaining..].to_vec(),
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

			p.gen_crc()?;

			return Ok(Some(p));
		}

		Ok(None)
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
	///
	/// # Errors
	///
	/// See [`segment_table`]
	pub fn segment_table(&self) -> Result<Vec<u8>> {
		segment_table(self.content.len())
	}
}

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
/// let pages = paginate(&comment_header_packet, stream_serial_number, 0, 0);
/// ```
#[allow(clippy::mixed_read_write_in_expression)]
pub fn paginate(packet: &[u8], stream_serial: u32, abgp: u64, flags: u8) -> Vec<Page> {
	let mut pages = Vec::new();

	let mut first_page = true;
	let mut pos = 0;

	for (idx, page) in packet.chunks(MAX_CONTENT_SIZE).enumerate() {
		let p = Page {
			content: page.to_vec(),
			header: PageHeader {
				start: pos,
				header_type_flag: {
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
				stream_serial,
				sequence_number: (idx + 1) as u32,
				checksum: 0,
			},
			end: {
				pos += page.len() as u64;
				pos
			},
		};

		first_page = false;
		pages.push(p);
	}

	if flags & CONTAINS_LAST_PAGE_OF_BITSTREAM == 0x04 {
		if let Some(last) = pages.last_mut() {
			last.header.header_type_flag |= CONTAINS_LAST_PAGE_OF_BITSTREAM;
		}
	}

	if pages.len() > 1 {
		let last_idx = pages.len() - 1;

		for (idx, p) in pages.iter_mut().enumerate() {
			if idx == last_idx {
				break;
			}

			p.header.abgp = 1_u64.wrapping_neg();
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
	match length {
		0 => return Ok(vec![1, 0]),
		l if l > MAX_CONTENT_SIZE => return Err(PageError::TooMuchData),
		_ => {},
	};

	let mut last_len = (length % 255) as u8;
	if last_len == 0 {
		last_len = 255;
	}

	let mut needed = (length / 255) + 1;
	needed = std::cmp::min(needed, 255);

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

/// A container for packets in an OGG file
pub struct Packets {
	content: Vec<u8>,
	packet_sizes: Vec<u64>,
}

impl Packets {
	/// Read as many packets as possible from a reader
	///
	/// # Errors
	///
	/// A page has a bad length
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let packets = Packets::read(&mut file)?;
	/// # Ok(()) }
	/// ```
	pub fn read<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Self::read_count(data, -1)
	}

	/// Read a specific number of packets from a reader
	///
	/// A special value of `-1` will read as many packets as possible,
	/// in which case [`Packets::read`] should be used.
	///
	/// NOTE: Any value 0 or below will return an empty [`Packets`]
	///
	/// # Errors
	///
	/// * Unable to read the specified number of packets
	/// * A page has a bad length
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// // We know that the file has at least 2 packets in it
	/// let packets = Packets::read_count(&mut file, 2)?;
	/// # Ok(()) }
	/// ```
	pub fn read_count<R>(data: &mut R, count: isize) -> Result<Self>
	where
		R: Read + Seek,
	{
		let mut content = Vec::new();
		let mut packet_sizes = Vec::new();

		if count == 0 || count < -1 {
			return Ok(Self {
				content,
				packet_sizes,
			});
		}

		let mut read = 0;

		let mut packet_size = 0_u64;
		let mut current_packet_content;
		'outer: loop {
			if let Ok((_, segment_table)) = PageHeader::read(data) {
				for i in segment_table {
					packet_size += i as u64;

					if i < 255 {
						if count != -1 {
							read += 1;
						}

						current_packet_content = vec![0; packet_size as usize];
						data.read_exact(&mut current_packet_content)?;

						packet_sizes.push(packet_size);
						packet_size = 0;

						content.append(&mut current_packet_content);

						if read == count {
							break 'outer;
						}
					}
				}

				// The packet continues on the next page, write what we can so far
				if packet_size != 0 {
					current_packet_content = vec![0; packet_size as usize];
					data.read_exact(&mut current_packet_content)?;
				}

				continue;
			}

			break;
		}

		if count != -1 && packet_sizes.len() != read as usize {
			return Err(PageError::NotEnoughData);
		}

		Ok(Self {
			content,
			packet_sizes,
		})
	}

	/// Gets the packet at a specified index, returning its contents
	///
	/// NOTES:
	///
	/// * This is zero-indexed
	/// * If the index is out of bounds, it will return [`None`]
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let packets = Packets::read(&mut file)?;
	///
	/// let first_packet = packets.get(0);
	/// assert!(first_packet.is_some());
	///
	/// let out_of_bounds = packets.get(1000000);
	/// assert!(out_of_bounds.is_none());
	/// # Ok(()) }
	/// ```
	pub fn get(&self, idx: usize) -> Option<&[u8]> {
		if idx >= self.content.len() {
			return None;
		}

		let start_pos = match idx {
			// Packet 0 starts at pos 0
			0 => 0,
			// Anything else we have to get the size of the previous packet
			other => self.packet_sizes[other - 1] as usize,
		};

		if let Some(packet_size) = self.packet_sizes.get(idx) {
			return Some(&self.content[start_pos..start_pos + *packet_size as usize]);
		}

		None
	}

	/// Sets the packet content, if it exists
	///
	/// NOTES:
	///
	/// * This is zero-indexed
	/// * If the index is out of bounds, it will return `false`
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let mut packets = Packets::read(&mut file)?;
	///
	/// let new_content = [0; 100];
	///
	/// assert_ne!(packets.get(0), Some(&new_content));
	///
	/// // Set our new content
	/// assert!(packets.set(0, new_content));
	///
	/// // Now our packet contains the new content
	/// assert_eq!(packets.get(0), Some(&new_content));
	///
	/// // We cannot index out of bounds
	/// assert!(!packets.set(1000000, new_content));
	/// # Ok(()) }
	/// ```
	pub fn set(&mut self, idx: usize, content: impl Into<Vec<u8>>) -> bool {
		if idx >= self.packet_sizes.len() {
			return false;
		}

		let start_pos = match idx {
			// Packet 0 starts at pos 0
			0 => 0,
			// Anything else we have to get the size of the previous packet
			other => self.packet_sizes[other - 1] as usize,
		};

		let content = content.into();
		let content_size = content.len();

		let end_pos = start_pos + self.packet_sizes[idx] as usize;
		self.content.splice(start_pos..end_pos, content);

		self.packet_sizes[idx] = content_size as u64;

		true
	}
}

/// An iterator over packets
pub struct PacketsIter<'a> {
	content: &'a [u8],
	packet_sizes: &'a [u64],
	cap: usize,
}

impl<'a> Iterator for PacketsIter<'a> {
	type Item = &'a [u8];

	fn next(&mut self) -> Option<Self::Item> {
		if self.cap == 0 {
			return None;
		}

		let packet_size = self.packet_sizes[0];

		self.cap -= 1;
		self.packet_sizes = &self.packet_sizes[1..];

		let (ret, remaining) = self.content.split_at(packet_size as usize);
		self.content = remaining;

		Some(ret)
	}
}

impl<'a> IntoIterator for &'a Packets {
	type Item = &'a [u8];
	type IntoIter = PacketsIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		PacketsIter {
			content: &self.content,
			packet_sizes: &self.packet_sizes,
			cap: self.packet_sizes.len(),
		}
	}
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

		let pages = paginate(&packet, 1234, 0, 0);

		let len = pages.len();

		assert_eq!(len, 17);
		let last_page_content = pages.last().unwrap().content();

		assert_eq!(
			last_page_content.len() % 255,
			*segment_table(last_page_content.len())
				.unwrap()
				.last()
				.unwrap() as usize
		);

		for (i, page) in pages.into_iter().enumerate() {
			let header = page.header;

			assert_eq!(header.stream_serial, 1234);

			if i + 1 == len {
				assert_eq!(header.abgp, 0);
			} else {
				// -1
				assert_eq!(header.abgp, u64::MAX);
			}

			assert_eq!(header.sequence_number, (i + 1) as u32);

			if i == 0 {
				assert_eq!(header.header_type_flag, 0);
			} else {
				assert_eq!(header.header_type_flag, 1);
			}
		}
	}
}
