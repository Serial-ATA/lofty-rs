use crate::config::global_options;
use crate::resolve::custom_resolvers;
use crate::tag::{TagSupport, TagType};

use std::ffi::OsStr;
use std::path::Path;

/// List of common audio extensions
///
/// This contains a bunch of common extensions for all supported [`FileType`]s, and can be used a filter
/// when scanning directories.
///
/// NOTE: This is **not** an exhaustive list, but it should work fine in most cases.
///
/// # Examples
///
/// ```rust,no_run
/// use lofty::file::EXTENSIONS;
/// use std::fs;
///
/// # fn main() -> lofty::error::Result<()> {
/// for entry in fs::read_dir(".")? {
/// 	let entry = entry?;
///
/// 	let path = entry.path();
/// 	let Some(extension) = path.extension() else {
/// 		continue;
/// 	};
///
/// 	// Skip any non-audio file extensions
/// 	if !EXTENSIONS.iter().any(|e| *e == extension) {
/// 		continue;
/// 	}
///
/// 	// `entry` is *most likely* a supported file at this point
/// 	let parsed = lofty::read_from_path(path)?;
/// }
/// # Ok(()) }
/// ```
pub const EXTENSIONS: &[&str] = &[
	// Also update `FileType::from_ext()` below
	"aac", "ape", "aiff", "aif", "afc", "aifc", "mp3", "mp2", "mp1", "wav", "wv", "opus", "flac",
	"ogg", "mp4", "m4a", "m4b", "m4p", "m4r", "m4v", "3gp", "mpc", "mp+", "mpp", "spx",
];

/// The type of file read
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[allow(clippy::unsafe_derive_deserialize)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum FileType {
	Aac,
	Aiff,
	Ape,
	Flac,
	Mpeg,
	Mp4,
	Mpc,
	Opus,
	Vorbis,
	Speex,
	Wav,
	WavPack,
	Custom(&'static str),
}

impl FileType {
	/// Returns the file type's "primary" [`TagType`], or the one most likely to be used in the target format
	///
	/// | [`FileType`]                      | [`TagType`]      |
	/// |-----------------------------------|------------------|
	/// | `Aac`, `Aiff`, `Mp3`, `Wav`       | `Id3v2`          |
	/// | `Ape` , `Mpc`, `WavPack`          | `Ape`            |
	/// | `Flac`, `Opus`, `Vorbis`, `Speex` | `VorbisComments` |
	/// | `Mp4`                             | `Mp4Ilst`        |
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`register_custom_resolver`](crate::resolve::register_custom_resolver).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::tag::TagType;
	///
	/// let file_type = FileType::Mpeg;
	/// assert_eq!(file_type.primary_tag_type(), TagType::Id3v2);
	/// ```
	pub fn primary_tag_type(&self) -> TagType {
		match self {
			FileType::Aac | FileType::Aiff | FileType::Mpeg | FileType::Wav => TagType::Id3v2,
			FileType::Ape | FileType::Mpc | FileType::WavPack => TagType::Ape,
			FileType::Flac | FileType::Opus | FileType::Vorbis | FileType::Speex => {
				TagType::VorbisComments
			},
			FileType::Mp4 => TagType::Mp4Ilst,
			FileType::Custom(c) => {
				let resolver = crate::resolve::lookup_resolver(c);
				resolver.primary_tag_type()
			},
		}
	}

	/// Describes how this `FileType` supports the given [`TagType`]
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`register_custom_resolver`](crate::resolve::register_custom_resolver).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use lofty::tag::TagType;
	///
	/// // `FileType::Mpeg` supports both reading and writing ID3v2
	/// assert!(FileType::Mpeg.tag_support(TagType::Id3v2).is_writable());
	/// ```
	pub fn tag_support(&self, tag_type: TagType) -> TagSupport {
		if let FileType::Custom(c) = self {
			let resolver = crate::resolve::lookup_resolver(c);
			return resolver.tag_support(tag_type);
		}

		macro_rules! tag_support {
			(
				$tag_type:ident,
				$(($variant:ident, $tag:path)),* $(,)?
			) => {
				match $tag_type {
					$(
						TagType::$variant => {
							if <$tag>::SUPPORTED_FORMATS.contains(self) {
								if <$tag>::READ_ONLY_FORMATS.contains(self) {
									return TagSupport::ReadOnly;
								}

								return TagSupport::ReadWrite;
							}

							TagSupport::Unsupported
						},
					)*
				}
			}
		}

		tag_support!(
			tag_type,
			(Ape, crate::ape::ApeTag),
			(Id3v1, crate::id3::v1::Id3v1Tag),
			(Id3v2, crate::id3::v2::Id3v2Tag),
			(Mp4Ilst, crate::mp4::Ilst),
			(VorbisComments, crate::ogg::VorbisComments),
			(RiffInfo, crate::iff::wav::RiffInfoList),
			(AiffText, crate::iff::aiff::AiffTextChunks),
		)
	}

	/// Attempts to extract a [`FileType`] from an extension
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	///
	/// let extension = "mp3";
	/// assert_eq!(FileType::from_ext(extension), Some(FileType::Mpeg));
	/// ```
	pub fn from_ext<E>(ext: E) -> Option<Self>
	where
		E: AsRef<OsStr>,
	{
		let ext = ext.as_ref().to_str()?.to_ascii_lowercase();

		// Give custom resolvers priority
		if unsafe { global_options().use_custom_resolvers } {
			if let Some((ty, _)) = custom_resolvers()
				.lock()
				.ok()?
				.iter()
				.find(|(_, f)| f.extension() == Some(ext.as_str()))
			{
				return Some(Self::Custom(ty));
			}
		}

		// Also update `EXTENSIONS` above
		match ext.as_str() {
			"aac" => Some(Self::Aac),
			"ape" => Some(Self::Ape),
			"aiff" | "aif" | "afc" | "aifc" => Some(Self::Aiff),
			"mp3" | "mp2" | "mp1" => Some(Self::Mpeg),
			"wav" | "wave" => Some(Self::Wav),
			"wv" => Some(Self::WavPack),
			"opus" => Some(Self::Opus),
			"flac" => Some(Self::Flac),
			"ogg" => Some(Self::Vorbis),
			"mp4" | "m4a" | "m4b" | "m4p" | "m4r" | "m4v" | "3gp" => Some(Self::Mp4),
			"mpc" | "mp+" | "mpp" => Some(Self::Mpc),
			"spx" => Some(Self::Speex),
			_ => None,
		}
	}

	/// Attempts to determine a [`FileType`] from a path
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use std::path::Path;
	///
	/// let path = Path::new("path/to/my.mp3");
	/// assert_eq!(FileType::from_path(path), Some(FileType::Mpeg));
	/// ```
	pub fn from_path<P>(path: P) -> Option<Self>
	where
		P: AsRef<Path>,
	{
		let ext = path.as_ref().extension();
		ext.and_then(Self::from_ext)
	}

	/// Attempts to extract a [`FileType`] from a buffer
	///
	/// NOTES:
	///
	/// * This is for use in [`Probe::guess_file_type`], it is recommended to use it that way
	/// * This **will not** search past tags at the start of the buffer.
	///   For this behavior, use [`Probe::guess_file_type`].
	///
	/// [`Probe::guess_file_type`]: crate::probe::Probe::guess_file_type
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::FileType;
	/// use std::fs::File;
	/// use std::io::Read;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_opus = "tests/files/assets/minimal/full_test.opus";
	/// let mut file = File::open(path_to_opus)?;
	///
	/// let mut buf = [0; 50]; // Search the first 50 bytes of the file
	/// file.read_exact(&mut buf)?;
	///
	/// assert_eq!(FileType::from_buffer(&buf), Some(FileType::Opus));
	/// # Ok(()) }
	/// ```
	pub fn from_buffer(buf: &[u8]) -> Option<Self> {
		match Self::from_buffer_inner(buf) {
			Some(FileTypeGuessResult::Determined(file_ty)) => Some(file_ty),
			// We make no attempt to search past an ID3v2 tag or junk here, since
			// we only provided a fixed-sized buffer to search from.
			//
			// That case is handled in `Probe::guess_file_type`
			_ => None,
		}
	}

	// TODO: APE tags in the beginning of the file
	pub(crate) fn from_buffer_inner(buf: &[u8]) -> Option<FileTypeGuessResult> {
		use crate::id3::v2::util::synchsafe::SynchsafeInteger;

		// Start out with an empty return
		let mut ret = None;

		if buf.is_empty() {
			return ret;
		}

		match Self::quick_type_guess(buf) {
			Some(f_ty) => ret = Some(FileTypeGuessResult::Determined(f_ty)),
			// Special case for ID3, gets checked in `Probe::guess_file_type`
			// The bare minimum size for an ID3v2 header is 10 bytes
			None if buf.len() >= 10 && &buf[..3] == b"ID3" => {
				// This is infallible, but preferable to an unwrap
				if let Ok(arr) = buf[6..10].try_into() {
					// Set the ID3v2 size
					ret = Some(FileTypeGuessResult::MaybePrecededById3(
						u32::from_be_bytes(arr).unsynch(),
					));
				}
			},
			None => ret = Some(FileTypeGuessResult::MaybePrecededByJunk),
		}

		ret
	}

	fn quick_type_guess(buf: &[u8]) -> Option<Self> {
		use crate::mpeg::header::verify_frame_sync;

		// Safe to index, since we return early on an empty buffer
		match buf[0] {
			77 if buf.starts_with(b"MAC") => Some(Self::Ape),
			255 if buf.len() >= 2 && verify_frame_sync([buf[0], buf[1]]) => {
				// ADTS and MPEG frame headers are way too similar

				// ADTS (https://wiki.multimedia.cx/index.php/ADTS#Header):
				//
				// AAAAAAAA AAAABCCX
				//
				// Letter 	Length (bits) 	Description
				// A 	    12 	            Syncword, all bits must be set to 1.
				// B 	    1 	            MPEG Version, set to 0 for MPEG-4 and 1 for MPEG-2.
				// C 	    2 	            Layer, always set to 0.

				// MPEG (http://www.mp3-tech.org/programmer/frame_header.html):
				//
				// AAAAAAAA AAABBCCX
				//
				// Letter 	Length (bits) 	Description
				// A 	    11              Syncword, all bits must be set to 1.
				// B 	    2 	            MPEG Audio version ID
				// C 	    2 	            Layer description

				// The subtle overlap in the ADTS header's frame sync and MPEG's version ID
				// is the first condition to check. However, since 0b10 and 0b11 are valid versions
				// in MPEG, we have to also check the layer.

				// So, if we have a version 1 (0b11) or version 2 (0b10) MPEG frame AND a layer of 0b00,
				// we can assume we have an ADTS header. Awesome!

				if buf[1] & 0b10000 > 0 && buf[1] & 0b110 == 0 {
					return Some(Self::Aac);
				}

				Some(Self::Mpeg)
			},
			70 if buf.len() >= 12 && &buf[..4] == b"FORM" => {
				let id = &buf[8..12];

				if id == b"AIFF" || id == b"AIFC" {
					return Some(Self::Aiff);
				}

				None
			},
			79 if buf.len() >= 36 && &buf[..4] == b"OggS" => {
				if &buf[29..35] == b"vorbis" {
					return Some(Self::Vorbis);
				} else if &buf[28..36] == b"OpusHead" {
					return Some(Self::Opus);
				} else if &buf[28..36] == b"Speex   " {
					return Some(Self::Speex);
				}

				None
			},
			102 if buf.starts_with(b"fLaC") => Some(Self::Flac),
			82 if buf.len() >= 12 && &buf[..4] == b"RIFF" => {
				if &buf[8..12] == b"WAVE" {
					return Some(Self::Wav);
				}

				None
			},
			119 if buf.len() >= 4 && &buf[..4] == b"wvpk" => Some(Self::WavPack),
			_ if buf.len() >= 8 && &buf[4..8] == b"ftyp" => Some(Self::Mp4),
			_ if buf.starts_with(b"MPCK") || buf.starts_with(b"MP+") => Some(Self::Mpc),
			_ => None,
		}
	}
}

/// The result of a `FileType` guess
///
/// External callers of `FileType::from_buffer()` will only ever see `Determined` cases.
/// The remaining cases are used internally in `Probe::guess_file_type()`.
pub(crate) enum FileTypeGuessResult {
	/// The `FileType` was guessed
	Determined(FileType),
	/// The stream starts with an ID3v2 tag
	MaybePrecededById3(u32),
	/// The stream starts with potential junk data
	MaybePrecededByJunk,
}
