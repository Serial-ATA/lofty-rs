//! A simple OGG page reader

#![allow(
	unknown_lints,
	clippy::too_many_lines,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::cast_possible_wrap,
	clippy::cast_possible_truncation,
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
	clippy::doc_markdown,
	let_underscore_drop,
	clippy::match_wildcard_for_single_variants,
	clippy::semicolon_if_nothing_returned,
	clippy::new_without_default,
	clippy::from_over_into,
	clippy::upper_case_acronyms,
	clippy::single_match_else,
	clippy::similar_names,
	clippy::tabs_in_doc_comments,
	clippy::len_without_is_empty,
	clippy::needless_late_init,
	clippy::type_complexity,
	clippy::type_repetition_in_bounds,
	unused_qualifications,
	clippy::return_self_not_must_use,
	clippy::bool_to_int_with_if,
	clippy::uninlined_format_args, /* This should be changed for any normal "{}", but I'm not a fan of it for any debug or width specific formatting */
	clippy::manual_let_else,
	clippy::struct_excessive_bools,
	clippy::match_bool
)]

mod crc;
mod error;
mod header;
mod packets;
mod paginate;

use std::io::{Read, Seek};

pub use crc::crc32;
pub use error::{PageError, Result};
pub use header::{PageHeader, PAGE_HEADER_SIZE};
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
				header: PageHeader {
					start: self.end,
					header_type_flag: 1,
					abgp: 0,
					stream_serial: self.header.stream_serial,
					sequence_number: self.header.sequence_number + 1,
					segments: segment_table(remaining),
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

	/// Consumes the page and returns its content
	#[must_use]
	pub fn take_content(self) -> Vec<u8> {
		self.content
	}

	/// Returns the page's segment table
	#[must_use]
	pub fn segment_table(&self) -> Vec<u8> {
		segment_table(self.content.len())
	}
}

/// Creates a segment table based on the length
#[must_use]
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
				segments: vec![19],
				checksum: 3579522525,
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
		}
	}
}
