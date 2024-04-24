use crate::{PageError, Result};

use std::io::{Read, Seek};

use byteorder::{LittleEndian, ReadBytesExt};

/// The size of an OGG page header
pub const PAGE_HEADER_SIZE: usize = 27;

/// An OGG page header
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PageHeader {
	/// The position in the stream the page started at
	pub start: u64,
	pub(crate) header_type_flag: u8,
	/// The page's absolute granule position
	pub abgp: u64,
	/// The page's stream serial number
	pub stream_serial: u32,
	/// The page's sequence number
	pub sequence_number: u32,
	pub(crate) segments: Vec<u8>,
	pub(crate) checksum: u32,
}

impl PageHeader {
	/// Creates a new `PageHeader`
	#[must_use]
	pub const fn new(
		header_type_flag: u8,
		abgp: u64,
		stream_serial: u32,
		sequence_number: u32,
	) -> Self {
		Self {
			start: 0,
			header_type_flag,
			abgp,
			stream_serial,
			sequence_number,
			segments: Vec::new(),
			checksum: 0,
		}
	}

	/// Reads a `PageHeader` from a reader
	///
	/// # Errors
	///
	/// * [`PageError::MissingMagic`]
	/// * [`PageError::InvalidVersion`]
	/// * [`PageError::BadSegmentCount`]
	/// * Reader does not have enough data
	pub fn read<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let start = data.stream_position()?;

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

		let header_type_flag = data.read_u8()?;

		let abgp = data.read_u64::<LittleEndian>()?;
		let stream_serial = data.read_u32::<LittleEndian>()?;
		let sequence_number = data.read_u32::<LittleEndian>()?;
		let checksum = data.read_u32::<LittleEndian>()?;

		let segments = data.read_u8()?;

		if segments < 1 {
			return Err(PageError::BadSegmentCount);
		}

		let mut segment_table = vec![0; segments as usize];
		data.read_exact(&mut segment_table)?;

		let header = Self {
			start,
			header_type_flag,
			abgp,
			stream_serial,
			sequence_number,
			segments: segment_table,
			checksum,
		};

		Ok(header)
	}

	/// Returns the size of the page content, excluding the header
	pub fn content_size(&self) -> usize {
		self.segments.iter().map(|&b| usize::from(b)).sum::<usize>()
	}

	/// Returns the page's header type flag
	pub fn header_type_flag(&self) -> u8 {
		self.header_type_flag
	}

	/// Returns the page's checksum
	pub fn checksum(&self) -> u32 {
		self.checksum
	}
}
