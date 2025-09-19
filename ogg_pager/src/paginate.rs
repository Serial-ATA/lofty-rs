use crate::error::Result;
use crate::{
	CONTAINS_FIRST_PAGE_OF_BITSTREAM, CONTAINS_LAST_PAGE_OF_BITSTREAM, CONTINUED_PACKET,
	MAX_WRITTEN_CONTENT_SIZE, MAX_WRITTEN_SEGMENT_COUNT, Page, PageHeader,
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
	last_segment_size: Option<u8>,
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
				packet_finished_on_page: false,
			},
			pos: 0,
			idx: 0,
			remaining_page_size: MAX_WRITTEN_CONTENT_SIZE,
			current_packet_len: 0,
			last_segment_size: None,
		}
	}

	fn fresh_packet(&mut self, packet: &[u8]) {
		self.flags.fresh_packet = true;
		self.pos = 0;

		self.current_packet_len = packet.len();
		self.last_segment_size = Some((packet.len() % 255) as u8);
	}

	fn flush_page(&mut self, content: &mut Vec<u8>) {
		let mut header = PageHeader {
			start: self.pos,
			header_type_flag: {
				if self.flags.first_page
					&& self.header_flags & CONTAINS_FIRST_PAGE_OF_BITSTREAM != 0
				{
					CONTAINS_FIRST_PAGE_OF_BITSTREAM
				} else if !self.flags.fresh_packet {
					// A packet from the previous page continues onto this page
					CONTINUED_PACKET
				} else {
					0
				}
			},
			abgp: 0,
			stream_serial: self.stream_serial,
			sequence_number: self.idx as u32,
			segments: Vec::new(),
			// Calculated later
			checksum: 0,
		};

		let content = core::mem::take(content);
		let content_len = content.len();
		self.pos += content_len as u64;

		// Determine how many *full* segments our page content takes up.
		// Anything < 255 will be covered by `last_segment_size`
		let full_segments_occupied = content_len / 255;

		// Moving on to a new packet
		debug_assert!(self.pos <= self.current_packet_len as u64);

		if self.flags.packet_finished_on_page {
			header.abgp = self.abgp;
		} else {
			// A special value of '-1' (in two's complement) indicates that no packets
			// finish on this page.
			header.abgp = 1_u64.wrapping_neg()
		}

		debug_assert!(full_segments_occupied <= MAX_WRITTEN_SEGMENT_COUNT);
		header.segments = vec![255; full_segments_occupied];

		if full_segments_occupied != MAX_WRITTEN_SEGMENT_COUNT {
			// End of the packet
			let last_segment_size = self
				.last_segment_size
				.expect("fresh packet should be indicated at this point");
			header.segments.push(last_segment_size);
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
pub fn paginate<'a, I>(packets: I, stream_serial: u32, abgp: u64, flags: u8) -> Result<Vec<Page>>
where
	I: IntoIterator<Item = &'a [u8]> + 'a,
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

	// After all packet bytes are consumed, check for nil page condition
	// If the packet length is a multiple of 255 * MAX_WRITTEN_SEGMENT_COUNT, we need a nil page

	// A 'nil' page just means it is zero-length. This is used when our packet is perfectly
	// divisible by `255 * MAX_SEGMENT_COUNT`. We need a zero-sized segment to mark the end of our
	// packet across page boundaries.
	//
	// Very rare circumstance, but still possible.
	//
	// From <https://xiph.org/ogg/doc/framing.html>:
	// "Note also that a 'nil' (zero length) packet is not an error; it consists of nothing more than a lacing value of zero in the header."
	if ctx.current_packet_len != 0
		&& ctx.current_packet_len % (255 * MAX_WRITTEN_SEGMENT_COUNT) == 0
	{
		ctx.flags.packet_finished_on_page = true;
		let mut nil_content = Vec::new();
		ctx.flush_page(&mut nil_content);
	}

	Ok(())
}
