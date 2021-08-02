#[allow(clippy::wildcard_imports)]
use crate::components::tags::*;
use crate::{AudioTag, FileType, LoftyError, Result, Tag, TaggedFile};

use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::ReadBytesExt;
use std::convert::TryInto;
use crate::components::logic::ogg::VorbisFile;
use crate::components::logic::iff::RiffFile;

/// Provides a way to extract a [`FileType`] from a reader
#[derive(Default)]
pub struct Probe(());

impl Probe {
	/// Create a new `Probe`
	pub fn new() -> Self {
		Self::default()
	}

	/// Attempts to get the [`FileType`] based on the file extension
	///
	/// NOTE: Since this only looks at the extension, the result could be incorrect.
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * `path` has no extension
	/// * `path` has an unsupported/unknown extension
	///
	/// # Warning
	/// Using this on a `WAV`/`AIFF`/`MP3` file will **always** assume there's an ID3 tag.
	/// [`guess_from_content`](Probe::guess_from_content) is recommended in the event that other tags are present.
	pub fn guess_from_extension(&self, path: impl AsRef<Path>) -> Result<TaggedFile> {
		let mut c = Cursor::new(std::fs::read(&path)?);

		let extension = path
			.as_ref()
			.extension()
			.ok_or(LoftyError::UnknownFileExtension)?;

		let extension_str = extension.to_str().ok_or(LoftyError::UnknownFileExtension)?;

		FileType::try_from_ext(extension_str)?;

		_read_from(&mut c, tag_type)
	}

	/// Attempts to get the tag format based on the file signature
	///
	/// NOTE: This is *slightly* slower than reading from extension, but more accurate.
	/// The only times were this would really be necessary is if the file format being read
	/// supports more than one metadata format (ex. RIFF), or there is no file extension.
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * The format couldn't be determined
	pub fn guess_from_content(&self, path: impl AsRef<Path>) -> Result<TaggedFile> {
		let mut c = Cursor::new(std::fs::read(&path)?);
		let file_type = FileType::try_from_sig(&mut c)?;

		_read_from(&mut c, file_type)
	}

	/// Attempts to get the tag format based on the data in the reader
	///
	/// See [`guess_from_content`][Probe::guess_from_content] for important notes, errors, and warnings.
	///
	/// # Errors
	///
	/// Same as [`guess_from_content`][Probe::guess_from_content]
	pub fn guess_from<R>(&self, reader: &mut R) -> Result<TaggedFile>
	where
		R: Read + Seek,
	{
		let file_type = FileType::try_from_sig(reader)?;

		_read_from(reader, tag_type)
	}
}

fn _read_from<R>(reader: &mut R, file_type: FileType) -> Result<TaggedFile>
where
	R: Read + Seek,
{
	match file_type {
		FileType::AIFF => AiffFile::read_from(),
		FileType::APE => ApeFile::read_from(),
		FileType::FLAC => FlacFile::read_from(),
		FileType::MP3 => MpegFile::read_from(),
		FileType::MP4 => // TODO,
		FileType::Opus => OpusFile::read_from(),
		FileType::Vorbis => VorbisFile::read_from(),
		FileType::WAV => RiffFile::read_from(),
	}
}