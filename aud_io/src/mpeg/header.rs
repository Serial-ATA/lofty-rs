use super::constants::{BITRATES, PADDING_SIZES, SAMPLE_RATES, SAMPLES, SIDE_INFORMATION_SIZES};
use super::error::{MpegFrameError, VbrHeaderError};

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

/// MPEG Audio version
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum MpegVersion {
	#[default]
	V1,
	V2,
	V2_5,
	/// Exclusive to AAC
	V4,
}

/// MPEG layer
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Layer {
	Layer1 = 1,
	Layer2 = 2,
	#[default]
	Layer3 = 3,
}

/// Channel mode
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum ChannelMode {
	#[default]
	Stereo = 0,
	JointStereo = 1,
	/// Two independent mono channels
	DualChannel = 2,
	SingleChannel = 3,
}

/// A rarely-used decoder hint that the file must be de-emphasized
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs, non_camel_case_types)]
pub enum Emphasis {
	/// 50/15 ms
	MS5015,
	Reserved,
	/// CCIT J.17
	CCIT_J17,
}

#[derive(Copy, Clone, Debug)]
pub struct FrameHeader {
	pub sample_rate: u32,
	pub len: u32,
	pub data_start: u32,
	pub samples: u16,
	pub bitrate: u32,
	pub version: MpegVersion,
	pub layer: Layer,
	pub channel_mode: ChannelMode,
	pub mode_extension: Option<u8>,
	pub copyright: bool,
	pub original: bool,
	pub emphasis: Option<Emphasis>,
}

impl FrameHeader {
	pub fn parse(data: u32) -> Result<Self, MpegFrameError> {
		let version = match (data >> 19) & 0b11 {
			0b00 => MpegVersion::V2_5,
			0b10 => MpegVersion::V2,
			0b11 => MpegVersion::V1,
			_ => return Err(MpegFrameError::BadVersion),
		};

		let version_index = if version == MpegVersion::V1 { 0 } else { 1 };

		let layer = match (data >> 17) & 0b11 {
			0b01 => Layer::Layer3,
			0b10 => Layer::Layer2,
			0b11 => Layer::Layer1,
			_ => {
				log::debug!("MPEG: Frame header uses a reserved layer");
				return Err(MpegFrameError::BadLayer);
			},
		};

		let mut header = FrameHeader {
			sample_rate: 0,
			len: 0,
			data_start: 0,
			samples: 0,
			bitrate: 0,
			version,
			layer,
			channel_mode: ChannelMode::default(),
			mode_extension: None,
			copyright: false,
			original: false,
			emphasis: None,
		};

		let layer_index = (header.layer as usize).saturating_sub(1);

		let bitrate_index = (data >> 12) & 0xF;
		header.bitrate = BITRATES[version_index][layer_index][bitrate_index as usize];
		if header.bitrate == 0 {
			return Err(MpegFrameError::BadBitrate);
		}

		// Sample rate index
		let sample_rate_index = (data >> 10) & 0b11;
		header.sample_rate = match sample_rate_index {
			// This is invalid
			0b11 => return Err(MpegFrameError::BadSampleRate),
			_ => SAMPLE_RATES[header.version as usize][sample_rate_index as usize],
		};

		let has_padding = ((data >> 9) & 1) == 1;
		let mut padding = 0;

		if has_padding {
			padding = u32::from(PADDING_SIZES[layer_index]);
		}

		header.channel_mode = match (data >> 6) & 0b11 {
			0b00 => ChannelMode::Stereo,
			0b01 => ChannelMode::JointStereo,
			0b10 => ChannelMode::DualChannel,
			0b11 => ChannelMode::SingleChannel,
			_ => unreachable!(),
		};

		if let ChannelMode::JointStereo = header.channel_mode {
			header.mode_extension = Some(((data >> 4) & 3) as u8);
		} else {
			header.mode_extension = None;
		}

		header.copyright = ((data >> 3) & 1) == 1;
		header.original = ((data >> 2) & 1) == 1;

		header.emphasis = match data & 0b11 {
			0b00 => None,
			0b01 => Some(Emphasis::MS5015),
			0b10 => Some(Emphasis::Reserved),
			0b11 => Some(Emphasis::CCIT_J17),
			_ => unreachable!(),
		};

		header.data_start = SIDE_INFORMATION_SIZES[version_index][header.channel_mode as usize] + 4;
		header.samples = SAMPLES[layer_index][version_index];
		header.len =
			(u32::from(header.samples) * header.bitrate * 125 / header.sample_rate) + padding;

		Ok(header)
	}

	/// Equivalent of [`cmp_header()`], but for an already constructed `Header`.
	pub fn cmp(self, other: &Self) -> bool {
		self.version == other.version
			&& self.layer == other.layer
			&& self.sample_rate == other.sample_rate
	}
}

#[derive(Copy, Clone)]
pub enum VbrHeaderType {
	Xing,
	Info,
	Vbri,
}

#[derive(Copy, Clone)]
pub struct VbrHeader {
	pub ty: VbrHeaderType,
	pub frames: u32,
	pub size: u32,
}

impl VbrHeader {
	pub fn parse<R: Read>(reader: &mut R) -> Result<Self, VbrHeaderError> {
		let mut header = [0; 4];
		reader.read_exact(&mut header)?;

		match &header {
			b"Xing" | b"Info" => {
				let mut flags = [0; 4];
				reader.read_exact(&mut flags)?;

				if flags[3] & 0x03 != 0x03 {
					log::debug!(
						"MPEG: Xing header doesn't have required flags set (0x0001 and 0x0002)"
					);
					return Err(VbrHeaderError::BadXing);
				}

				let frames = reader
					.read_u32::<BigEndian>()
					.map_err(|_| VbrHeaderError::BadXing)?;
				let size = reader
					.read_u32::<BigEndian>()
					.map_err(|_| VbrHeaderError::BadXing)?;

				let ty = match &header {
					b"Xing" => VbrHeaderType::Xing,
					b"Info" => VbrHeaderType::Info,
					_ => unreachable!(),
				};

				Ok(Self { ty, frames, size })
			},
			b"VBRI" => {
				// Skip 6 bytes
				// Version ID (2)
				// Delay float (2)
				// Quality indicator (2)
				let _info = reader
					.read_uint::<BigEndian>(6)
					.map_err(|_| VbrHeaderError::BadVbri)?;

				let size = reader
					.read_u32::<BigEndian>()
					.map_err(|_| VbrHeaderError::BadVbri)?;
				let frames = reader
					.read_u32::<BigEndian>()
					.map_err(|_| VbrHeaderError::BadVbri)?;

				Ok(Self {
					ty: VbrHeaderType::Vbri,
					frames,
					size,
				})
			},
			_ => Err(VbrHeaderError::UnknownHeader),
		}
	}

	pub fn is_valid(&self) -> bool {
		self.frames > 0 && self.size > 0
	}
}
