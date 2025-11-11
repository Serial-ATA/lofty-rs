use crate::error::Result;
use crate::macros::decode_err;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

use aud_io::musepack::constants::{MPC_DECODER_SYNTH_DELAY, MPC_FRAME_LENGTH};
use aud_io::musepack::sv7::{Link, Profile, StreamHeader};

// http://trac.musepack.net/musepack/wiki/SV7Specification

/// MPC stream version 7 audio properties
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct MpcSv7Properties {
	pub(crate) duration: Duration,
	pub(crate) average_bitrate: u32,
	pub(crate) channels: u8, // NOTE: always 2
	// -- Section 1 --
	pub(crate) frame_count: u32,
	// -- Section 2 --
	pub(crate) intensity_stereo: bool,
	pub(crate) mid_side_stereo: bool,
	pub(crate) max_band: u8,
	pub(crate) profile: Profile,
	pub(crate) link: Link,
	pub(crate) sample_freq: u32,
	pub(crate) max_level: u16,
	// -- Section 3 --
	pub(crate) title_gain: i16,
	pub(crate) title_peak: u16,
	// -- Section 4 --
	pub(crate) album_gain: i16,
	pub(crate) album_peak: u16,
	// -- Section 5 --
	pub(crate) true_gapless: bool,
	pub(crate) last_frame_length: u16,
	pub(crate) fast_seeking_safe: bool,
	// -- Section 6 --
	pub(crate) encoder_version: u8,
}

impl From<MpcSv7Properties> for FileProperties {
	fn from(input: MpcSv7Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.average_bitrate),
			audio_bitrate: Some(input.average_bitrate),
			sample_rate: Some(input.sample_freq),
			bit_depth: None,
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl From<StreamHeader> for MpcSv7Properties {
	fn from(input: StreamHeader) -> Self {
		Self {
			duration: Duration::ZERO,
			average_bitrate: 0,
			channels: input.channels,
			// -- Section 1 --
			frame_count: input.frame_count,
			// -- Section 2 --
			intensity_stereo: input.intensity_stereo,
			mid_side_stereo: input.mid_side_stereo,
			max_band: input.max_band,
			profile: input.profile,
			link: input.link,
			sample_freq: input.sample_frequency,
			max_level: input.max_level,
			// -- Section 3 --
			title_gain: input.replaygain_title_gain,
			title_peak: input.replaygain_title_peak,
			// -- Section 4 --
			album_gain: input.replaygain_album_gain,
			album_peak: input.replaygain_album_peak,
			// -- Section 5 --
			true_gapless: input.true_gapless,
			last_frame_length: input.last_frame_length,
			fast_seeking_safe: input.fast_seeking_safe,
			// -- Section 6 --
			encoder_version: input.encoder_version,
		}
	}
}

impl MpcSv7Properties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Average bitrate (kbps)
	pub fn average_bitrate(&self) -> u32 {
		self.average_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_freq
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Total number of audio frames
	pub fn frame_count(&self) -> u32 {
		self.frame_count
	}

	/// Whether intensity stereo coding (IS) is used
	pub fn intensity_stereo(&self) -> bool {
		self.intensity_stereo
	}

	/// Whether MidSideStereo is used
	pub fn mid_side_stereo(&self) -> bool {
		self.mid_side_stereo
	}

	/// Last subband used in the whole file
	pub fn max_band(&self) -> u8 {
		self.max_band
	}

	/// Profile used
	pub fn profile(&self) -> Profile {
		self.profile
	}

	/// Volume description of the start and end
	pub fn link(&self) -> Link {
		self.link
	}

	/// Maximum level of the coded PCM input signal
	pub fn max_level(&self) -> u16 {
		self.max_level
	}

	/// Change in the replay level
	///
	/// The value is a signed 16-bit integer, with the level being attenuated by that many mB
	pub fn title_gain(&self) -> i16 {
		self.title_gain
	}

	/// Maximum level of the decoded title
	///
	/// * 16422: -6 dB
	/// * 32767:  0 dB
	/// * 65379: +6 dB
	pub fn title_peak(&self) -> u16 {
		self.title_peak
	}

	/// Change in the replay level if the whole CD is supposed to be played with the same level change
	///
	/// The value is a signed 16-bit integer, with the level being attenuated by that many mB
	pub fn album_gain(&self) -> i16 {
		self.album_gain
	}

	/// Maximum level of the whole decoded CD
	///
	/// * 16422: -6 dB
	/// * 32767:  0 dB
	/// * 65379: +6 dB
	pub fn album_peak(&self) -> u16 {
		self.album_peak
	}

	/// Whether true gapless is used
	pub fn true_gapless(&self) -> bool {
		self.true_gapless
	}

	/// Used samples of the last frame
	///
	/// * TrueGapless = 0: always 0
	/// * TrueGapless = 1: 1...1152
	pub fn last_frame_length(&self) -> u16 {
		self.last_frame_length
	}

	/// Whether fast seeking can be used safely
	pub fn fast_seeking_safe(&self) -> bool {
		self.fast_seeking_safe
	}

	/// Encoder version
	///
	/// * Encoder version * 100  (106 = 1.06)
	/// * EncoderVersion % 10 == 0        Release (1.0)
	/// * EncoderVersion %  2 == 0        Beta (1.06)
	/// * EncoderVersion %  2 == 1        Alpha (1.05a...z)
	pub fn encoder_version(&self) -> u8 {
		self.encoder_version
	}

	pub(crate) fn read<R>(reader: &mut R, stream_length: u64) -> Result<Self>
	where
		R: Read,
	{
		let header = StreamHeader::parse(reader)?;
		if header.last_frame_length > MPC_FRAME_LENGTH as u16 {
			decode_err!(@BAIL Mpc, "Invalid last frame length");
		}

		if header.sample_frequency == 0 {
			log::warn!("Sample rate is 0, unable to calculate duration and bitrate");
			return Ok(header.into());
		}

		if header.frame_count == 0 {
			log::warn!("Frame count is 0, unable to calculate duration and bitrate");
			return Ok(header.into());
		}

		let mut properties = MpcSv7Properties::from(header);

		let time_per_frame = (MPC_FRAME_LENGTH as f64) / f64::from(properties.sample_freq);
		let length = (f64::from(properties.frame_count) * time_per_frame) * 1000.0;
		properties.duration = Duration::from_millis(length as u64);

		let total_samples;
		if properties.true_gapless {
			total_samples = (u64::from(properties.frame_count) * MPC_FRAME_LENGTH)
				- (MPC_FRAME_LENGTH - u64::from(properties.last_frame_length));
		} else {
			total_samples =
				(u64::from(properties.frame_count) * MPC_FRAME_LENGTH) - MPC_DECODER_SYNTH_DELAY;
		}

		properties.average_bitrate = ((stream_length * 8 * u64::from(properties.sample_freq))
			/ (total_samples * 1000)) as u32;

		Ok(properties)
	}
}
