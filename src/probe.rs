use crate::logic::iff::aiff::AiffFile;
use crate::logic::ape::ApeFile;
use crate::logic::ogg::flac::FlacFile;
use crate::logic::mpeg::MpegFile;
use crate::logic::ogg::opus::OpusFile;
use crate::logic::ogg::vorbis::VorbisFile;
use crate::logic::iff::wav::WavFile;
use crate::types::file::AudioFile;
use crate::{FileType, LoftyError, Result, TaggedFile};

use std::io::{Cursor, Read, Seek};
use std::path::Path;

/// Provides a way to extract a [`FileType`] or [`TaggedFile`] from a reader
pub struct Probe;

impl Probe {
	/// Create a new `Probe`
	pub fn new() -> Self {
		Self
	}

	/// Attempts to get the [`FileType`] based on the data in the reader
	pub fn file_type<R>(&self, reader: &mut R) -> Option<FileType>
	where
		R: Read + Seek,
	{
		FileType::try_from_sig(reader).ok()
	}

	/// Attempts to get a [`FileType`] from a path
	///
	/// NOTE: This is based on the content of the file.
	/// If you want to guess based on extension, see [`Probe::file_type_from_extension`](Self::file_type_from_extension)
	pub fn file_type_from_path(&self, path: impl AsRef<Path>) -> Option<FileType> {
		if let Ok(content) = std::fs::read(&path) {
			let mut cursor = Cursor::new(content);
			return self.file_type(&mut cursor);
		}

		None
	}

	/// Attempts to get the [`FileType`] based on the file extension
	///
	/// NOTE: Since this only looks at the extension, the result could be incorrect.
	pub fn file_type_from_extension(&self, path: impl AsRef<Path>) -> Option<FileType> {
		if let Some(ext_os) = path.as_ref().extension() {
			if let Some(ext) = ext_os.to_str() {
				return FileType::try_from_ext(&*ext.to_lowercase()).ok();
			}
		}

		None
	}

	/// Attempts to extract a [`TaggedFile`] from a reader
	///
	/// # Errors
	///
	/// * The format couldn't be determined
	pub fn read_from<R>(self, reader: &mut R) -> Result<TaggedFile>
	where
		R: Read + Seek,
	{
		match FileType::try_from_sig(reader) {
			Ok(f_type) => _read_from(reader, f_type),
			Err(_) => Err(LoftyError::UnknownFormat),
		}
	}

	/// Attempts to extract a [`TaggedFile`] from a path
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * The format couldn't be determined
	pub fn read_from_path(self, path: impl AsRef<Path>) -> Result<TaggedFile> {
		let mut cursor = Cursor::new(std::fs::read(&path)?);
		self.read_from(&mut cursor)
	}
}

fn _read_from<R>(reader: &mut R, file_type: FileType) -> Result<TaggedFile>
where
	R: Read + Seek,
{
	match file_type {
		FileType::AIFF => Ok(AiffFile::read_from(reader)?.into()),
		FileType::APE => Ok(ApeFile::read_from(reader)?.into()),
		FileType::FLAC => Ok(FlacFile::read_from(reader)?.into()),
		FileType::MP3 => Ok(MpegFile::read_from(reader)?.into()),
		FileType::Opus => Ok(OpusFile::read_from(reader)?.into()),
		FileType::Vorbis => Ok(VorbisFile::read_from(reader)?.into()),
		FileType::WAV => Ok(WavFile::read_from(reader)?.into()),
		_ => Err(LoftyError::UnknownFormat), // FileType::MP4 => {}, TODO,
	}
}
