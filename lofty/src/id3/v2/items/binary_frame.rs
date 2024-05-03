use crate::error::Result;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};

use std::io::Read;

/// A binary fallback for all unknown `ID3v2` frames
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct BinaryFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The binary data
	pub data: Vec<u8>,
}

impl<'a> BinaryFrame<'a> {
	/// Create a new [`BinaryFrame`]
	pub fn new(id: FrameId<'a>, data: Vec<u8>) -> Self {
		let header = FrameHeader::new(id, FrameFlags::default());
		Self { header, data }
	}

	/// Get the ID for the frame
	pub fn id(&self) -> &FrameId<'_> {
		&self.header.id
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read a [`BinaryFrame`]
	///
	/// NOTE: This will exhaust the entire reader
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	pub fn parse<R>(reader: &mut R, id: FrameId<'a>, frame_flags: FrameFlags) -> Result<Self>
	where
		R: Read,
	{
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;

		let header = FrameHeader::new(id, frame_flags);
		Ok(BinaryFrame { header, data })
	}

	/// Convert an [`BinaryFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let Self { data, .. } = self;
		data.clone()
	}
}
