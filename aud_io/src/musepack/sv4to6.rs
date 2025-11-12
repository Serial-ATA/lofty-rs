use crate::error::Result;
use crate::musepack::error::MusePackError;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct StreamHeader {
	pub average_bitrate: u32,
	pub intensity_stereo: bool,
	pub mid_side_stereo: bool,
	pub stream_version: u16,
	pub max_band: u8,
	pub block_size: u32,
	pub frame_count: u32,
}

impl StreamHeader {
	pub fn parse(header_data: [u32; 8]) -> Result<StreamHeader> {
		let mut header = Self::default();

		header.average_bitrate = (header_data[0] >> 23) & 0x1FF;
		header.intensity_stereo = (header_data[0] >> 22) & 0x1 == 1;
		header.mid_side_stereo = (header_data[0] >> 21) & 0x1 == 1;

		header.stream_version = ((header_data[0] >> 11) & 0x03FF) as u16;
		if !(4..=6).contains(&header.stream_version) {
			return Err(MusePackError::UnexpectedStreamVersion {
				expected: 4,
				actual: header.stream_version as u8,
			}
			.into());
		}

		header.max_band = ((header_data[0] >> 6) & 0x1F) as u8;
		header.block_size = header_data[0] & 0x3F;

		if header.stream_version >= 5 {
			header.frame_count = header_data[1]; // 32 bit
		} else {
			header.frame_count = header_data[1] >> 16; // 16 bit
		}

		Ok(header)
	}
}
