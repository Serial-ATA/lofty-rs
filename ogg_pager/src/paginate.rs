use crate::error::Result;
use crate::{
	segment_table, Page, PageHeader, CONTAINS_FIRST_PAGE_OF_BITSTREAM,
	CONTAINS_LAST_PAGE_OF_BITSTREAM, CONTINUED_PACKET, MAX_WRITTEN_CONTENT_SIZE,
	MAX_WRITTEN_SEGMENT_COUNT,
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
		}
	}

	fn flush(&mut self, content: &mut Vec<u8>, segment_table: &mut Vec<u8>) {
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
			// No need to calculate this yet
			checksum: 0,
		};

		let content = core::mem::take(content);
		let content_len = content.len();
		self.pos += content_len as u64;

		// Moving on to a new packet
		if self.pos > self.current_packet_len as u64 {
			self.flags.packet_spans_multiple_pages = false;
		}

		// We need to determine how many segments our page content takes up.
		// If it takes up the remainder of the segment table for the entire packet,
		// we'll just consume it as is.
		let segments_occupied = if content_len >= 255 {
			content_len / 255
		} else {
			1
		};

		if self.flags.packet_spans_multiple_pages {
			header.segments = segment_table.drain(..segments_occupied).collect();
		} else {
			header.segments = core::mem::take(segment_table);
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

	let mut packets_iter = packets.into_iter();
	let mut packet = match packets_iter.next() {
		Some(packet) => packet,
		// We weren't given any content to paginate
		None => return Ok(ctx.pages),
	};
	ctx.current_packet_len = packet.len();

	let mut segments = segment_table(packet.len());
	let mut page_content = Vec::new();

	loop {
		if !ctx.flags.packet_spans_multiple_pages && !ctx.flags.first_page {
			match packets_iter.next() {
				Some(packet_) => {
					packet = packet_;
					segments.append(&mut segment_table(packet.len()));
					ctx.current_packet_len = packet.len();
					ctx.flags.fresh_packet = true;
				},
				None => break,
			};
		}

		// We read as much of the packet as we can, given the amount of space left in the page.
		// The packet may need to span multiple pages.
		let bytes_read = packet
			.take(ctx.remaining_page_size as u64)
			.read_to_end(&mut page_content)?;
		ctx.remaining_page_size -= bytes_read;

		packet = &packet[bytes_read..];
		// We need to indicate whether or not any packet was finished on this page.
		// This is used for the absolute granule position.
		if packet.is_empty() {
			ctx.flags.packet_finished_on_page = true;
		}

		// The first packet of the bitstream must have its own page, unlike any other packet.
		let first_page_of_bitstream = ctx.header_flags & CONTAINS_FIRST_PAGE_OF_BITSTREAM != 0;
		let first_packet_finished_on_page =
			ctx.flags.first_page && first_page_of_bitstream && ctx.flags.packet_finished_on_page;

		// We have a maximum of `MAX_WRITTEN_SEGMENT_COUNT` segments available per page, if we require more than
		// is left in the segment table, we'll have to split the packet into multiple pages.
		let segments_required = (packet.len() / MAX_WRITTEN_SEGMENT_COUNT) + 1;
		let remaining_segments = MAX_WRITTEN_SEGMENT_COUNT.saturating_sub(segments.len());
		ctx.flags.packet_spans_multiple_pages = segments_required > remaining_segments;

		if first_packet_finished_on_page
			// We've completely filled this page, we need to flush before moving on
			|| (ctx.remaining_page_size == 0 || remaining_segments == 0)
			// We've read all this packet has to offer
			|| packet.is_empty()
		{
			ctx.flush(&mut page_content, &mut segments);
		}

		ctx.flags.first_page = false;
		ctx.flags.fresh_packet = false;
	}

	// Flush any content leftover
	if !page_content.is_empty() {
		ctx.flush(&mut page_content, &mut segments);
	}

	if flags & CONTAINS_LAST_PAGE_OF_BITSTREAM == 0x04 {
		if let Some(last) = ctx.pages.last_mut() {
			last.header.header_type_flag |= CONTAINS_LAST_PAGE_OF_BITSTREAM;
		}
	}

	Ok(ctx.pages)
}
