//! Format-agnostic picture handling

use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::err;
use crate::util::text::utf8_decode_str;

use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt as _};
use data_encoding::BASE64;

/// Common picture item keys for APE
pub const APE_PICTURE_TYPES: [&str; 21] = [
	"Cover Art (Other)",
	"Cover Art (Png Icon)",
	"Cover Art (Icon)",
	"Cover Art (Front)",
	"Cover Art (Back)",
	"Cover Art (Leaflet)",
	"Cover Art (Media)",
	"Cover Art (Lead Artist)",
	"Cover Art (Artist)",
	"Cover Art (Conductor)",
	"Cover Art (Band)",
	"Cover Art (Composer)",
	"Cover Art (Lyricist)",
	"Cover Art (Recording Location)",
	"Cover Art (During Recording)",
	"Cover Art (During Performance)",
	"Cover Art (Video Capture)",
	"Cover Art (Fish)",
	"Cover Art (Illustration)",
	"Cover Art (Band Logotype)",
	"Cover Art (Publisher Logotype)",
];

/// MIME types for pictures.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum MimeType {
	/// PNG image
	Png,
	/// JPEG image
	Jpeg,
	/// TIFF image
	Tiff,
	/// BMP image
	Bmp,
	/// GIF image
	Gif,
	/// Some unknown MIME type
	Unknown(String),
}

impl MimeType {
	/// Get a `MimeType` from a string
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::MimeType;
	///
	/// let jpeg_mimetype_str = "image/jpeg";
	/// assert_eq!(MimeType::from_str(jpeg_mimetype_str), MimeType::Jpeg);
	/// ```
	#[must_use]
	#[allow(clippy::should_implement_trait)] // Infallible in contrast to FromStr
	pub fn from_str(mime_type: &str) -> Self {
		match &*mime_type.to_lowercase() {
			"image/jpeg" | "image/jpg" => Self::Jpeg,
			"image/png" => Self::Png,
			"image/tiff" => Self::Tiff,
			"image/bmp" => Self::Bmp,
			"image/gif" => Self::Gif,
			_ => Self::Unknown(mime_type.to_owned()),
		}
	}

	/// Get a &str from a `MimeType`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::MimeType;
	///
	/// let jpeg_mimetype = MimeType::Jpeg;
	/// assert_eq!(jpeg_mimetype.as_str(), "image/jpeg")
	/// ```
	#[must_use]
	pub fn as_str(&self) -> &str {
		match self {
			MimeType::Jpeg => "image/jpeg",
			MimeType::Png => "image/png",
			MimeType::Tiff => "image/tiff",
			MimeType::Bmp => "image/bmp",
			MimeType::Gif => "image/gif",
			MimeType::Unknown(unknown) => unknown,
		}
	}

	/// Returns the extension for the `MimeType` if it is known
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::MimeType;
	///
	/// assert_eq!(MimeType::Jpeg.ext(), Some("jpg"));
	/// ```
	pub fn ext(&self) -> Option<&str> {
		match self {
			MimeType::Jpeg => Some("jpg"),
			MimeType::Png => Some("png"),
			MimeType::Tiff => Some("tif"),
			MimeType::Bmp => Some("bmp"),
			MimeType::Gif => Some("gif"),
			MimeType::Unknown(_) => None,
		}
	}
}

impl Display for MimeType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

/// The picture type, according to ID3v2 APIC
#[allow(missing_docs)]
#[allow(clippy::unsafe_derive_deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum PictureType {
	Other,
	Icon,
	OtherIcon,
	CoverFront,
	CoverBack,
	Leaflet,
	Media,
	LeadArtist,
	Artist,
	Conductor,
	Band,
	Composer,
	Lyricist,
	RecordingLocation,
	DuringRecording,
	DuringPerformance,
	ScreenCapture,
	BrightFish,
	Illustration,
	BandLogo,
	PublisherLogo,
	Undefined(u8),
}

impl PictureType {
	// ID3/OGG specific methods

	/// Get a `u8` from a `PictureType` according to ID3v2 APIC
	pub fn as_u8(&self) -> u8 {
		match self {
			Self::Other => 0,
			Self::Icon => 1,
			Self::OtherIcon => 2,
			Self::CoverFront => 3,
			Self::CoverBack => 4,
			Self::Leaflet => 5,
			Self::Media => 6,
			Self::LeadArtist => 7,
			Self::Artist => 8,
			Self::Conductor => 9,
			Self::Band => 10,
			Self::Composer => 11,
			Self::Lyricist => 12,
			Self::RecordingLocation => 13,
			Self::DuringRecording => 14,
			Self::DuringPerformance => 15,
			Self::ScreenCapture => 16,
			Self::BrightFish => 17,
			Self::Illustration => 18,
			Self::BandLogo => 19,
			Self::PublisherLogo => 20,
			Self::Undefined(i) => *i,
		}
	}

	/// Get a `PictureType` from a u8 according to ID3v2 APIC
	pub fn from_u8(byte: u8) -> Self {
		match byte {
			0 => Self::Other,
			1 => Self::Icon,
			2 => Self::OtherIcon,
			3 => Self::CoverFront,
			4 => Self::CoverBack,
			5 => Self::Leaflet,
			6 => Self::Media,
			7 => Self::LeadArtist,
			8 => Self::Artist,
			9 => Self::Conductor,
			10 => Self::Band,
			11 => Self::Composer,
			12 => Self::Lyricist,
			13 => Self::RecordingLocation,
			14 => Self::DuringRecording,
			15 => Self::DuringPerformance,
			16 => Self::ScreenCapture,
			17 => Self::BrightFish,
			18 => Self::Illustration,
			19 => Self::BandLogo,
			20 => Self::PublisherLogo,
			i => Self::Undefined(i),
		}
	}

	// APE specific methods

	/// Get an APE item key from a `PictureType`
	pub fn as_ape_key(&self) -> Option<&str> {
		match self {
			Self::Other => Some("Cover Art (Other)"),
			Self::Icon => Some("Cover Art (Png Icon)"),
			Self::OtherIcon => Some("Cover Art (Icon)"),
			Self::CoverFront => Some("Cover Art (Front)"),
			Self::CoverBack => Some("Cover Art (Back)"),
			Self::Leaflet => Some("Cover Art (Leaflet)"),
			Self::Media => Some("Cover Art (Media)"),
			Self::LeadArtist => Some("Cover Art (Lead Artist)"),
			Self::Artist => Some("Cover Art (Artist)"),
			Self::Conductor => Some("Cover Art (Conductor)"),
			Self::Band => Some("Cover Art (Band)"),
			Self::Composer => Some("Cover Art (Composer)"),
			Self::Lyricist => Some("Cover Art (Lyricist)"),
			Self::RecordingLocation => Some("Cover Art (Recording Location)"),
			Self::DuringRecording => Some("Cover Art (During Recording)"),
			Self::DuringPerformance => Some("Cover Art (During Performance)"),
			Self::ScreenCapture => Some("Cover Art (Video Capture)"),
			Self::BrightFish => Some("Cover Art (Fish)"),
			Self::Illustration => Some("Cover Art (Illustration)"),
			Self::BandLogo => Some("Cover Art (Band Logotype)"),
			Self::PublisherLogo => Some("Cover Art (Publisher Logotype)"),
			Self::Undefined(_) => None,
		}
	}

	/// Get a `PictureType` from an APE item key
	pub fn from_ape_key(key: &str) -> Self {
		match key {
			"Cover Art (Other)" => Self::Other,
			"Cover Art (Png Icon)" => Self::Icon,
			"Cover Art (Icon)" => Self::OtherIcon,
			"Cover Art (Front)" => Self::CoverFront,
			"Cover Art (Back)" => Self::CoverBack,
			"Cover Art (Leaflet)" => Self::Leaflet,
			"Cover Art (Media)" => Self::Media,
			"Cover Art (Lead Artist)" => Self::LeadArtist,
			"Cover Art (Artist)" => Self::Artist,
			"Cover Art (Conductor)" => Self::Conductor,
			"Cover Art (Band)" => Self::Band,
			"Cover Art (Composer)" => Self::Composer,
			"Cover Art (Lyricist)" => Self::Lyricist,
			"Cover Art (Recording Location)" => Self::RecordingLocation,
			"Cover Art (During Recording)" => Self::DuringRecording,
			"Cover Art (During Performance)" => Self::DuringPerformance,
			"Cover Art (Video Capture)" => Self::ScreenCapture,
			"Cover Art (Fish)" => Self::BrightFish,
			"Cover Art (Illustration)" => Self::Illustration,
			"Cover Art (Band Logotype)" => Self::BandLogo,
			"Cover Art (Publisher Logotype)" => Self::PublisherLogo,
			_ => Self::Undefined(0),
		}
	}
}

/// Information about a [`Picture`]
///
/// This information is necessary for FLAC's `METADATA_BLOCK_PICTURE`.
/// See [`Picture::as_flac_bytes`] for more information.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct PictureInformation {
	/// The picture's width in pixels
	pub width: u32,
	/// The picture's height in pixels
	pub height: u32,
	/// The picture's color depth in bits per pixel
	pub color_depth: u32,
	/// The number of colors used
	pub num_colors: u32,
}

impl PictureInformation {
	/// Attempt to extract [`PictureInformation`] from a [`Picture`]
	///
	/// NOTE: This only supports PNG and JPEG images. If another image is provided,
	/// the `PictureInformation` will be zeroed out.
	///
	/// # Errors
	///
	/// * `picture.data` is less than 8 bytes in length
	/// * See [`PictureInformation::from_png`] and [`PictureInformation::from_jpeg`]
	pub fn from_picture(picture: &Picture) -> Result<Self> {
		let reader = &mut &*picture.data;

		if reader.len() < 8 {
			err!(NotAPicture);
		}

		match reader[..4] {
			[0x89, b'P', b'N', b'G'] => Ok(Self::from_png(reader).unwrap_or_default()),
			[0xFF, 0xD8, 0xFF, ..] => Ok(Self::from_jpeg(reader).unwrap_or_default()),
			_ => Ok(Self::default()),
		}
	}

	/// Attempt to extract [`PictureInformation`] from a PNG
	///
	/// # Errors
	///
	/// * `reader` is not a valid PNG
	pub fn from_png(mut data: &[u8]) -> Result<Self> {
		let reader = &mut data;

		let mut sig = [0; 8];
		reader.read_exact(&mut sig)?;

		if sig != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
			err!(NotAPicture);
		}

		let mut ihdr = [0; 8];
		reader.read_exact(&mut ihdr)?;

		// Verify the signature is immediately followed by the IHDR chunk
		if !ihdr.ends_with(&[0x49, 0x48, 0x44, 0x52]) {
			err!(NotAPicture);
		}

		let width = reader.read_u32::<BigEndian>()?;
		let height = reader.read_u32::<BigEndian>()?;
		let mut color_depth = u32::from(reader.read_u8()?);
		let color_type = reader.read_u8()?;

		match color_type {
			2 => color_depth *= 3,
			4 | 6 => color_depth *= 4,
			_ => {},
		}

		let mut ret = Self {
			width,
			height,
			color_depth,
			num_colors: 0,
		};

		// The color type 3 (indexed-color) means there should be
		// a "PLTE" chunk, whose data can be used in the `num_colors`
		// field. It isn't really applicable to other color types.
		if color_type != 3 {
			return Ok(ret);
		}

		let mut reader = Cursor::new(reader);

		// Skip 7 bytes
		// Compression method (1)
		// Filter method (1)
		// Interlace method (1)
		// CRC (4)
		reader.seek(SeekFrom::Current(7))?;

		let mut chunk_type = [0; 4];

		while let (Ok(size), Ok(())) = (
			reader.read_u32::<BigEndian>(),
			reader.read_exact(&mut chunk_type),
		) {
			if &chunk_type == b"PLTE" {
				// The PLTE chunk contains 1-256 3-byte entries
				ret.num_colors = size / 3;
				break;
			}

			// Skip the chunk's data (size) and CRC (4 bytes)
			let (content_size, overflowed) = size.overflowing_add(4);
			if overflowed {
				break;
			}

			reader.seek(SeekFrom::Current(i64::from(content_size)))?;
		}

		Ok(ret)
	}

	/// Attempt to extract [`PictureInformation`] from a JPEG
	///
	/// # Errors
	///
	/// * `reader` is not a JPEG image
	/// * `reader` does not contain a `SOFn` frame
	pub fn from_jpeg(mut data: &[u8]) -> Result<Self> {
		let reader = &mut data;

		let mut frame_marker = [0; 4];
		reader.read_exact(&mut frame_marker)?;

		if !matches!(frame_marker, [0xFF, 0xD8, 0xFF, ..]) {
			err!(NotAPicture);
		}

		let mut section_len = reader.read_u16::<BigEndian>()?;

		let mut reader = Cursor::new(reader);

		// The length contains itself, so anything < 2 is invalid
		let (content_len, overflowed) = section_len.overflowing_sub(2);
		if overflowed {
			err!(NotAPicture);
		}
		reader.seek(SeekFrom::Current(i64::from(content_len)))?;

		while let Ok(0xFF) = reader.read_u8() {
			let marker = reader.read_u8()?;
			section_len = reader.read_u16::<BigEndian>()?;

			// This marks the SOS (Start of Scan), which is
			// the end of the header
			if marker == 0xDA {
				break;
			}

			// We are looking for a frame with a "SOFn" marker,
			// with `n` either being 0 or 2. Since there isn't a
			// header like PNG, we actually need to search for this
			// frame
			if marker == 0xC0 || marker == 0xC2 {
				let precision = reader.read_u8()?;
				let height = u32::from(reader.read_u16::<BigEndian>()?);
				let width = u32::from(reader.read_u16::<BigEndian>()?);
				let components = reader.read_u8()?;

				return Ok(Self {
					width,
					height,
					color_depth: u32::from(precision * components),
					num_colors: 0,
				});
			}

			reader.seek(SeekFrom::Current(i64::from(section_len - 2)))?;
		}

		err!(NotAPicture)
	}
}

/// Builder for a [`Picture`]
///
/// This is created through [`Picture::unchecked()`].
pub struct PictureBuilder {
	pic_type: PictureType,
	mime_type: Option<MimeType>,
	description: Option<Cow<'static, str>>,
	data: Cow<'static, [u8]>,
}

impl PictureBuilder {
	fn new(data: Cow<'static, [u8]>) -> Self {
		Self {
			pic_type: PictureType::Other,
			mime_type: None,
			description: None,
			data,
		}
	}

	/// Set the [`PictureType`] for this picture
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::{Picture, PictureType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let picture_path = "band.jpg";
	/// # let picture_path = "tests/files/assets/issue_37.jpg";
	/// let picture_data = std::fs::read(picture_path)?;
	///
	/// let picture = Picture::unchecked(picture_data)
	/// 	.pic_type(PictureType::Band)
	/// 	.build();
	/// # Ok(()) }
	/// ```
	pub fn pic_type(mut self, pic_type: PictureType) -> Self {
		self.pic_type = pic_type;
		self
	}

	/// Set the [`PictureType`] for this picture
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::{MimeType, Picture};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let picture_path = "band.jpg";
	/// # let picture_path = "tests/files/assets/issue_37.jpg";
	/// let picture_data = std::fs::read(picture_path)?;
	///
	/// let picture = Picture::unchecked(picture_data)
	/// 	.mime_type(MimeType::Jpeg)
	/// 	.build();
	/// # Ok(()) }
	/// ```
	pub fn mime_type(mut self, mime_type: MimeType) -> Self {
		self.mime_type = Some(mime_type);
		self
	}

	/// Set the [`PictureType`] for this picture
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::Picture;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let picture_path = "band.jpg";
	/// # let picture_path = "tests/files/assets/issue_37.jpg";
	/// let picture_data = std::fs::read(picture_path)?;
	///
	/// let picture = Picture::unchecked(picture_data)
	/// 	.description("The band on stage")
	/// 	.build();
	/// # Ok(()) }
	/// ```
	pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Convert this builder into a [`Picture`]
	pub fn build(self) -> Picture {
		self.into()
	}
}

impl From<PictureBuilder> for Picture {
	fn from(builder: PictureBuilder) -> Self {
		Self {
			pic_type: builder.pic_type,
			mime_type: builder.mime_type,
			description: builder.description,
			data: builder.data,
		}
	}
}

/// Represents a picture.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Picture {
	/// The picture type according to ID3v2 APIC
	pub(crate) pic_type: PictureType,
	/// The picture's mimetype
	pub(crate) mime_type: Option<MimeType>,
	/// The picture's description
	pub(crate) description: Option<Cow<'static, str>>,
	/// The binary data of the picture
	pub(crate) data: Cow<'static, [u8]>,
}

impl Debug for Picture {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Picture")
			.field("pic_type", &self.pic_type)
			.field("mime_type", &self.mime_type)
			.field("description", &self.description)
			.field("data", &format!("<{} bytes>", self.data.len()))
			.finish()
	}
}

impl Picture {
	/// Placeholder for conversions
	pub(crate) const EMPTY: Self = Picture {
		pic_type: PictureType::Other,
		mime_type: None,
		description: None,
		data: Cow::Owned(Vec::new()),
	};

	/// Create a [`Picture`] from a reader
	///
	/// NOTES:
	///
	/// * This is for reading picture data only, from a [`File`](std::fs::File) for example.
	/// * `pic_type` will always be [`PictureType::Other`], be sure to change it accordingly if
	///   writing.
	///
	/// # Errors
	///
	/// * `reader` contains less than 8 bytes
	/// * `reader` does not contain a supported format. See [`MimeType`] for valid formats
	pub fn from_reader<R>(reader: &mut R) -> Result<Self>
	where
		R: Read,
	{
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;

		if data.len() < 8 {
			err!(NotAPicture);
		}

		let mime_type = Self::mimetype_from_bin(&data[..8])?;

		Ok(Self {
			pic_type: PictureType::Other,
			mime_type: Some(mime_type),
			description: None,
			data: data.into(),
		})
	}

	/// Create a new `Picture` with no verification
	///
	/// This will **not** verify `data`'s signature.
	///
	/// This should only be used if all data has been verified beforehand.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::picture::{MimeType, Picture, PictureType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let picture_path = "band.jpg";
	/// # let picture_path = "tests/files/assets/issue_37.jpg";
	/// let picture_data = std::fs::read(picture_path)?;
	///
	/// let picture = Picture::unchecked(picture_data)
	/// 	.pic_type(PictureType::Band)
	/// 	.mime_type(MimeType::Jpeg)
	/// 	.description("The band on stage")
	/// 	.build();
	/// # Ok(()) }
	/// ```
	pub fn unchecked(data: Vec<u8>) -> PictureBuilder {
		PictureBuilder::new(Cow::Owned(data))
	}

	/// Returns the [`PictureType`]
	pub fn pic_type(&self) -> PictureType {
		self.pic_type
	}

	/// Sets the [`PictureType`]
	pub fn set_pic_type(&mut self, pic_type: PictureType) {
		self.pic_type = pic_type
	}

	/// Returns the [`MimeType`]
	///
	/// The `mime_type` is determined from the `data`, and
	/// is immutable.
	pub fn mime_type(&self) -> Option<&MimeType> {
		self.mime_type.as_ref()
	}

	// Used commonly internally
	pub(crate) fn mime_str(&self) -> &str {
		match self.mime_type.as_ref() {
			Some(mime_type) => mime_type.as_str(),
			None => "",
		}
	}

	/// Returns the description
	pub fn description(&self) -> Option<&str> {
		self.description.as_deref()
	}

	/// Sets the description
	pub fn set_description(&mut self, description: Option<String>) {
		self.description = description.map(Cow::from);
	}

	/// Returns the [`Picture`] data as borrowed bytes.
	pub fn data(&self) -> &[u8] {
		&self.data
	}

	/// Consumes a [`Picture`], returning the data as [`Vec`] without clones or allocation.
	pub fn into_data(self) -> Vec<u8> {
		self.data.into_owned()
	}

	/// Convert a [`Picture`] to a base64 encoded FLAC `METADATA_BLOCK_PICTURE` String
	///
	/// Use `encode` to convert the picture to a base64 encoded String ([RFC 4648 §4](http://www.faqs.org/rfcs/rfc4648.html))
	///
	/// NOTES:
	///
	/// * This does not include a key (Vorbis comments) or METADATA_BLOCK_HEADER (FLAC blocks)
	/// * FLAC blocks have different size requirements than OGG Vorbis/Opus, size is not checked here
	/// * When writing to Vorbis comments, the data **must** be base64 encoded
	pub fn as_flac_bytes(&self, picture_information: PictureInformation, encode: bool) -> Vec<u8> {
		let mut data = Vec::<u8>::new();

		let picture_type = u32::from(self.pic_type.as_u8()).to_be_bytes();

		let mime_str = self.mime_str();
		let mime_len = mime_str.len() as u32;

		data.extend(picture_type);
		data.extend(mime_len.to_be_bytes());
		data.extend(mime_str.as_bytes());

		if let Some(desc) = &self.description {
			let desc_len = desc.len() as u32;

			data.extend(desc_len.to_be_bytes());
			data.extend(desc.as_bytes());
		} else {
			data.extend([0; 4]);
		}

		data.extend(picture_information.width.to_be_bytes());
		data.extend(picture_information.height.to_be_bytes());
		data.extend(picture_information.color_depth.to_be_bytes());
		data.extend(picture_information.num_colors.to_be_bytes());

		let pic_data = &self.data;
		let pic_data_len = pic_data.len() as u32;

		data.extend(pic_data_len.to_be_bytes());
		data.extend(pic_data.iter());

		if encode {
			BASE64.encode(&data).into_bytes()
		} else {
			data
		}
	}

	/// Get a [`Picture`] from FLAC `METADATA_BLOCK_PICTURE` bytes:
	///
	/// NOTE: This takes both the base64 encoded string from Vorbis comments, and
	/// the raw data from a FLAC block, specified with `encoded`.
	///
	/// # Errors
	///
	/// This function will return [`NotAPicture`][ErrorKind::NotAPicture] if
	/// at any point it's unable to parse the data
	pub fn from_flac_bytes(
		bytes: &[u8],
		encoded: bool,
		parse_mode: ParsingMode,
	) -> Result<(Self, PictureInformation)> {
		if encoded {
			let data = BASE64
				.decode(bytes)
				.map_err(|_| LoftyError::new(ErrorKind::NotAPicture))?;
			Self::from_flac_bytes_inner(&data, parse_mode)
		} else {
			Self::from_flac_bytes_inner(bytes, parse_mode)
		}
	}

	fn from_flac_bytes_inner(
		content: &[u8],
		parse_mode: ParsingMode,
	) -> Result<(Self, PictureInformation)> {
		use crate::macros::try_vec;

		let mut size = content.len();
		let mut reader = Cursor::new(content);

		if size < 32 {
			err!(NotAPicture);
		}

		let pic_ty = reader.read_u32::<BigEndian>()?;
		size -= 4;

		// ID3v2 APIC uses a single byte for picture type.
		// Anything greater than that is probably invalid, so
		// we just stop early
		if pic_ty > 255 && parse_mode == ParsingMode::Strict {
			err!(NotAPicture);
		}

		let mime_len = reader.read_u32::<BigEndian>()? as usize;
		size -= 4;

		if mime_len > size {
			err!(SizeMismatch);
		}

		let mime_type_str = utf8_decode_str(&content[8..8 + mime_len])?;
		size -= mime_len;

		reader.seek(SeekFrom::Current(mime_len as i64))?;

		let desc_len = reader.read_u32::<BigEndian>()? as usize;
		size -= 4;

		let mut description = None;
		if desc_len > 0 && desc_len < size {
			let pos = 12 + mime_len;

			if let Ok(desc) = utf8_decode_str(&content[pos..pos + desc_len]) {
				description = Some(desc.to_owned().into());
			}

			size -= desc_len;
			reader.seek(SeekFrom::Current(desc_len as i64))?;
		}

		let width = reader.read_u32::<BigEndian>()?;
		let height = reader.read_u32::<BigEndian>()?;
		let color_depth = reader.read_u32::<BigEndian>()?;
		let num_colors = reader.read_u32::<BigEndian>()?;
		let data_len = reader.read_u32::<BigEndian>()? as usize;
		size -= 20;

		if data_len <= size {
			let mut data = try_vec![0; data_len];

			if let Ok(()) = reader.read_exact(&mut data) {
				let mime_type;
				if mime_type_str.is_empty() {
					mime_type = None;
				} else {
					mime_type = Some(MimeType::from_str(mime_type_str));
				}

				return Ok((
					Self {
						pic_type: PictureType::from_u8(pic_ty as u8),
						mime_type,
						description,
						data: Cow::from(data),
					},
					PictureInformation {
						width,
						height,
						color_depth,
						num_colors,
					},
				));
			}
		}

		err!(NotAPicture)
	}

	/// Convert a [`Picture`] to an APE Cover Art byte vec:
	///
	/// NOTE: This is only the picture data and description, a
	/// key and terminating null byte will not be prepended.
	/// To map a [`PictureType`] to an APE key see [`PictureType::as_ape_key`]
	pub fn as_ape_bytes(&self) -> Vec<u8> {
		let mut data: Vec<u8> = Vec::new();

		if let Some(desc) = &self.description {
			data.extend(desc.as_bytes());
		}

		data.push(0);
		data.extend(self.data.iter());

		data
	}

	/// Get a [`Picture`] from an APEv2 binary item:
	///
	/// NOTE: This function expects `bytes` to contain *only* the APE item data
	///
	/// # Errors
	///
	/// This function will return [`NotAPicture`](ErrorKind::NotAPicture)
	/// if at any point it's unable to parse the data
	pub fn from_ape_bytes(key: &str, bytes: &[u8]) -> Result<Self> {
		if bytes.is_empty() {
			err!(NotAPicture);
		}

		let pic_type = PictureType::from_ape_key(key);

		let reader = &mut &*bytes;
		let mut pos = 0;

		let mut description = None;
		let mut desc_text = String::new();

		while let Ok(ch) = reader.read_u8() {
			pos += 1;

			if ch == b'\0' {
				break;
			}

			desc_text.push(char::from(ch));
		}

		if !desc_text.is_empty() {
			description = Some(Cow::from(desc_text));
		}

		let mime_type = {
			let mut identifier = [0; 8];
			reader.read_exact(&mut identifier)?;

			Self::mimetype_from_bin(&identifier[..])?
		};

		let data = Cow::from(bytes[pos..].to_vec());

		Ok(Picture {
			pic_type,
			mime_type: Some(mime_type),
			description,
			data,
		})
	}

	pub(crate) fn mimetype_from_bin(bytes: &[u8]) -> Result<MimeType> {
		match bytes[..8] {
			[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] => Ok(MimeType::Png),
			[0xFF, 0xD8, ..] => Ok(MimeType::Jpeg),
			[b'G', b'I', b'F', 0x38, 0x37 | 0x39, b'a', ..] => Ok(MimeType::Gif),
			[b'B', b'M', ..] => Ok(MimeType::Bmp),
			[b'I', b'I', b'*', 0x00, ..] | [b'M', b'M', 0x00, b'*', ..] => Ok(MimeType::Tiff),
			_ => err!(NotAPicture),
		}
	}
}

impl From<Picture> for Cow<'_, Picture> {
	fn from(pic: Picture) -> Self {
		Cow::Owned(pic)
	}
}

impl<'a> From<&'a Picture> for Cow<'a, Picture> {
	fn from(pic: &'a Picture) -> Self {
		Cow::Borrowed(pic)
	}
}
