use crate::error::Result;
use crate::probe::ParsingMode;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

/// Used profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
	/// No profile
	#[default]
	None,
	/// Unstable/Experimental
	Unstable,
	/// Profiles 2-4
	Unused,
	/// Below Telephone (q= 0.0)
	BelowTelephone0,
	/// Below Telephone (q= 1.0)
	BelowTelephone1,
	/// Telephone (q= 2.0)
	Telephone,
	/// Thumb (q= 3.0)
	Thumb,
	/// Radio (q= 4.0)
	Radio,
	/// Standard (q= 5.0)
	Standard,
	/// Xtreme (q= 6.0)
	Xtreme,
	/// Insane (q= 7.0)
	Insane,
	/// BrainDead (q= 8.0)
	BrainDead,
	/// Above BrainDead (q= 9.0)
	AboveBrainDead9,
	/// Above BrainDead (q= 10.0)
	AboveBrainDead10,
}

/// Volume description for the start and end of the title
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Link {
	/// Title starts or ends with a very low level (no live or classical genre titles)
	#[default]
	VeryLowStartOrEnd,
	/// Title ends loudly
	LoudEnd,
	/// Title starts loudly
	LoudStart,
	/// Title starts loudly and ends loudly
	LoudStartAndEnd,
}

// http://trac.musepack.net/musepack/wiki/SV7Specification

/// MPC stream version 7 audio properties
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct MpcSv7Properties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) channels: u8, // NOTE: always 2
	// -- Section 1 --
	frame_count: u32,
	// -- Section 2 --
	intensity_stereo: bool,
	mid_side_stereo: bool,
	max_band: u8,
	profile: Profile,
	link: Link,
	sample_freq: u32,
	max_level: u16,
	// -- Section 3 --
	title_gain: i16,
	title_peak: u16,
	// -- Section 4 --
	album_gain: i16,
	album_peak: u16,
	// -- Section 5 --
	true_gapless: bool,
	last_frame_length: u16,
	fast_seeking_safe: bool,
	// -- Section 6 --
	encoder_version: u8,
}

impl From<MpcSv7Properties> for FileProperties {
	fn from(input: MpcSv7Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_freq),
			bit_depth: None,
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl MpcSv7Properties {
	pub(crate) fn read<R>(_reader: &mut R, _parse_mode: ParsingMode) -> Result<Self>
	where
		R: Read,
	{
		todo!()
	}
}
