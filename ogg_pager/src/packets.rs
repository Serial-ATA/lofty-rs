use crate::Page;
use crate::error::{PageError, Result};
use crate::header::PageHeader;
use crate::paginate::paginate;

use std::fmt::{Debug, Formatter};
use std::io::{Read, Seek, Write};

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
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
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
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// // We know that the file has at least 2 packets in it
	/// let packets = Packets::read_count(&mut file, 2)?;
	/// # Ok(()) }
	/// ```
	#[allow(clippy::read_zero_byte_vec)]
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
		let mut packet_bytes_already_read = None;
		let mut current_packet_content;
		'outer: loop {
			if let Ok(header) = PageHeader::read(data) {
				for i in header.segments {
					packet_size += u64::from(i);

					if i < 255 {
						if count != -1 {
							read += 1;
						}

						let byte_count_to_read = Self::get_byte_count_to_read(
							packet_size,
							&mut packet_bytes_already_read,
						);

						current_packet_content = vec![0; byte_count_to_read as usize];
						data.read_exact(&mut current_packet_content)?;

						packet_sizes.push(packet_size);
						packet_size = 0;
						packet_bytes_already_read = None;

						content.append(&mut current_packet_content);

						if read == count {
							break 'outer;
						}
					}
				}

				// The packet continues on the next page, write what we can so far
				if packet_size != 0 {
					let byte_count_to_read =
						Self::get_byte_count_to_read(packet_size, &mut packet_bytes_already_read);

					current_packet_content = vec![0; byte_count_to_read as usize];
					data.read_exact(&mut current_packet_content)?;
					content.append(&mut current_packet_content);
				}

				continue;
			}

			break;
		}

		if count != -1 && packet_sizes.len() != count as usize {
			return Err(PageError::NotEnoughData);
		}

		Ok(Self {
			content,
			packet_sizes,
		})
	}

	fn get_byte_count_to_read(
		packet_size: u64,
		packet_bytes_already_read: &mut Option<u64>,
	) -> u64 {
		let byte_count_to_read;
		match packet_bytes_already_read {
			Some(already_read_bytes_count) => {
				byte_count_to_read = packet_size - *already_read_bytes_count;
				*packet_bytes_already_read = Some(*already_read_bytes_count + byte_count_to_read);
			},
			None => {
				byte_count_to_read = packet_size;
				*packet_bytes_already_read = Some(packet_size);
			},
		}

		byte_count_to_read
	}

	/// Returns the number of packets
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// // I want to read 2 packets
	/// let packets = Packets::read_count(&mut file, 2)?;
	///
	/// // And that's what I received!
	/// assert_eq!(packets.len(), 2);
	/// # Ok(()) }
	/// ```
	pub fn len(&self) -> usize {
		self.packet_sizes.len()
	}

	/// Returns true if there are no packets
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let packets = Packets::read(&mut file)?;
	///
	/// // My file contains packets!
	/// assert!(!packets.is_empty());
	/// # Ok(()) }
	/// ```
	pub fn is_empty(&self) -> bool {
		self.packet_sizes.is_empty()
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
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
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
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let mut packets = Packets::read(&mut file)?;
	///
	/// let new_content = [0; 100];
	///
	/// assert_ne!(packets.get(0), Some(new_content.as_slice()));
	///
	/// // Set our new content
	/// assert!(packets.set(0, new_content));
	///
	/// // Now our packet contains the new content
	/// assert_eq!(packets.get(0), Some(new_content.as_slice()));
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

	/// Returns an iterator over the packets
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::Packets;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let packets = Packets::read(&mut file)?;
	///
	/// for packet in packets.iter() {
	/// 	println!("Packet size: {}", packet.len());
	/// }
	/// # Ok(()) }
	pub fn iter(&self) -> PacketsIter<'_> {
		<&Self as IntoIterator>::into_iter(self)
	}

	/// Convert the packets into a stream of pages
	///
	/// See [paginate()] for more information.
	///
	/// # Errors
	///
	/// See [`paginate()`]
	///
	/// # Examples
	///
	/// ```rust
	/// use ogg_pager::{CONTAINS_FIRST_PAGE_OF_BITSTREAM, CONTAINS_LAST_PAGE_OF_BITSTREAM, Packets};
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// # let path = "../lofty/tests/files/assets/minimal/full_test.ogg";
	/// let mut file = std::fs::File::open(path)?;
	///
	/// let mut packets = Packets::read(&mut file)?;
	///
	/// let stream_serial_number = 1234;
	/// let absolute_granule_position = 0;
	/// let flags = CONTAINS_FIRST_PAGE_OF_BITSTREAM | CONTAINS_LAST_PAGE_OF_BITSTREAM;
	///
	/// let pages = packets.paginate(stream_serial_number, absolute_granule_position, flags)?;
	///
	/// println!("We created {} pages!", pages.len());
	/// # Ok(()) }
	/// ```
	pub fn paginate(&self, stream_serial: u32, abgp: u64, flags: u8) -> Result<Vec<Page>> {
		let mut packets = Vec::new();

		let mut pos = 0;
		for packet_size in self.packet_sizes.iter().copied() {
			packets.push(&self.content[pos..pos + packet_size as usize]);
			pos += packet_size as usize;
		}

		paginate(packets, stream_serial, abgp, flags)
	}

	/// Write packets to a writer
	///
	/// This will paginate and write all of the packets to a writer.
	///
	/// # Errors
	///
	/// * Unable to write, see [`std::io::Error`]
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use ogg_pager::{CONTAINS_FIRST_PAGE_OF_BITSTREAM, CONTAINS_LAST_PAGE_OF_BITSTREAM, Packets};
	/// use std::fs::OpenOptions;
	///
	/// # fn main() -> Result<(), ogg_pager::PageError> {
	/// let mut file = std::fs::File::open("foo.ogg")?;
	///
	/// let mut packets = Packets::read(&mut file)?;
	///
	/// let stream_serial_number = 1234;
	/// let absolute_granule_position = 0;
	/// let flags = CONTAINS_FIRST_PAGE_OF_BITSTREAM | CONTAINS_LAST_PAGE_OF_BITSTREAM;
	///
	/// let mut new_file = OpenOptions::new().write(true).open("bar.ogg")?;
	/// let pages_written = packets.write_to(
	/// 	&mut new_file,
	/// 	stream_serial_number,
	/// 	absolute_granule_position,
	/// 	flags,
	/// )?;
	///
	/// println!("We wrote {} pages!", pages_written);
	/// # Ok(()) }
	/// ```
	pub fn write_to<W>(
		&self,
		writer: &mut W,
		stream_serial: u32,
		abgp: u64,
		flags: u8,
	) -> Result<usize>
	where
		W: Write,
	{
		let paginated = self.paginate(stream_serial, abgp, flags)?;
		let num_pages = paginated.len();

		for mut page in paginated {
			page.gen_crc();
			writer.write_all(&page.as_bytes())?;
		}

		Ok(num_pages)
	}
}

/// An iterator over packets
///
/// This is created by calling `into_iter` on [`Packets`]
#[derive(Clone, PartialEq, Eq, Debug)]
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

impl Debug for Packets {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Packets")
			.field("total_bytes", &self.content.len())
			.field("count", &self.packet_sizes.len())
			.finish()
	}
}
