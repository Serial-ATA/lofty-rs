use super::Tag;

/// Split (and merge) tags.
///
/// Useful and required for implementing lossless read/modify/write round trips.
/// Its counterpart `MergeTag` is used for recombining the results later.
///
/// # Example
///
/// ```rust,no_run
/// use lofty::config::{ParseOptions, WriteOptions};
/// use lofty::mpeg::MpegFile;
/// use lofty::prelude::*;
///
/// // Read the tag from a file
/// # fn main() -> lofty::error::Result<()> {
/// # let mut file = std::fs::OpenOptions::new().write(true).open("/path/to/file.mp3")?;
/// # let parse_options = ParseOptions::default();
/// let mut mpeg_file = <MpegFile as AudioFile>::read_from(&mut file, parse_options)?;
/// let mut id3v2 = mpeg_file
/// 	.id3v2_mut()
/// 	.map(std::mem::take)
/// 	.unwrap_or_default();
///
/// // Split: ID3v2 -> [`lofty::Tag`]
/// let (mut remainder, mut tag) = id3v2.split_tag();
///
/// // Modify the metadata in the generic [`lofty::Tag`], independent
/// // of the underlying tag and file format.
/// tag.insert_text(ItemKey::TrackTitle, "Track Title".to_owned());
/// tag.remove_key(ItemKey::Composer);
///
/// // ID3v2 <- [`lofty::Tag`]
/// let id3v2 = remainder.merge_tag(tag);
///
/// // Write the changes back into the file
/// mpeg_file.set_id3v2(id3v2);
/// mpeg_file.save_to(&mut file, WriteOptions::default())?;
///
/// # Ok(()) }
/// ```
pub trait SplitTag: private::Sealed {
	/// The remainder of the split operation that is not represented
	/// in the resulting `Tag`.
	type Remainder: MergeTag;

	/// Extract and split generic contents into a [`Tag`].
	///
	/// Returns the remaining content that cannot be represented in the
	/// resulting `Tag` in `Self::Remainder`. This is useful if the
	/// modified [`Tag`] is merged later using [`MergeTag::merge_tag`].
	fn split_tag(self) -> (Self::Remainder, Tag);
}

/// The counterpart of [`SplitTag`].
pub trait MergeTag: private::Sealed {
	/// The resulting tag.
	type Merged: SplitTag;

	/// Merge a generic [`Tag`] back into the remainder of [`SplitTag::split_tag`].
	///
	/// Restores the original representation merged with the contents of
	/// `tag` for further processing, e.g. writing back into a file.
	///
	/// Multi-valued items in `tag` with identical keys might get lost
	/// depending on the support for multi-valued fields in `self`.
	fn merge_tag(self, tag: Tag) -> Self::Merged;
}

// https://rust-lang.github.io/api-guidelines/future-proofing.html#c-sealed
mod private {
	use crate::ape::ApeTag;
	use crate::id3::v1::Id3v1Tag;
	use crate::id3::v2::Id3v2Tag;
	use crate::iff::aiff::AiffTextChunks;
	use crate::iff::wav::RiffInfoList;
	use crate::ogg::VorbisComments;
	use crate::tag::Tag;

	pub trait Sealed {}

	impl Sealed for AiffTextChunks {}
	impl Sealed for crate::iff::aiff::tag::SplitTagRemainder {}

	impl Sealed for ApeTag {}
	impl Sealed for crate::ape::tag::SplitTagRemainder {}

	impl Sealed for Id3v1Tag {}
	impl Sealed for crate::id3::v1::tag::SplitTagRemainder {}

	impl Sealed for Id3v2Tag {}
	impl Sealed for crate::id3::v2::tag::SplitTagRemainder {}

	impl Sealed for crate::mp4::Ilst {}
	impl Sealed for crate::mp4::ilst::SplitTagRemainder {}

	impl Sealed for RiffInfoList {}
	impl Sealed for crate::iff::wav::tag::SplitTagRemainder {}

	impl Sealed for Tag {}
	impl Sealed for crate::tag::SplitTagRemainder {}

	impl Sealed for VorbisComments {}
	impl Sealed for crate::ogg::tag::SplitTagRemainder {}
}
