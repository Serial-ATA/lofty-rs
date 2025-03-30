use super::{Tag, utils};
use crate::config::WriteOptions;
use crate::error::LoftyError;
use crate::file::FileType;
use crate::io::{FileLike, Length, Truncate};
use crate::macros::err;
use crate::probe::Probe;

use std::fs::OpenOptions;
use std::path::Path;

/// The tag's format
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

		if !special_exceptions && !file_type.supports_tag_type(*self) {
			err!(UnsupportedTag);
		}

		let file = probe.into_inner();
		utils::write_tag(&Tag::new(*self), file, file_type, WriteOptions::default()) // TODO
	}
}
