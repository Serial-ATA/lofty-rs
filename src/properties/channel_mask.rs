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
				#[allow(missing_docs)]
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
		FRONT_LEFT            => 0,
		FRONT_RIGHT           => 1,
		FRONT_CENTER          => 2,
		LOW_FREQUENCY         => 3,
		BACK_LEFT             => 4,
		BACK_RIGHT            => 5,
		FRONT_LEFT_OF_CENTER  => 6,
		FRONT_RIGHT_OF_CENTER => 7,
		BACK_CENTER           => 8,
		SIDE_LEFT             => 9,
		SIDE_RIGHT            => 10,
		TOP_CENTER            => 11,
		TOP_FRONT_LEFT        => 12,
		TOP_FRONT_CENTER      => 13,
		TOP_FRONT_RIGHT       => 14,
		TOP_BACK_LEFT         => 15,
		TOP_BACK_CENTER       => 16,
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

	/// The bit mask
	#[must_use]
	pub const fn bits(self) -> u32 {
		self.0
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
