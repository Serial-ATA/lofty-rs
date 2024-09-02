use crate::error::{LoftyError, Result};
use crate::macros::decode_err;

/// The type of the target.
///
/// This is used to determine the type of the target that the tag is applied to.
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum TargetType {
	/// For video, this represents: SHOT
	Shot = 10,
	/// This is used to represent the following:
	///
	/// - Audio: SUBTRACK / PART / MOVEMENT
	/// - Video: SCENE
	Scene = 20,
	/// This is used to represent the following:
	///
	/// - Audio: TRACK / SONG
	/// - Video: CHAPTER
	Track = 30,
	/// For both audio and video, this represents: PART / SESSION
	Part = 40,
	/// This is used to represent the following:
	///
	/// - Audio: ALBUM / OPERA / CONCERT
	/// - Video: MOVIE / EPISODE / CONCERT
	Album = 50,
	/// This is used to represent the following:
	///
	/// - Audio: EDITION / ISSUE / VOLUME / OPUS
	/// - Video: SEASON / SEQUEL / VOLUME
	Edition = 60,
	/// For both audio and video, this represents: COLLECTION
	Collection = 70,
}

impl TryFrom<u8> for TargetType {
	type Error = LoftyError;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			10 => Ok(Self::Shot),
			20 => Ok(Self::Scene),
			30 => Ok(Self::Track),
			40 => Ok(Self::Part),
			50 => Ok(Self::Album),
			60 => Ok(Self::Edition),
			70 => Ok(Self::Collection),
			_ => decode_err!(@BAIL Ebml, "TargetType value out of range"),
		}
	}
}
