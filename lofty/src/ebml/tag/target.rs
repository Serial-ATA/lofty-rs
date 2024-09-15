use crate::error::{LoftyError, Result};
use crate::macros::decode_err;

/// The type of the target.
///
/// This is used to determine the type of the target that the tag is applied to.
#[repr(u8)]
#[non_exhaustive]
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
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
	// The spec defines TargetType 50 (Album) as the default value, as it is the most
	// common grouping level.
	#[default]
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

/// The target for which a [`SimpleTag`] is applied.
///
/// In Matroska, tags are specified on the level of targets. For example, there is no "TRACK TITLE"
/// tag, but rather a "TITLE" tag that is applied to a [`TargetType::Track`] target.
///
/// See [`TargetType`] for more information on the types of targets.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
	/// The type of the target.
	pub target_type: TargetType,
	/// An informational string that can be used to display the logical level of the target.
	pub name: Option<String>,
	/// A unique ID to identify the [Track](s) the tags belong to.
	///
	/// If the value is 0 at this level, the tags apply to all tracks in the Segment. If set to any
	/// other value, it **MUST** match the [`TrackUID`] value of a track found in this Segment.
	pub track_uids: Option<Vec<u64>>,
	/// A unique ID to identify the [EditionEntry](s) the tags belong to.
	///
	/// If the value is 0 at this level, the tags apply to all editions in the Segment. If set to
	/// any other value, it **MUST** match the [`EditionUID`] value of an edition found in this Segment.
	pub edition_uids: Option<Vec<u64>>,
	/// A unique ID to identify the [Chapter](s) the tags belong to.
	///
	/// If the value is 0 at this level, the tags apply to all chapters in the Segment. If set to
	/// any other value, it **MUST** match the [`ChapterUID`] value of a chapter found in this Segment.
	pub chapter_uids: Option<Vec<u64>>,
	/// A unique ID to identify the [`AttachedFile`]\(s) the tags belong to.
	///
	/// If the value is 0 at this level, the tags apply to all the attachments in the Segment. If
	/// set to any other value, it **MUST** match the [`AttachedFile::uid`]) value of an attachment
	/// found in this Segment.
	///
	/// [`AttachedFile`]: crate::ebml::AttachedFile
	/// [`AttachedFile::uid`]: crate::ebml::AttachedFile::uid
	pub attachment_uids: Option<Vec<u64>>,
}
