use crate::aac::AACFile;
use crate::ape::ApeFile;
use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
use crate::flac::FlacFile;
use crate::iff::aiff::AiffFile;
use crate::iff::wav::WavFile;
use crate::macros::err;
use crate::mp4::Mp4File;
use crate::mpeg::header::search_for_frame_sync;
use crate::mpeg::MPEGFile;
use crate::ogg::opus::OpusFile;
use crate::ogg::speex::SpeexFile;
use crate::ogg::vorbis::VorbisFile;
use crate::resolve::CUSTOM_RESOLVERS;
use crate::wavpack::WavPackFile;

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

/// Options to control how Lofty parses a file
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub struct ParseOptions {
	pub(crate) read_properties: bool,
	pub(crate) use_custom_resolvers: bool,
	pub(crate) parsing_mode: ParsingMode,
}

impl Default for ParseOptions {
	/// The default implementation for `ParseOptions`
	///
	/// The defaults are as follows:
	///
	/// ```rust,ignore
	/// ParseOptions {
	/// 	read_properties: true,
	/// 	use_custom_resolvers: true,
	/// 	parsing_mode: ParsingMode::Strict,
	/// }
	/// ```
	fn default() -> Self {
		Self {
			read_properties: true,
			use_custom_resolvers: true,
			parsing_mode: ParsingMode::Strict,
		}
	}
}

impl ParseOptions {
	/// Creates a new `ParseOptions`, alias for `Default` implementation
	///
	/// See also: [`ParseOptions::default`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ParseOptions;
	///
	/// let parsing_options = ParseOptions::new();
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Whether or not to read the audio properties
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ParseOptions;
	///
	/// // By default, `read_properties` is enabled. Here, we don't want to read them.
	/// let parsing_options = ParseOptions::new().read_properties(false);
	/// ```
	pub fn read_properties(&mut self, read_properties: bool) -> Self {
		self.read_properties = read_properties;
		*self
	}

	/// Whether or not to check registered custom resolvers
	///
	/// See also: [`crate::resolve`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ParseOptions;
	///
	/// // By default, `use_custom_resolvers` is enabled. Here, we don't want to use them.
	/// let parsing_options = ParseOptions::new().use_custom_resolvers(false);
	/// ```
	pub fn use_custom_resolvers(&mut self, use_custom_resolvers: bool) -> Self {
		self.use_custom_resolvers = use_custom_resolvers;
		*self
	}

	/// The parsing mode to use, see [`ParsingMode`] for details
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{ParseOptions, ParsingMode};
	///
	/// // By default, `parsing_mode` is ParsingMode::Strict. Here, we don't need absolute correctness.
	/// let parsing_options = ParseOptions::new().parsing_mode(ParsingMode::Relaxed);
	/// ```
	pub fn parsing_mode(&mut self, parsing_mode: ParsingMode) -> Self {
		self.parsing_mode = parsing_mode;
		*self
	}
}

/// The parsing strictness mode
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum ParsingMode {
	/// The default mode, will eagerly error on invalid input
	Strict,
	/// Less eager to error, may produce invalid output
	Relaxed,
}

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
/// # use lofty::{LoftyError, Probe};
/// # fn main() -> Result<(), LoftyError> {
/// use lofty::FileType;
///
/// let probe = Probe::open("path/to/my.mp3")?;
///
/// // Inferred from the `mp3` extension
/// assert_eq!(probe.file_type(), Some(FileType::MPEG));
/// # Ok(())
/// # }
/// ```
///
/// When a path isn't available, or is unreliable, content-based detection is also possible.
///
/// ```rust,no_run
/// # use lofty::{LoftyError, Probe};
/// # fn main() -> Result<(), LoftyError> {
/// use lofty::FileType;
///
/// // Our same path probe with a guessed file type
/// let probe = Probe::open("path/to/my.mp3")?.guess_file_type()?;
///
/// // Inferred from the file's content
/// assert_eq!(probe.file_type(), Some(FileType::MPEG));
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
	options: Option<ParseOptions>,
	f_ty: Option<FileType>,
}

impl<R: Read> Probe<R> {
	/// Create a new `Probe`
	///
	/// Before creating a `Probe`, consider wrapping it in a [`BufReader`](std::io::BufReader) for better
	/// performance.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::Probe;
	/// use std::fs::File;
	/// use std::io::BufReader;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let file = File::open(path)?;
	/// let reader = BufReader::new(file);
	///
	/// let probe = Probe::new(reader);
	/// # Ok(()) }
	/// ```
	pub fn new(reader: R) -> Self {
		Self {
			inner: reader,
			options: None,
			f_ty: None,
		}
	}

	/// Create a new `Probe` with a specified [`FileType`]
	///
	/// Before creating a `Probe`, consider wrapping it in a [`BufReader`](std::io::BufReader) for better
	/// performance.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, Probe};
	/// use std::fs::File;
	/// use std::io::BufReader;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let my_mp3_path = "tests/files/assets/minimal/full_test.mp3";
	/// // We know the file is going to be an MP3,
	/// // so we can skip the format detection
	/// let file = File::open(my_mp3_path)?;
	/// let reader = BufReader::new(file);
	///
	/// let probe = Probe::with_file_type(reader, FileType::MPEG);
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
	/// use lofty::{FileType, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{FileType, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let reader = std::io::Cursor::new(&[]);
	/// let mut probe = Probe::new(reader);
	/// assert_eq!(probe.file_type(), None);
	///
	/// probe.set_file_type(FileType::MPEG);
	///
	/// assert_eq!(probe.file_type(), Some(FileType::MPEG));
	/// # Ok(()) }
	/// ```
	pub fn set_file_type(&mut self, file_type: FileType) {
		self.f_ty = Some(file_type)
	}

	/// Set the [`ParseOptions`] for the Probe
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{ParseOptions, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{FileType, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{FileType, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
	/// let probe = Probe::open("path/to/my.mp3")?;
	///
	/// // Guessed from the "mp3" extension, see `FileType::from_ext`
	/// assert_eq!(probe.file_type(), Some(FileType::MPEG));
	/// # Ok(()) }
	/// ```
	pub fn open<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();

		Ok(Self {
			inner: BufReader::new(File::open(path)?),
			options: None,
			f_ty: FileType::from_path(path),
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
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// # let file = std::fs::File::open(path)?;
	/// # let reader = std::io::BufReader::new(file);
	/// let probe = Probe::new(reader).guess_file_type()?;
	///
	/// // Determined the file is MP3 from the content
	/// assert_eq!(probe.file_type(), Some(FileType::MPEG));
	/// # Ok(()) }
	/// ```
	pub fn guess_file_type(mut self) -> std::io::Result<Self> {
		let f_ty = self.guess_inner()?;
		self.f_ty = f_ty.or(self.f_ty);

		Ok(self)
	}

	#[allow(clippy::shadow_unrelated)]
	fn guess_inner(&mut self) -> std::io::Result<Option<FileType>> {
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

		// Guess the file type by using these 36 bytes
		match FileType::from_buffer_inner(&buf[..buf_len]) {
			// We were able to determine a file type
			(Some(f_ty), _) => Ok(Some(f_ty)),
			// The file starts with an ID3v2 tag; this means other data can follow (e.g. APE or MP3 frames)
			(None, Some(id3_len)) => {
				// `id3_len` is the size of the tag, not including the header (10 bytes)
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
					[b'M', b'A', b'C', ..] => Ok(Some(FileType::APE)),
					b"fLaC" => Ok(Some(FileType::FLAC)),
					// Search for a frame sync, which may be preceded by junk
					_ if search_for_frame_sync(&mut self.inner)?.is_some() => {
						// Seek back to the start of the frame sync to check if we are dealing with
						// an AAC or MPEG file. See `FileType::quick_type_guess` for explanation.
						self.inner.seek(SeekFrom::Current(-2))?;

						let mut buf = [0; 2];
						self.inner.read_exact(&mut buf)?;

						if buf[1] & 0b10000 > 0 && buf[1] & 0b110 == 0 {
							return Ok(Some(FileType::AAC));
						}

						Ok(Some(FileType::MPEG))
					},
					_ => Ok(None),
				};

				// before returning any result for a file type, seek back to the front
				self.inner.seek(SeekFrom::Start(starting_position))?;

				file_type_after_id3_block
			},
			_ => {
				if let Ok(lock) = CUSTOM_RESOLVERS.lock() {
					#[allow(clippy::significant_drop_in_scrutinee)]
					for (_, resolve) in lock.iter() {
						if let ret @ Some(_) = resolve.guess(&buf[..buf_len]) {
							return Ok(ret);
						}
					}
				}

				Ok(None)
			},
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
	/// use lofty::{FileType, Probe};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// # let file = std::fs::File::open(path)?;
	/// # let reader = std::io::BufReader::new(file);
	/// let probe = Probe::new(reader).guess_file_type()?;
	///
	/// let parsed_file = probe.read()?;
	/// # Ok(()) }
	/// ```
	pub fn read(mut self) -> Result<TaggedFile> {
		let reader = &mut self.inner;
		let options = self.options.unwrap_or_default();

		match self.f_ty {
			Some(f_type) => Ok(match f_type {
				FileType::AAC => AACFile::read_from(reader, options)?.into(),
				FileType::AIFF => AiffFile::read_from(reader, options)?.into(),
				FileType::APE => ApeFile::read_from(reader, options)?.into(),
				FileType::FLAC => FlacFile::read_from(reader, options)?.into(),
				FileType::MPEG => MPEGFile::read_from(reader, options)?.into(),
				FileType::Opus => OpusFile::read_from(reader, options)?.into(),
				FileType::Vorbis => VorbisFile::read_from(reader, options)?.into(),
				FileType::WAV => WavFile::read_from(reader, options)?.into(),
				FileType::MP4 => Mp4File::read_from(reader, options)?.into(),
				FileType::Speex => SpeexFile::read_from(reader, options)?.into(),
				FileType::WavPack => WavPackFile::read_from(reader, options)?.into(),
				FileType::Custom(c) => {
					if !options.use_custom_resolvers {
						err!(UnknownFormat)
					}

					let resolver = crate::resolve::lookup_resolver(c);
					resolver.read_from(reader, options)?
				},
			}),
			None => err!(UnknownFormat),
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
///
/// # Examples
///
/// ```rust
/// use lofty::read_from;
/// use std::fs::File;
///
/// # fn main() -> lofty::Result<()> {
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
/// # fn main() -> lofty::Result<()> {
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
	use crate::{FileType, Probe};

	use std::fs::File;

	#[test]
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
		assert_eq!(probe.file_type(), Some(FileType::MPEG));
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

	#[test]
	fn probe_aac() {
		test_probe("tests/files/assets/minimal/untagged.aac", FileType::AAC);
	}

	#[test]
	fn probe_aac_with_id3v2() {
		test_probe("tests/files/assets/minimal/full_test.aac", FileType::AAC);
	}

	#[test]
	fn probe_aiff() {
		test_probe("tests/files/assets/minimal/full_test.aiff", FileType::AIFF);
	}

	#[test]
	fn probe_ape_with_id3v2() {
		test_probe("tests/files/assets/minimal/full_test.ape", FileType::APE);
	}

	#[test]
	fn probe_flac() {
		test_probe("tests/files/assets/minimal/full_test.flac", FileType::FLAC);
	}

	#[test]
	fn probe_flac_with_id3v2() {
		test_probe("tests/files/assets/flac_with_id3v2.flac", FileType::FLAC);
	}

	#[test]
	fn probe_mp3_with_id3v2() {
		test_probe("tests/files/assets/minimal/full_test.mp3", FileType::MPEG);
	}

	#[test]
	fn probe_vorbis() {
		test_probe("tests/files/assets/minimal/full_test.ogg", FileType::Vorbis);
	}

	#[test]
	fn probe_opus() {
		test_probe("tests/files/assets/minimal/full_test.opus", FileType::Opus);
	}

	#[test]
	fn probe_speex() {
		test_probe("tests/files/assets/minimal/full_test.spx", FileType::Speex);
	}

	#[test]
	fn probe_mp4() {
		test_probe(
			"tests/files/assets/minimal/m4a_codec_aac.m4a",
			FileType::MP4,
		);
	}

	#[test]
	fn probe_wav() {
		test_probe(
			"tests/files/assets/minimal/wav_format_pcm.wav",
			FileType::WAV,
		);
	}
}
