//! Format-agonostic file parsing tools

use crate::aac::AacFile;
use crate::ape::ApeFile;
use crate::config::{ParseOptions, global_options};
use crate::error::Result;
use crate::file::{AudioFile, BoundTaggedFile, FileType, FileTypeGuessResult, TaggedFile};
use crate::flac::FlacFile;
use crate::iff::aiff::AiffFile;
use crate::iff::wav::WavFile;
use crate::macros::err;
use crate::mp4::Mp4File;
use crate::mpeg::MpegFile;
use crate::mpeg::header::search_for_frame_sync;
use crate::musepack::MpcFile;
use crate::ogg::opus::OpusFile;
use crate::ogg::speex::SpeexFile;
use crate::ogg::vorbis::VorbisFile;
use crate::resolve::custom_resolvers;
use crate::wavpack::WavPackFile;

use crate::io::FileLike;
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
/// ```rust,no_run
/// # fn main() -> lofty::error::Result<()> {
/// use lofty::file::FileType;
/// use lofty::probe::Probe;
///
/// let probe = Probe::open("path/to/my.mp3")?;
///
/// // Inferred from the `mp3` extension
/// assert_eq!(probe.file_type(), Some(FileType::Mpeg));
/// # Ok(())
/// # }
/// ```
///
/// When a path isn't available, or is unreliable, content-based detection is also possible.
///
/// ```rust,no_run
/// # fn main() -> lofty::error::Result<()> {
/// use lofty::file::FileType;
/// use lofty::probe::Probe;
///
/// // Our same path probe with a guessed file type
/// let probe = Probe::open("path/to/my.mp3")?.guess_file_type()?;
///
/// // Inferred from the file's content
/// assert_eq!(probe.file_type(), Some(FileType::Mpeg));
/// # Ok(())
/// # }
/// ```
///
/// Or with another reader
///
/// ```rust
/// # fn main() -> lofty::error::Result<()> {
/// use lofty::file::FileType;
/// use lofty::probe::Probe;
/// use std::io::Cursor;
///
/// static MAC_HEADER: &[u8; 3] = b"MAC";
///
/// let probe = Probe::new(Cursor::new(MAC_HEADER)).guess_file_type()?;
///
/// // Inferred from the MAC header
/// assert_eq!(probe.file_type(), Some(FileType::Ape));
/// # Ok(())
/// # }
/// ```
pub struct Probe<R: Read> {
	inner: R,
	options: Option<ParseOptions>,
	f_ty: Option<FileType>,
}

impl<R: Read> Probe<R> {
	/// Create a new `Probe`
	///
	/// Before creating a `Probe`, consider wrapping it in a [`BufReader`] for better
	/// performance.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::probe::Probe;
	/// use std::fs::File;
	/// use std::io::BufReader;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let file = File::open(path)?;
	/// let reader = BufReader::new(file);
	///
	/// let probe = Probe::new(reader);
	/// # Ok(()) }
	/// ```
	#[must_use]
	pub const fn new(reader: R) -> Self {
		Self {
			inner: reader,
			options: None,
			f_ty: None,
		}
	}

	/// Create a new `Probe` with a specified [`FileType`]
	///
	/// Before creating a `Probe`, consider wrapping it in a [`BufReader`] for better
	/// performance.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	/// use std::fs::File;
	/// use std::io::BufReader;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let my_mp3_path = "tests/files/assets/minimal/full_test.mp3";
	/// // We know the file is going to be an MP3,
	/// // so we can skip the format detection
	/// let file = File::open(my_mp3_path)?;
	/// let reader = BufReader::new(file);
	///
	/// let probe = Probe::with_file_type(reader, FileType::Mpeg);
	/// # Ok(()) }
	/// ```
	pub fn with_file_type(reader: R, file_type: FileType) -> Self {
		Self {
			inner: reader,
			options: None,
			f_ty: Some(file_type),
		}
	}

	/// Returns the current [`FileType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let reader = std::io::Cursor::new(&[]);
	/// let probe = Probe::new(reader);
	///
	/// let file_type = probe.file_type();
	/// # Ok(()) }
	/// ```
	pub fn file_type(&self) -> Option<FileType> {
		self.f_ty
	}

	/// Set the [`FileType`] with which to read the file
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let reader = std::io::Cursor::new(&[]);
	/// let mut probe = Probe::new(reader);
	/// assert_eq!(probe.file_type(), None);
	///
	/// let probe = probe.set_file_type(FileType::Mpeg);
	///
	/// assert_eq!(probe.file_type(), Some(FileType::Mpeg));
	/// # Ok(()) }
	/// ```
	pub fn set_file_type(mut self, file_type: FileType) -> Self {
		self.f_ty = Some(file_type);
		self
	}

	/// Set the [`ParseOptions`] for the Probe
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let reader = std::io::Cursor::new(&[]);
	/// // By default, properties will be read.
	/// // In this example, we want to turn this off.
	/// let options = ParseOptions::new().read_properties(false);
	///
	/// let probe = Probe::new(reader).options(options);
	/// # Ok(()) }
	/// ```
	#[must_use]
	pub fn options(mut self, options: ParseOptions) -> Self {
		self.options = Some(options);
		self
	}

	/// Extract the reader
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let reader = std::io::Cursor::new(&[]);
	/// let probe = Probe::new(reader);
	///
	/// let reader = probe.into_inner();
	/// # Ok(()) }
	/// ```
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
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let probe = Probe::open("path/to/my.mp3")?;
	///
	/// // Guessed from the "mp3" extension, see `FileType::from_ext`
	/// assert_eq!(probe.file_type(), Some(FileType::Mpeg));
	/// # Ok(()) }
	/// ```
	pub fn open<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		log::debug!("Probe: Opening `{}` for reading", path.display());

		let file_type = FileType::from_path(path);
		log::debug!("Probe: Guessed file type `{:?}` from extension", file_type);

		Ok(Self {
			inner: BufReader::new(File::open(path)?),
			options: None,
			f_ty: file_type,
		})
	}
}

impl<R: Read + Seek> Probe<R> {
	/// Attempts to get the [`FileType`] based on the data in the reader
	///
	/// On success, the file type will be replaced
	///
	/// NOTE: The chance for succeeding is influenced by [`ParseOptions`].
	/// Be sure to set it with [`Probe::options()`] prior to calling this method.
	/// Some files may require more than the default [`ParseOptions::DEFAULT_MAX_JUNK_BYTES`] to be detected successfully.
	///
	/// # Errors
	///
	/// All errors that occur within this function are [`std::io::Error`].
	/// If an error does occur, there is likely an issue with the provided
	/// reader, and the entire `Probe` should be discarded.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// # let file = std::fs::File::open(path)?;
	/// # let reader = std::io::BufReader::new(file);
	/// let probe = Probe::new(reader).guess_file_type()?;
	///
	/// // Determined the file is MP3 from the content
	/// assert_eq!(probe.file_type(), Some(FileType::Mpeg));
	/// # Ok(()) }
	/// ```
	pub fn guess_file_type(mut self) -> std::io::Result<Self> {
		let max_junk_bytes = self
			.options
			.map_or(ParseOptions::DEFAULT_MAX_JUNK_BYTES, |options| {
				options.max_junk_bytes
			});

		let f_ty = self.guess_inner(max_junk_bytes)?;
		self.f_ty = f_ty.or(self.f_ty);

		log::debug!("Probe: Guessed file type: {:?}", self.f_ty);

		Ok(self)
	}

	#[allow(clippy::shadow_unrelated)]
	fn guess_inner(&mut self, max_junk_bytes: usize) -> std::io::Result<Option<FileType>> {
		// temporary buffer for storing 36 bytes
		// (36 is just a guess as to how long the data for estimating the file type might be)
		let mut buf = [0; 36];

		let starting_position = self.inner.stream_position()?;
		// Read (up to) 36 bytes
		let buf_len = std::io::copy(
			&mut self.inner.by_ref().take(buf.len() as u64),
			&mut Cursor::new(&mut buf[..]),
		)? as usize;

		self.inner.seek(SeekFrom::Start(starting_position))?;

		// Give custom resolvers priority
		if unsafe { global_options().use_custom_resolvers } {
			if let Ok(lock) = custom_resolvers().lock() {
				#[allow(clippy::significant_drop_in_scrutinee)]
				for (_, resolve) in lock.iter() {
					if let ret @ Some(_) = resolve.guess(&buf[..buf_len]) {
						return Ok(ret);
					}
				}
			}
		}

		// Guess the file type by using these 36 bytes
		let Some(file_type_guess) = FileType::from_buffer_inner(&buf[..buf_len]) else {
			return Ok(None);
		};

		match file_type_guess {
			// We were able to determine a file type
			FileTypeGuessResult::Determined(file_ty) => Ok(Some(file_ty)),
			// The file starts with an ID3v2 tag; this means other data can follow (e.g. APE or MP3 frames)
			FileTypeGuessResult::MaybePrecededById3(id3_len) => {
				// `id3_len` is the size of the tag, not including the header (10 bytes)
				log::debug!("Probe: ID3v2 tag detected, skipping {} bytes", 10 + id3_len);
				let position_after_id3_block = self
					.inner
					.seek(SeekFrom::Current(i64::from(10 + id3_len)))?;

				// try to guess the file type after the ID3 block by inspecting the first 4 bytes
				let mut ident = [0; 4];
				std::io::copy(
					&mut self.inner.by_ref().take(ident.len() as u64),
					&mut Cursor::new(&mut ident[..]),
				)?;

				self.inner.seek(SeekFrom::Start(position_after_id3_block))?;

				let file_type_after_id3_block = match &ident {
					[b'M', b'A', b'C', ..] => Ok(Some(FileType::Ape)),
					b"fLaC" => Ok(Some(FileType::Flac)),
					b"MPCK" | [b'M', b'P', b'+', ..] => Ok(Some(FileType::Mpc)),
					// Search for a frame sync, which may be preceded by junk
					_ => self.check_mpeg_or_aac(max_junk_bytes),
				};

				// before returning any result for a file type, seek back to the front
				self.inner.seek(SeekFrom::Start(starting_position))?;

				file_type_after_id3_block
			},
			// TODO: Check more than MPEG/AAC
			FileTypeGuessResult::MaybePrecededByJunk => {
				log::debug!(
					"Probe: Possible junk bytes detected, searching up to {} bytes",
					max_junk_bytes
				);

				let ret = self.check_mpeg_or_aac(max_junk_bytes);

				// before returning any result for a file type, seek back to the front
				self.inner.seek(SeekFrom::Start(starting_position))?;

				ret
			},
		}
	}

	/// Searches for an MPEG/AAC frame sync, which may be preceded by junk bytes
	fn check_mpeg_or_aac(&mut self, max_junk_bytes: usize) -> std::io::Result<Option<FileType>> {
		{
			let mut restricted_reader = self.inner.by_ref().take(max_junk_bytes as u64);
			if search_for_frame_sync(&mut restricted_reader)?.is_none() {
				return Ok(None);
			}
		}

		// Seek back to the start of the frame sync to check if we are dealing with
		// an AAC or MPEG file. See `FileType::quick_type_guess` for explanation.
		let sync_pos = self.inner.seek(SeekFrom::Current(-2))?;
		log::debug!("Probe: Found possible frame sync at position {}", sync_pos);

		let mut buf = [0; 2];
		self.inner.read_exact(&mut buf)?;

		if buf[1] & 0b10000 > 0 && buf[1] & 0b110 == 0 {
			Ok(Some(FileType::Aac))
		} else {
			Ok(Some(FileType::Mpeg))
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
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`register_custom_resolver`](crate::resolve::register_custom_resolver).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// # let file = std::fs::File::open(path)?;
	/// # let reader = std::io::BufReader::new(file);
	/// let probe = Probe::new(reader).guess_file_type()?;
	///
	/// let parsed_file = probe.read()?;
	/// # Ok(()) }
	/// ```
	pub fn read(self) -> Result<TaggedFile> {
		self.read_inner().map(|(tagged_file, _)| tagged_file)
	}

	fn read_inner(mut self) -> Result<(TaggedFile, R)> {
		let reader = &mut self.inner;
		let options = self.options.unwrap_or_default();

		if !options.read_tags && !options.read_properties {
			log::warn!("Skipping both tag and property reading, file will be empty");
		}

		let tagged_file = match self.f_ty {
			Some(f_type) => match f_type {
				FileType::Aac => AacFile::read_from(reader, options)?.into(),
				FileType::Aiff => AiffFile::read_from(reader, options)?.into(),
				FileType::Ape => ApeFile::read_from(reader, options)?.into(),
				FileType::Flac => FlacFile::read_from(reader, options)?.into(),
				FileType::Mpeg => MpegFile::read_from(reader, options)?.into(),
				FileType::Opus => OpusFile::read_from(reader, options)?.into(),
				FileType::Vorbis => VorbisFile::read_from(reader, options)?.into(),
				FileType::Wav => WavFile::read_from(reader, options)?.into(),
				FileType::Mp4 => Mp4File::read_from(reader, options)?.into(),
				FileType::Mpc => MpcFile::read_from(reader, options)?.into(),
				FileType::Speex => SpeexFile::read_from(reader, options)?.into(),
				FileType::WavPack => WavPackFile::read_from(reader, options)?.into(),
				FileType::Custom(c) => {
					if !unsafe { global_options().use_custom_resolvers } {
						err!(UnknownFormat)
					}

					let resolver = crate::resolve::lookup_resolver(c);
					resolver.read_from(reader, options)?
				},
			},
			None => err!(UnknownFormat),
		};

		Ok((tagged_file, self.inner))
	}
}

impl<F: FileLike> Probe<F> {
	/// Attempts to extract a [`BoundTaggedFile`] from the reader
	///
	/// # Errors
	///
	/// See [`Self::read()`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::probe::Probe;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// # let file = std::fs::File::open(path)?;
	/// let probe = Probe::new(file).guess_file_type()?;
	///
	/// let bound_tagged_file = probe.read_bound()?;
	/// # Ok(()) }
	/// ```
	pub fn read_bound(self) -> Result<BoundTaggedFile<F>> {
		let (tagged_file, file_handle) = self.read_inner()?;

		Ok(BoundTaggedFile {
			inner: tagged_file,
			file_handle,
		})
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
///
/// # Examples
///
/// ```rust
/// use lofty::read_from;
/// use std::fs::File;
///
/// # fn main() -> lofty::error::Result<()> {
/// # let path = "tests/files/assets/minimal/full_test.mp3";
/// let mut file = File::open(path)?;
///
/// let parsed_file = read_from(&mut file)?;
/// # Ok(()) }
/// ```
pub fn read_from(file: &mut File) -> Result<TaggedFile> {
	Probe::new(BufReader::new(file)).guess_file_type()?.read()
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
///
/// # Examples
///
/// ```rust
/// use lofty::read_from_path;
///
/// # fn main() -> lofty::error::Result<()> {
/// # let path = "tests/files/assets/minimal/full_test.mp3";
/// let parsed_file = read_from_path(path)?;
/// # Ok(()) }
/// ```
pub fn read_from_path<P>(path: P) -> Result<TaggedFile>
where
	P: AsRef<Path>,
{
	Probe::open(path)?.read()
}

#[cfg(test)]
mod tests {
	use crate::config::{GlobalOptions, ParseOptions};
	use crate::file::FileType;
	use crate::probe::Probe;

	use std::fs::File;

	#[test_log::test]
	fn mp3_id3v2_trailing_junk() {
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
		let data: Vec<u8> = data.into_iter().flatten().copied().collect();
		let data = std::io::Cursor::new(&data);
		let probe = Probe::new(data).guess_file_type().unwrap();
		assert_eq!(probe.file_type(), Some(FileType::Mpeg));
	}

	#[test_log::test]
	fn parse_options_allocation_limit() {
		// In this test, we read a partial MP3 file that has an ID3v2 tag containing a frame outside
		// of the allocation limit. We'll be testing with an encrypted frame, since we immediately read those into memory.

		use crate::id3::v2::util::synchsafe::SynchsafeInteger;

		fn create_encrypted_frame(size: usize) -> Vec<u8> {
			// Encryption method (1 byte) + encryption method data length indicator (4 bytes)
			// This is required and goes before the data.
			let flag_data = vec![0; 5];

			let bytes = vec![0; size];

			let frame_length_synch = ((bytes.len() + flag_data.len()) as u32)
				.synch()
				.unwrap()
				.to_be_bytes();
			let frame_header = vec![
				b'S',
				b'M',
				b'T',
				b'H',
				frame_length_synch[0],
				frame_length_synch[1],
				frame_length_synch[2],
				frame_length_synch[3],
				0x00,
				0b0000_0101, // Encrypted, Has data length indicator
			];

			[frame_header, flag_data, bytes].concat()
		}

		fn create_fake_mp3(frame_size: u32) -> Vec<u8> {
			let id3v2_tag_length = (frame_size + 5 + 10).synch().unwrap().to_be_bytes();
			[
				// ID3v2.4 header (10 bytes)
				vec![
					0x49,
					0x44,
					0x33,
					0x04,
					0x00,
					0x00,
					id3v2_tag_length[0],
					id3v2_tag_length[1],
					id3v2_tag_length[2],
					id3v2_tag_length[3],
				],
				// Random encrypted frame
				create_encrypted_frame(frame_size as usize),
				// start of MP3 frame (not all bytes are shown in this slice)
				vec![
					0xFF, 0xFB, 0x50, 0xC4, 0x00, 0x03, 0xC0, 0x00, 0x01, 0xA4, 0x00, 0x00, 0x00,
					0x20, 0x00, 0x00, 0x34, 0x80, 0x00, 0x00, 0x04,
				],
			]
			.into_iter()
			.flatten()
			.collect::<Vec<u8>>()
		}

		let parse_options = ParseOptions::new().read_properties(false);

		let mut global_options = GlobalOptions::new().allocation_limit(50);
		crate::config::apply_global_options(global_options);

		// An allocation with a size of 40 bytes should be ok
		let within_limits = create_fake_mp3(40);
		let probe = Probe::new(std::io::Cursor::new(&within_limits))
			.set_file_type(FileType::Mpeg)
			.options(parse_options);
		assert!(probe.read().is_ok());

		// An allocation with a size of 60 bytes should fail
		let too_big = create_fake_mp3(60);
		let probe = Probe::new(std::io::Cursor::new(&too_big))
			.set_file_type(FileType::Mpeg)
			.options(parse_options);
		assert!(probe.read().is_err());

		// Now test the default allocation limit (16MB), which should of course be ok with 60 bytes
		global_options.allocation_limit = GlobalOptions::DEFAULT_ALLOCATION_LIMIT;
		crate::config::apply_global_options(global_options);

		let probe = Probe::new(std::io::Cursor::new(&too_big))
			.set_file_type(FileType::Mpeg)
			.options(parse_options);
		assert!(probe.read().is_ok());
	}

	fn test_probe(path: &str, expected_file_type_guess: FileType) {
		test_probe_file(path, expected_file_type_guess);
		test_probe_path(path, expected_file_type_guess);
	}

	// Test from file contents
	fn test_probe_file(path: &str, expected_file_type_guess: FileType) {
		let mut f = File::open(path).unwrap();
		let probe = Probe::new(&mut f).guess_file_type().unwrap();
		assert_eq!(probe.file_type(), Some(expected_file_type_guess));
	}

	// Test from file extension
	fn test_probe_path(path: &str, expected_file_type_guess: FileType) {
		let probe = Probe::open(path).unwrap();
		assert_eq!(probe.file_type(), Some(expected_file_type_guess));
	}

	#[test_log::test]
	fn probe_aac() {
		test_probe("tests/files/assets/minimal/untagged.aac", FileType::Aac);
	}

	#[test_log::test]
	fn probe_aac_with_id3v2() {
		test_probe("tests/files/assets/minimal/full_test.aac", FileType::Aac);
	}

	#[test_log::test]
	fn probe_aiff() {
		test_probe("tests/files/assets/minimal/full_test.aiff", FileType::Aiff);
	}

	#[test_log::test]
	fn probe_ape_with_id3v2() {
		test_probe("tests/files/assets/minimal/full_test.ape", FileType::Ape);
	}

	#[test_log::test]
	fn probe_flac() {
		test_probe("tests/files/assets/minimal/full_test.flac", FileType::Flac);
	}

	#[test_log::test]
	fn probe_flac_with_id3v2() {
		test_probe("tests/files/assets/flac_with_id3v2.flac", FileType::Flac);
	}

	#[test_log::test]
	fn probe_mp3_with_id3v2() {
		test_probe("tests/files/assets/minimal/full_test.mp3", FileType::Mpeg);
	}

	#[test_log::test]
	fn probe_mp3_with_lots_of_junk() {
		test_probe("tests/files/assets/junk.mp3", FileType::Mpeg);
	}

	#[test_log::test]
	fn probe_vorbis() {
		test_probe("tests/files/assets/minimal/full_test.ogg", FileType::Vorbis);
	}

	#[test_log::test]
	fn probe_opus() {
		test_probe("tests/files/assets/minimal/full_test.opus", FileType::Opus);
	}

	#[test_log::test]
	fn probe_speex() {
		test_probe("tests/files/assets/minimal/full_test.spx", FileType::Speex);
	}

	#[test_log::test]
	fn probe_mp4() {
		test_probe(
			"tests/files/assets/minimal/m4a_codec_aac.m4a",
			FileType::Mp4,
		);
	}

	#[test_log::test]
	fn probe_wav() {
		test_probe(
			"tests/files/assets/minimal/wav_format_pcm.wav",
			FileType::Wav,
		);
	}
}
