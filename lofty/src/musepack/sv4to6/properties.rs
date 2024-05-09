use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::decode_err;
use crate::musepack::constants::{MPC_DECODER_SYNTH_DELAY, MPC_FRAME_LENGTH};
use crate::properties::FileProperties;
use crate::util::math::RoundedDivision;

use std::io::Read;
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

/// MPC stream versions 4-6 audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MpcSv4to6Properties {
	pub(crate) duration: Duration,
	pub(crate) channels: u8,     // NOTE: always 2
	pub(crate) sample_rate: u32, // NOTE: always 44100

	// Fields actually contained in the header
	pub(crate) average_bitrate: u32,
	pub(crate) mid_side_stereo: bool,
	pub(crate) stream_version: u16,
	pub(crate) max_band: u8,
	pub(crate) frame_count: u32,
}

impl From<MpcSv4to6Properties> for FileProperties {
	fn from(input: MpcSv4to6Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.average_bitrate),
			audio_bitrate: Some(input.average_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: None,
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl MpcSv4to6Properties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Average bitrate (kbps)
	pub fn average_bitrate(&self) -> u32 {
		self.average_bitrate
	}

	/// Whether MidSideStereo is used
	pub fn mid_side_stereo(&self) -> bool {
		self.mid_side_stereo
	}

	/// The MPC stream version (4-6)
	pub fn stream_version(&self) -> u16 {
		self.stream_version
	}

	/// Last subband used in the whole file
	pub fn max_band(&self) -> u8 {
		self.max_band
	}

	/// Total number of audio frames
	pub fn frame_count(&self) -> u32 {
		self.frame_count
	}

	pub(crate) fn read<R>(
		reader: &mut R,
		parse_mode: ParsingMode,
		stream_length: u64,
	) -> Result<Self>
	where
		R: Read,
	{
		let mut header_data = [0u32; 8];
		reader.read_u32_into::<LittleEndian>(&mut header_data)?;

		let mut properties = Self::default();

		properties.average_bitrate = (header_data[0] >> 23) & 0x1FF;
		let intensity_stereo = (header_data[0] >> 22) & 0x1 == 1;
		properties.mid_side_stereo = (header_data[0] >> 21) & 0x1 == 1;

		properties.stream_version = ((header_data[0] >> 11) & 0x03FF) as u16;
		if !(4..=6).contains(&properties.stream_version) {
			decode_err!(@BAIL Mpc, "Invalid stream version encountered")
		}

		properties.max_band = ((header_data[0] >> 6) & 0x1F) as u8;
		let block_size = header_data[0] & 0x3F;

		if properties.stream_version >= 5 {
			properties.frame_count = header_data[1]; // 32 bit
		} else {
			properties.frame_count = header_data[1] >> 16; // 16 bit
		}

		if parse_mode == ParsingMode::Strict {
			if properties.average_bitrate != 0 {
				decode_err!(@BAIL Mpc, "Encountered CBR stream")
			}

			if intensity_stereo {
				decode_err!(@BAIL Mpc, "Stream uses intensity stereo coding")
			}

			if block_size != 1 {
				decode_err!(@BAIL Mpc, "Stream has an invalid block size (must be 1)")
			}
		}

		if properties.stream_version < 6 {
			// Versions before 6 had an invalid last frame
			properties.frame_count = properties.frame_count.saturating_sub(1);
		}

		properties.sample_rate = 44100;
		properties.channels = 2;

		// Nothing more we can do
		if properties.frame_count == 0 {
			return Ok(properties);
		}

		let samples = (u64::from(properties.frame_count) * MPC_FRAME_LENGTH)
			.saturating_sub(MPC_DECODER_SYNTH_DELAY);
		let length = (samples * 1000).div_round(u64::from(properties.sample_rate));
		properties.duration = Duration::from_millis(length);

		// 576 is a magic number from the reference decoder
		//
		// Quote from the reference source (libmpcdec/trunk/src/streaminfo.c:248 @rev 153):
		// "estimation, exact value needs too much time"
		let pcm_frames = (MPC_FRAME_LENGTH * u64::from(properties.frame_count)).saturating_sub(576);

		// Is this accurate? If not, it really doesn't matter.
		properties.average_bitrate = ((stream_length as f64
			* 8.0 * f64::from(properties.sample_rate))
			/ (pcm_frames as f64)
			/ (MPC_FRAME_LENGTH as f64)) as u32;

		Ok(properties)
	}
}
