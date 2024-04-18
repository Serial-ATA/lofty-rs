use crate::error::Result;
use crate::{
	Page, PageHeader, CONTAINS_FIRST_PAGE_OF_BITSTREAM, CONTAINS_LAST_PAGE_OF_BITSTREAM,
	CONTINUED_PACKET, MAX_WRITTEN_CONTENT_SIZE, MAX_WRITTEN_SEGMENT_COUNT,
};

use std::io::Read;

struct PaginateContext {
	pages: Vec<Page>,
	abgp: u64,
	stream_serial: u32,
	header_flags: u8,
	flags: PaginateContextFlags,
	pos: u64,
	idx: usize,
	remaining_page_size: usize,
	current_packet_len: usize,
	last_segment_size: u8,
}

impl PaginateContext {
	fn new(abgp: u64, stream_serial: u32, header_flags: u8) -> Self {
		Self {
			pages: Vec::new(),
			abgp,
			stream_serial,
			header_flags,
			flags: PaginateContextFlags {
				first_page: true,
				fresh_packet: true,
				packet_spans_multiple_pages: false,
				packet_finished_on_page: false,
			},
			pos: 0,
			idx: 0,
			remaining_page_size: MAX_WRITTEN_CONTENT_SIZE,
			current_packet_len: 0,
			last_segment_size: 0,
		}
	}

	fn fresh_packet(&mut self, packet: &[u8]) {
		self.flags.fresh_packet = true;
		self.pos = 0;

		self.current_packet_len = packet.len();
		self.last_segment_size = (packet.len() % 255) as u8;
	}

	fn flush_page(&mut self, content: &mut Vec<u8>) {
		let mut header = PageHeader {
			start: self.pos,
			header_type_flag: {
				match self.flags.first_page {
					true if self.header_flags & CONTAINS_FIRST_PAGE_OF_BITSTREAM != 0 => {
						CONTAINS_FIRST_PAGE_OF_BITSTREAM
					},
					// A packet from the previous page continues onto this page
					false if !self.flags.fresh_packet => CONTINUED_PACKET,
					_ => 0,
				}
			},
			abgp: if self.flags.packet_finished_on_page {
				self.abgp
			} else {
				// A special value of '-1' (in two's complement) indicates that no packets
				// finish on this page.
				1_u64.wrapping_neg()
			},
			stream_serial: self.stream_serial,
			sequence_number: self.idx as u32,
			segments: Vec::new(),
			// Calculated later
			checksum: 0,
		};

		let content = core::mem::take(content);
		let content_len = content.len();
		self.pos += content_len as u64;

		// Moving on to a new packet
		debug_assert!(self.pos <= self.current_packet_len as u64);
		if self.pos == self.current_packet_len as u64 {
			self.flags.packet_spans_multiple_pages = false;
		}

		// We need to determine how many segments our page content takes up.
		// If it takes up the remainder of the segment table for the entire packet,
		// we'll just consume it as is.
		let segments_occupied = if content_len >= 255 {
			content_len.div_ceil(255)
		} else {
			1
		};

		debug_assert!(segments_occupied <= MAX_WRITTEN_SEGMENT_COUNT);
		if self.flags.packet_spans_multiple_pages {
			header.segments = vec![255; segments_occupied];
		} else {
			header.segments = vec![255; segments_occupied - 1];
			header.segments.push(self.last_segment_size);
		}

		self.pages.push(Page {
			content,
			header,
			end: self.pos,
		});

		self.idx += 1;
		self.flags.packet_finished_on_page = false;
		self.remaining_page_size = MAX_WRITTEN_CONTENT_SIZE;
	}
}

struct PaginateContextFlags {
	first_page: bool,
	fresh_packet: bool,
	packet_spans_multiple_pages: bool,
	packet_finished_on_page: bool,
}

/// Create pages from a list of packets
///
/// # Errors
///
/// * Unable to read packet content
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
pub fn paginate<'a, I: 'a>(
	packets: I,
	stream_serial: u32,
	abgp: u64,
	flags: u8,
) -> Result<Vec<Page>>
where
	I: IntoIterator<Item = &'a [u8]>,
{
	let mut ctx = PaginateContext::new(abgp, stream_serial, flags);

	for packet in packets {
		ctx.fresh_packet(packet);
		paginate_packet(&mut ctx, packet)?;
	}

	if flags & CONTAINS_LAST_PAGE_OF_BITSTREAM == 0x04 {
		if let Some(last) = ctx.pages.last_mut() {
			last.header.header_type_flag |= CONTAINS_LAST_PAGE_OF_BITSTREAM;
		}
	}

	Ok(ctx.pages)
}

fn paginate_packet(ctx: &mut PaginateContext, packet: &[u8]) -> Result<()> {
	let mut page_content = Vec::with_capacity(MAX_WRITTEN_CONTENT_SIZE);
	let mut packet = packet;
	loop {
		if packet.is_empty() {
			break;
		}

		let bytes_read = packet
			.take(ctx.remaining_page_size as u64)
			.read_to_end(&mut page_content)?;
		ctx.remaining_page_size -= bytes_read;

		packet = &packet[bytes_read..];

		if bytes_read <= MAX_WRITTEN_CONTENT_SIZE && packet.is_empty() {
			ctx.flags.packet_finished_on_page = true;
		} else {
			ctx.flags.packet_spans_multiple_pages = true;
		}

		if ctx.remaining_page_size == 0 || packet.is_empty() {
			ctx.flush_page(&mut page_content);
		}

		ctx.flags.first_page = false;
		ctx.flags.fresh_packet = false;
	}

	// Flush any content leftover
	if !page_content.is_empty() {
		ctx.flush_page(&mut page_content);
	}

	Ok(())
}
