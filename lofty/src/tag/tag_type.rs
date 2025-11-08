use super::{Tag, utils};
use crate::config::WriteOptions;
use crate::error::LoftyError;
use crate::file::FileType;
use crate::io::{FileLike, Length, Truncate};
use crate::macros::err;
use crate::probe::Probe;

use std::fs::OpenOptions;
use std::path::Path;

/// Describes how a [`TagType`] is supported in a given [`FileType`]
///
/// See [`FileType::tag_support()`]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TagSupport {
	/// The tag isn't supported in this [`FileType`]
	Unsupported,
	/// The tag type can be *read* from this [`FileType`], but cannot be written back to it.
	///
	/// For example, ID3v2 tags can be read from, but not written to FLAC files.
	ReadOnly,
	/// The tag type can be both read from and written to this [`FileType`].
	ReadWrite,
}

impl TagSupport {
	/// Whether the tag type can be read from the file.
	///
	/// This is `true` for both [`TagSupport::ReadOnly`] and [`TagSupport::ReadWrite`].
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::tag::TagType;
	///
	/// // APE files support reading and writing APE tags
	/// assert!(FileType::Ape.tag_support(TagType::Ape).is_readable());
	/// assert!(FileType::Ape.tag_support(TagType::Ape).is_writable());
	///
	/// // FLAC files only support *reading* ID3v2
	/// assert!(FileType::Flac.tag_support(TagType::Id3v2).is_readable());
	/// assert!(!FileType::Flac.tag_support(TagType::Id3v2).is_writable());
	///
	/// // And WAV files don't support Vorbis Comments at all
	/// assert!(
	/// 	!FileType::Wav
	/// 		.tag_support(TagType::VorbisComments)
	/// 		.is_readable()
	/// );
	/// ```
	pub fn is_readable(self) -> bool {
		matches!(self, Self::ReadOnly | Self::ReadWrite)
	}

	/// Whether the tag type can be written to the file.
	///
	/// This is only `true` for [`TagSupport::ReadWrite`].
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::tag::TagType;
	///
	/// // APE files support reading and writing APE tags
	/// assert!(FileType::Ape.tag_support(TagType::Ape).is_readable());
	/// assert!(FileType::Ape.tag_support(TagType::Ape).is_writable());
	///
	/// // FLAC files only support *reading* ID3v2
	/// assert!(FileType::Flac.tag_support(TagType::Id3v2).is_readable());
	/// assert!(!FileType::Flac.tag_support(TagType::Id3v2).is_writable());
	///
	/// // And WAV files don't support Vorbis Comments at all
	/// assert!(
	/// 	!FileType::Wav
	/// 		.tag_support(TagType::VorbisComments)
	/// 		.is_readable()
	/// );
	/// assert!(
	/// 	!FileType::Wav
	/// 		.tag_support(TagType::VorbisComments)
	/// 		.is_writable()
	/// );
	/// ```
	pub fn is_writable(self) -> bool {
		matches!(self, Self::ReadWrite)
	}
}

/// The tag's format
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(clippy::unsafe_derive_deserialize)]
#[non_exhaustive]
pub enum TagType {
	/// This covers both APEv1 and APEv2 as it doesn't matter much
	Ape,
	/// Represents an ID3v1 tag
	Id3v1,
	/// This covers all ID3v2 versions since they all get upgraded to ID3v2.4
	Id3v2,
	/// Represents an MP4 ilst atom
	Mp4Ilst,
	/// Represents vorbis comments
	VorbisComments,
	/// Represents a RIFF INFO LIST
	RiffInfo,
	/// Represents AIFF text chunks
	AiffText,
}

impl TagType {
	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagType::remove_from`]
	pub fn remove_from_path(&self, path: impl AsRef<Path>) -> crate::error::Result<()> {
		let mut file = OpenOptions::new().read(true).write(true).open(path)?;
		self.remove_from(&mut file)
	}

	#[allow(clippy::shadow_unrelated)]
	/// Remove a tag from a [`FileLike`]
	///
	/// # Errors
	///
	/// * It is unable to guess the file format
	/// * The format doesn't support the tag
	/// * It is unable to write to the file
	pub fn remove_from<F>(&self, file: &mut F) -> crate::error::Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		let probe = Probe::new(file).guess_file_type()?;
		let Some(file_type) = probe.file_type() else {
			err!(UnknownFormat);
		};

		// TODO: This should not have to be manually updated
		let special_exceptions = ((file_type == FileType::Ape
			|| file_type == FileType::Mpc
			|| file_type == FileType::Flac)
			&& *self == TagType::Id3v2)
			|| file_type == FileType::Mpc && *self == TagType::Id3v1;

		if !special_exceptions && !file_type.tag_support(*self).is_readable() {
			err!(UnsupportedTag);
		}

		let file = probe.into_inner();
		utils::write_tag(&Tag::new(*self), file, file_type, WriteOptions::default()) // TODO
	}
}
