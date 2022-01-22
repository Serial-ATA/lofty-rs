use crate::ape::ApeFile;
use crate::error::{LoftyError, Result};
use crate::iff::aiff::AiffFile;
use crate::iff::wav::WavFile;
use crate::mp3::header::search_for_frame_sync;
use crate::mp3::Mp3File;
use crate::mp4::Mp4File;
use crate::ogg::flac::FlacFile;
use crate::ogg::opus::OpusFile;
use crate::ogg::vorbis::VorbisFile;
use crate::types::file::{AudioFile, FileType, TaggedFile};

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

/// A format agnostic reader
///
/// This provides a way to determine the [`FileType`] of a reader, for when a concrete
/// type is not known.
///
/// ## Usage
///
/// When reading from a path, the [`FileType`] will be inferred from the path, rather than the
/// open file.
///
/// ```rust
/// # use lofty::{LoftyError, Probe};
/// # fn main() -> Result<(), LoftyError> {
/// use lofty::FileType;
///
/// let probe = Probe::open("tests/files/assets/a.mp3")?;
///
/// // Inferred from the `mp3` extension
/// assert_eq!(probe.file_type(), Some(FileType::MP3));
/// # Ok(())
/// # }
/// ```
///
/// When a path isn't available, or is unreliable, content-based detection is also possible.
///
/// ```rust
/// # use lofty::{LoftyError, Probe};
/// # fn main() -> Result<(), LoftyError> {
/// use lofty::FileType;
///
/// // Our same path probe with a guessed file type
/// let probe = Probe::open("tests/files/assets/a.mp3")?.guess_file_type()?;
///
/// // Inferred from the file's content
/// assert_eq!(probe.file_type(), Some(FileType::MP3));
/// # Ok(())
/// # }
/// ```
///
/// Or with another reader
///
/// ```rust
/// # use lofty::{LoftyError, Probe};
/// # fn main() -> Result<(), LoftyError> {
/// use lofty::FileType;
/// use std::io::Cursor;
///
/// static MAC_HEADER: &[u8; 3] = b"MAC";
///
/// let probe = Probe::new(Cursor::new(MAC_HEADER)).guess_file_type()?;
///
/// // Inferred from the MAC header
/// assert_eq!(probe.file_type(), Some(FileType::APE));
/// # Ok(())
/// # }
/// ```
pub struct Probe<R: Read> {
	inner: R,
	f_ty: Option<FileType>,
}

impl<R: Read> Probe<R> {
	/// Create a new `Probe`
	pub fn new(reader: R) -> Self {
		Self {
			inner: reader,
			f_ty: None,
		}
	}

	/// Create a new `Probe` with a specified [`FileType`]
	pub fn with_file_type(reader: R, file_type: FileType) -> Self {
		Self {
			inner: reader,
			f_ty: Some(file_type),
		}
	}

	/// Returns the current [`FileType`]
	pub fn file_type(&self) -> Option<FileType> {
		self.f_ty
	}

	/// Set the [`FileType`] with which to read the file
	pub fn set_file_type(&mut self, file_type: FileType) {
		self.f_ty = Some(file_type)
	}

	/// Extract the reader
	pub fn into_inner(self) -> R {
		self.inner
	}
}

impl Probe<BufReader<File>> {
	/// Opens a file for reading
	///
	/// This will initially guess the [`FileType`] from the path, but
	/// this can be overwritten with [`Probe::guess_file_type`] or [`Probe::set_file_type`]
	///
	/// # Errors
	///
	/// * `path` does not exist
	pub fn open<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();

		Ok(Self {
			inner: BufReader::new(File::open(path)?),
			f_ty: FileType::from_path(path).ok(),
		})
	}
}

impl<R: Read + Seek> Probe<R> {
	/// Attempts to get the [`FileType`] based on the data in the reader
	///
	/// On success, the file type will be replaced
	///
	/// # Errors
	///
	/// All errors that occur within this function are [`std::io::Error`].
	/// If an error does occur, there is likely an issue with the provided
	/// reader, and the entire `Probe` should be discarded.
	pub fn guess_file_type(mut self) -> Result<Self> {
		let f_ty = self.guess_inner()?;
		self.f_ty = f_ty.or(self.f_ty);

		Ok(self)
	}

	#[allow(clippy::shadow_unrelated)]
	fn guess_inner(&mut self) -> std::io::Result<Option<FileType>> {
		// temporary buffer for storing 36 bytes
		// (36 is just a guess as to how long the data for estimating the file type might be)
		let mut buf = [0; 36];

		// read the first 36 bytes and seek back to the starting position
		let starting_position = self.inner.stream_position()?;
		let buf_len = std::io::copy(
			&mut self.inner.by_ref().take(buf.len() as u64),
			&mut Cursor::new(&mut buf[..]),
		)? as usize;
		self.inner.seek(SeekFrom::Start(starting_position))?;

		// estimate the file type by using these 36 bytes
		// note that any error from `from_buffer_inner` are suppressed, as it returns an error on unknown format
		match FileType::from_buffer_inner(&buf[..buf_len]) {
			// the file type was guessed based on these bytes
			Ok((Some(f_ty), _)) => Ok(Some(f_ty)),
			// the first data block is ID3 data; this means other data can follow (e.g. APE or MP3 frames)
			Ok((None, id3_len)) => {
				// the position right after the ID3 block is the internal size value (id3_len)
				// added to the length of the ID3 header (which is 10 bytes),
				// as the size does not include the header itself
				let position_after_id3_block = self
					.inner
					.seek(SeekFrom::Current(i64::from(10 + id3_len)))?;

				let file_type_after_id3_block = {
					// try to guess the file type after the ID3 block by inspecting the first 3 bytes
					let mut ident = [0; 3];
					std::io::copy(
						&mut self.inner.by_ref().take(ident.len() as u64),
						&mut Cursor::new(&mut ident[..]),
					)?;

					if &ident == b"MAC" {
						Ok(Some(FileType::APE))
					} else {
						// potentially some junk bytes are between the ID3 block and the following MP3 block
						// search for any possible sync bits after the ID3 block
						self.inner.seek(SeekFrom::Start(position_after_id3_block))?;
						if search_for_frame_sync(&mut self.inner)?.is_some() {
							Ok(Some(FileType::MP3))
						} else {
							Ok(None)
						}
					}
				};

				// before returning any result for a file type, seek back to the front
				self.inner.seek(SeekFrom::Start(starting_position))?;

				file_type_after_id3_block
			},
			_ => Ok(None),
		}
	}

	/// Attempts to extract a [`TaggedFile`] from the reader
	///
	/// If `read_properties` is false, the properties will be zeroed out.
	///
	/// # Errors
	///
	/// * No file type
	///     - This expects the file type to have been set already, either with
	///       [`Probe::guess_file_type`] or [`Probe::set_file_type`]. When reading from
	///       paths, this is not necessary.
	/// * The reader contains invalid data
	pub fn read(mut self, read_properties: bool) -> Result<TaggedFile> {
		let reader = &mut self.inner;

		match self.f_ty {
			Some(f_type) => Ok(match f_type {
				FileType::AIFF => AiffFile::read_from(reader, read_properties)?.into(),
				FileType::APE => ApeFile::read_from(reader, read_properties)?.into(),
				FileType::FLAC => FlacFile::read_from(reader, read_properties)?.into(),
				FileType::MP3 => Mp3File::read_from(reader, read_properties)?.into(),
				FileType::Opus => OpusFile::read_from(reader, read_properties)?.into(),
				FileType::Vorbis => VorbisFile::read_from(reader, read_properties)?.into(),
				FileType::WAV => WavFile::read_from(reader, read_properties)?.into(),
				FileType::MP4 => Mp4File::read_from(reader, read_properties)?.into(),
			}),
			None => Err(LoftyError::UnknownFormat),
		}
	}
}

/// Read a [`TaggedFile`] from a [File]
///
/// # Errors
///
/// See:
///
/// * [`Probe::guess_file_type`]
/// * [`Probe::read`]
pub fn read_from(file: &mut File, read_properties: bool) -> Result<TaggedFile> {
	Probe::new(file).guess_file_type()?.read(read_properties)
}

/// Read a [`TaggedFile`] from a path
///
/// NOTE: This will determine the [`FileType`] from the extension
///
/// # Errors
///
/// See:
///
/// * [`Probe::open`]
/// * [`Probe::read`]
pub fn read_from_path<P>(path: P, read_properties: bool) -> Result<TaggedFile>
where
	P: AsRef<Path>,
{
	Probe::open(path)?.read(read_properties)
}

#[cfg(test)]
mod tests {
	use crate::Probe;

	#[test]
	fn mp3_file_id3v2_3() {
		// test data that contains 4 bytes of junk (0x20) between the ID3 portion and the first MP3 frame
		let data: [&[u8]; 4] = [
			// ID3v2.3 header (10 bytes)
			&[0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x23],
			// TALB frame
			&[
				0x54, 0x41, 0x4C, 0x42, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x01, 0xFF, 0xFE, 0x61,
				0x00, 0x61, 0x00, 0x61, 0x00, 0x61, 0x00, 0x61, 0x00, 0x61, 0x00, 0x61, 0x00, 0x61,
				0x00, 0x61, 0x00, 0x61, 0x00, 0x61, 0x00,
			],
			// 4 bytes of junk
			&[0x20, 0x20, 0x20, 0x20],
			// start of MP3 frame (not all bytes are shown in this slice)
			&[
				0xFF, 0xFB, 0x50, 0xC4, 0x00, 0x03, 0xC0, 0x00, 0x01, 0xA4, 0x00, 0x00, 0x00, 0x20,
				0x00, 0x00, 0x34, 0x80, 0x00, 0x00, 0x04,
			],
		];
		let data: Vec<u8> = data.into_iter().flatten().cloned().collect();
		let data = std::io::Cursor::new(&data);
		let probe = Probe::new(data).guess_file_type().unwrap();
		assert_eq!(probe.file_type(), Some(crate::FileType::MP3));
	}
}
