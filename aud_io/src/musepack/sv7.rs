use super::constants::{FREQUENCY_TABLE, MPC_OLD_GAIN_REF};
use crate::error::Result;
use crate::musepack::error::MusePackError;

use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

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

impl TryFrom<u8> for Profile {
	type Error = ();

	#[rustfmt::skip]
	fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
		match value {
			0         => Ok(Self::None),
			1         => Ok(Self::Unstable),
			2 | 3 | 4 => Ok(Self::Unused),
			5         => Ok(Self::BelowTelephone0),
			6         => Ok(Self::BelowTelephone1),
			7         => Ok(Self::Telephone),
			8         => Ok(Self::Thumb),
			9         => Ok(Self::Radio),
			10        => Ok(Self::Standard),
			11        => Ok(Self::Xtreme),
			12        => Ok(Self::Insane),
			13        => Ok(Self::BrainDead),
			14        => Ok(Self::AboveBrainDead9),
			15        => Ok(Self::AboveBrainDead10),
			_         => Err(()),
		}
	}
}

/// Volume description for the start and end of the title
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Link {
	/// Title starts or ends with a very low level (no live or classical genre titles)
	#[default]
	VeryLowStartOrEnd = 0,
	/// Title ends loudly
	LoudEnd = 1,
	/// Title starts loudly
	LoudStart = 2,
	/// Title starts loudly and ends loudly
	LoudStartAndEnd = 3,
}

impl TryFrom<u8> for Link {
	type Error = ();

	fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::VeryLowStartOrEnd),
			1 => Ok(Self::LoudEnd),
			2 => Ok(Self::LoudStart),
			3 => Ok(Self::LoudStartAndEnd),
			_ => Err(()),
		}
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct StreamHeader {
	pub channels: u8,

	// Section 1
	pub frame_count: u32,

	// Section 2
	pub intensity_stereo: bool,
	pub mid_side_stereo: bool,
	pub max_band: u8,
	pub profile: Profile,
	pub link: Link,
	pub sample_frequency: u32,
	pub max_level: u16,

	// Section 3
	pub replaygain_title_peak: u16,
	pub replaygain_title_gain: i16,

	// Section 4
	pub replaygain_album_peak: u16,
	pub replaygain_album_gain: i16,

	// Section 5
	pub true_gapless: bool,
	pub last_frame_length: u16,
	pub fast_seeking_safe: bool,

	// Section 6
	pub encoder_version: u8,
}

impl StreamHeader {
	pub fn parse<R>(reader: &mut R) -> Result<Self>
	where
		R: Read,
	{
		let version = reader.read_u8()? & 0x0F;
		if version != 7 {
			return Err(MusePackError::UnexpectedStreamVersion {
				expected: 7,
				actual: version,
			}
			.into());
		}

		let mut header = Self {
			channels: 2, // Always 2 channels
			..Self::default()
		};

		// TODO: Make a Bitreader, would be nice crate-wide but especially here
		// The SV7 header is split into 6 32-bit sections

		// -- Section 1 --
		header.frame_count = reader.read_u32::<LittleEndian>()?;

		// -- Section 2 --
		let chunk = reader.read_u32::<LittleEndian>()?;

		let byte1 = ((chunk & 0xFF00_0000) >> 24) as u8;

		header.intensity_stereo = ((byte1 & 0x80) >> 7) == 1;
		header.mid_side_stereo = ((byte1 & 0x40) >> 6) == 1;
		header.max_band = byte1 & 0x3F;

		let byte2 = ((chunk & 0xFF_0000) >> 16) as u8;

		header.profile = Profile::try_from((byte2 & 0xF0) >> 4).unwrap(); // Infallible
		header.link = Link::try_from((byte2 & 0x0C) >> 2).unwrap(); // Infallible

		let sample_freq_index = byte2 & 0x03;
		header.sample_frequency = FREQUENCY_TABLE[sample_freq_index as usize];

		let remaining_bytes = (chunk & 0xFFFF) as u16;
		header.max_level = remaining_bytes;

		// -- Section 3 --
		let title_peak = reader.read_u16::<LittleEndian>()?;
		let title_gain = reader.read_u16::<LittleEndian>()?;

		// -- Section 4 --
		let album_peak = reader.read_u16::<LittleEndian>()?;
		let album_gain = reader.read_u16::<LittleEndian>()?;

		// -- Section 5 --
		let chunk = reader.read_u32::<LittleEndian>()?;

		header.true_gapless = (chunk >> 31) == 1;

		if header.true_gapless {
			header.last_frame_length = ((chunk >> 20) & 0x7FF) as u16;
		}

		header.fast_seeking_safe = (chunk >> 19) & 1 == 1;

		// NOTE: Rest of the chunk is zeroed and unused

		// -- Section 6 --
		header.encoder_version = reader.read_u8()?;

		// -- End of parsing --

		// Convert ReplayGain values
		let set_replay_gain = |gain: u16| -> i16 {
			if gain == 0 {
				return 0;
			}

			let gain = ((MPC_OLD_GAIN_REF - f32::from(gain) / 100.0) * 256.0 + 0.5) as i16;
			if !(0..i16::MAX).contains(&gain) {
				return 0;
			}
			gain
		};
		let set_replay_peak = |peak: u16| -> u16 {
			if peak == 0 {
				return 0;
			}

			((f64::from(peak).log10() * 20.0 * 256.0) + 0.5) as u16
		};

		header.replaygain_title_gain = set_replay_gain(title_gain);
		header.replaygain_title_peak = set_replay_peak(title_peak);
		header.replaygain_album_gain = set_replay_gain(album_gain);
		header.replaygain_album_peak = set_replay_peak(album_peak);

		Ok(header)
	}
}
