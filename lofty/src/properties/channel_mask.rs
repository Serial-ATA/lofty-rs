use std::ops::{BitAnd, BitOr};

macro_rules! define_channels {
	([
		$(
			$(#[$meta:meta])?
			$name:ident => $shift:literal
		),+
	]) => {
		impl ChannelMask {
			$(
				$(#[$meta])?
				pub const $name: Self = Self(1 << $shift);
			)+
		}
	};
}

/// Channel mask
///
/// A mask of (at least) 18 bits, one for each channel.
///
/// * Standard speaker channels: <https://en.wikipedia.org/wiki/Surround_sound#Channel_notation>
/// * CAF channel bitmap: <https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_spec/CAF_spec.html#//apple_ref/doc/uid/TP40001862-CH210-BCGBHHHI>
/// * WAV default channel ordering: <https://learn.microsoft.com/en-us/previous-versions/windows/hardware/design/dn653308(v=vs.85)>
/// * FFmpeg: <https://ffmpeg.org/doxygen/trunk/group__channel__masks.html>
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[repr(transparent)]
pub struct ChannelMask(pub(crate) u32);

define_channels! {
	[
		/// Front left speaker
		FRONT_LEFT            => 0,
		/// Front right speaker
		FRONT_RIGHT           => 1,
		/// Front center speaker
		FRONT_CENTER          => 2,
		/// Low frequency effects (subwoofer)
		LOW_FREQUENCY         => 3,
		/// Back left speaker (surround left)
		BACK_LEFT             => 4,
		/// Back right speaker (surround right)
		BACK_RIGHT            => 5,
		/// Front left of center speaker
		FRONT_LEFT_OF_CENTER  => 6,
		/// Front right of center speaker
		FRONT_RIGHT_OF_CENTER => 7,
		/// Back center speaker
		BACK_CENTER           => 8,
		/// Side left speaker
		SIDE_LEFT             => 9,
		/// Side right speaker
		SIDE_RIGHT            => 10,
		/// Top center speaker
		TOP_CENTER            => 11,
		/// Top front left speaker
		TOP_FRONT_LEFT        => 12,
		/// Top front center speaker
		TOP_FRONT_CENTER      => 13,
		/// Top front right speaker
		TOP_FRONT_RIGHT       => 14,
		/// Top back left speaker
		TOP_BACK_LEFT         => 15,
		/// Top back center speaker
		TOP_BACK_CENTER       => 16,
		/// Top back right speaker
		TOP_BACK_RIGHT        => 17
	]
}

impl ChannelMask {
	/// A single front center channel
	#[must_use]
	pub const fn mono() -> Self {
		Self::FRONT_CENTER
	}

	/// Front left+right channels
	#[must_use]
	pub const fn stereo() -> Self {
		// TODO: #![feature(const_trait_impl)]
		Self(Self::FRONT_LEFT.0 | Self::FRONT_RIGHT.0)
	}

	/// Front left+right+center channels
	#[must_use]
	pub const fn linear_surround() -> Self {
		Self(Self::FRONT_LEFT.0 | Self::FRONT_RIGHT.0 | Self::FRONT_CENTER.0)
	}

	/// The bit mask
	#[must_use]
	pub const fn bits(self) -> u32 {
		self.0
	}

	/// Create a channel mask from the number of channels in an Opus file
	///
	/// See <https://datatracker.ietf.org/doc/html/rfc7845#section-5.1.1.2> for the mapping.
	pub const fn from_opus_channels(channels: u8) -> Option<Self> {
		match channels {
			1 => Some(Self::mono()),
			2 => Some(Self::stereo()),
			3 => Some(Self::linear_surround()),
			4 => Some(Self(
				Self::FRONT_LEFT.bits()
					| Self::FRONT_RIGHT.bits()
					| Self::BACK_LEFT.bits()
					| Self::BACK_RIGHT.bits(),
			)),
			5 => Some(Self(
				Self::linear_surround().bits() | Self::BACK_LEFT.bits() | Self::BACK_RIGHT.bits(),
			)),
			6 => Some(Self(
				Self::linear_surround().bits()
					| Self::BACK_LEFT.bits()
					| Self::BACK_RIGHT.bits()
					| Self::LOW_FREQUENCY.bits(),
			)),
			7 => Some(Self(
				Self::linear_surround().bits()
					| Self::SIDE_LEFT.bits()
					| Self::SIDE_RIGHT.bits()
					| Self::BACK_CENTER.bits()
					| Self::LOW_FREQUENCY.bits(),
			)),
			8 => Some(Self(
				Self::linear_surround().bits()
					| Self::SIDE_LEFT.bits()
					| Self::SIDE_RIGHT.bits()
					| Self::BACK_LEFT.bits()
					| Self::BACK_RIGHT.bits()
					| Self::LOW_FREQUENCY.bits(),
			)),
			_ => None,
		}
	}

	/// Create a channel mask from the number of channels in an MP4 file
	///
	/// See <https://wiki.multimedia.cx/index.php/MPEG-4_Audio#Channel_Configurations> for the mapping.
	pub const fn from_mp4_channels(channels: u8) -> Option<Self> {
		match channels {
			1 => Some(Self::mono()),
			2 => Some(Self::stereo()),
			3 => Some(Self::linear_surround()),
			4 => Some(Self(
				Self::FRONT_LEFT.bits()
					| Self::FRONT_RIGHT.bits()
					| Self::BACK_LEFT.bits()
					| Self::BACK_RIGHT.bits(),
			)),
			5 => Some(Self(
				Self::linear_surround().bits() | Self::BACK_LEFT.bits() | Self::BACK_RIGHT.bits(),
			)),
			6 => Some(Self(
				Self::linear_surround().bits()
					| Self::BACK_LEFT.bits()
					| Self::BACK_RIGHT.bits()
					| Self::LOW_FREQUENCY.bits(),
			)),
			7 => Some(Self(
				Self::linear_surround().bits()
					| Self::SIDE_LEFT.bits()
					| Self::SIDE_RIGHT.bits()
					| Self::BACK_LEFT.bits()
					| Self::BACK_RIGHT.bits()
					| Self::LOW_FREQUENCY.bits(),
			)),
			_ => None,
		}
	}
}

impl BitOr for ChannelMask {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self {
		Self(self.0 | rhs.0)
	}
}

impl BitAnd for ChannelMask {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self {
		Self(self.0 & rhs.0)
	}
}
