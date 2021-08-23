use crate::logic::id3::v2::Id3v2Version;
use crate::{LoftyError, Result};

use std::borrow::Cow;
use std::io::{Cursor, Read};
use std::io::{Seek, SeekFrom, Write};

use byteorder::WriteBytesExt;
#[cfg(any(feature = "vorbis_comments", feature = "id3v2",))]
use byteorder::{BigEndian, ReadBytesExt};

#[cfg(feature = "ape")]
pub const APE_PICTYPES: [&str; 21] = [
	"Other",
	"Png Icon",
	"Icon",
	"Front",
	"Back",
	"Leaflet",
	"Media",
	"Lead Artist",
	"Artist",
	"Conductor",
	"Band",
	"Composer",
	"Lyricist",
	"Recording Location",
	"During Recording",
	"During Performance",
	"Video Capture",
	"Fish",
	"Illustration",
	"Band Logotype",
	"Publisher Logotype",
];

/// Mime types for covers.
#[derive(Debug, Clone, Eq, PartialEq)]
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
	/// Some unknown mimetype
	Unknown(String),
	/// No mimetype
	None,
}

impl MimeType {
	/// Get a MimeType from a string
	pub fn from_str(mime_type: &str) -> Self {
		match &*mime_type.to_lowercase() {
			"image/jpeg" => Self::Jpeg,
			"image/png" => Self::Png,
			"image/tiff" => Self::Tiff,
			"image/bmp" => Self::Bmp,
			"image/gif" => Self::Gif,
			"" => Self::None,
			_ => Self::Unknown(mime_type.to_string()),
		}
	}

	/// Get a &str from a MimeType
	pub fn as_str(&self) -> &str {
		match self {
			MimeType::Jpeg => "image/jpeg",
			MimeType::Png => "image/png",
			MimeType::Tiff => "image/tiff",
			MimeType::Bmp => "image/bmp",
			MimeType::Gif => "image/gif",
			MimeType::Unknown(unknown) => &*unknown,
			MimeType::None => "",
		}
	}

	/// Get a String from a MimeType
	pub fn to_string(&self) -> String {
		match self {
			MimeType::Jpeg => "image/jpeg".to_string(),
			MimeType::Png => "image/png".to_string(),
			MimeType::Tiff => "image/tiff".to_string(),
			MimeType::Bmp => "image/bmp".to_string(),
			MimeType::Gif => "image/gif".to_string(),
			MimeType::Unknown(unknown) => unknown.to_owned(),
			MimeType::None => String::new(),
		}
	}
}

/// The picture type
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

	/// Get a u8 from a PictureType according to ID3v2 APIC
	#[cfg(any(feature = "id3v2", feature = "vorbis_comments"))]
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
			Self::Undefined(i) => u8::from(i.to_owned()),
		}
	}

	/// Get a PictureType from a u8 according to ID3v2 APIC
	#[cfg(any(feature = "id3v2", feature = "vorbis_comments"))]
	pub fn from_u8(bytes: u8) -> Self {
		match bytes {
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
			i => Self::Undefined(i as u8),
		}
	}

	// APE specific methods

	/// Get an APE item key from a PictureType
	#[cfg(feature = "ape")]
	pub fn as_ape_key(&self) -> &str {
		match self {
			Self::Other => "Cover Art (Other)",
			Self::Icon => "Cover Art (Png Icon)",
			Self::OtherIcon => "Cover Art (Icon)",
			Self::CoverFront => "Cover Art (Front)",
			Self::CoverBack => "Cover Art (Back)",
			Self::Leaflet => "Cover Art (Leaflet)",
			Self::Media => "Cover Art (Media)",
			Self::LeadArtist => "Cover Art (Lead Artist)",
			Self::Artist => "Cover Art (Artist)",
			Self::Conductor => "Cover Art (Conductor)",
			Self::Band => "Cover Art (Band)",
			Self::Composer => "Cover Art (Composer)",
			Self::Lyricist => "Cover Art (Lyricist)",
			Self::RecordingLocation => "Cover Art (Recording Location)",
			Self::DuringRecording => "Cover Art (During Recording)",
			Self::DuringPerformance => "Cover Art (During Performance)",
			Self::ScreenCapture => "Cover Art (Video Capture)",
			Self::BrightFish => "Cover Art (Fish)",
			Self::Illustration => "Cover Art (Illustration)",
			Self::BandLogo => "Cover Art (Band Logotype)",
			Self::PublisherLogo => "Cover Art (Publisher Logotype)",
			Self::Undefined(_) => "",
		}
	}

	/// Get a PictureType from an APE item key
	#[cfg(feature = "ape")]
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

/// The text encoding for use in ID3v2 APIC frames
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum TextEncoding {
	/// ISO-8859-1
	Latin1 = 0,
	/// UTF-16 with a byte order mark
	UTF16 = 1,
	/// UTF-16 big endian
	UTF16BE = 2,
	/// UTF-8
	UTF8 = 3,
}

impl TextEncoding {
	/// Get a TextEncoding from a u8, must be 0-3 inclusive
	pub fn from_u8(byte: u8) -> Option<Self> {
		match byte {
			0 => Some(Self::Latin1),
			1 => Some(Self::UTF16),
			2 => Some(Self::UTF16BE),
			3 => Some(Self::UTF8),
			_ => None,
		}
	}
}

/// Information about a [`Picture`]
#[derive(Debug, Clone, Eq, PartialEq)]
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

/// Represents a picture.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Picture {
	/// The picture type according to ID3v2 APIC
	pub pic_type: PictureType,
	/// **(ONLY APPLICABLE TO ID3v2)** The text encoding
	pub text_encoding: TextEncoding,
	/// The picture's mimetype
	pub mime_type: MimeType,
	/// The picture's description
	pub description: Option<Cow<'static, str>>,
	/// Basic information about the picture
	pub information: PictureInformation,
	/// The binary data of the picture
	pub data: Cow<'static, [u8]>,
}

impl Picture {
	/// Create a new `Picture`
	pub fn new(
		pic_type: PictureType,
		mime_type: MimeType,
		description: Option<String>,
		information: PictureInformation,
		data: Vec<u8>,
	) -> Self {
		Self {
			pic_type,
			mime_type,
			text_encoding: TextEncoding::UTF8,
			description: description.map(Cow::from),
			information,
			data: Cow::from(data),
		}
	}

	#[cfg(feature = "id3v2")]
	/// Convert a [`Picture`] to a ID3v2 A/PIC byte Vec
	///
	/// NOTE: This does not include the frame header
	///
	/// # Errors
	///
	/// ID3v2.2:
	///
	/// * The mimetype is not [`MimeType::Png`] or [`MimeType::Jpeg`]
	/// * Too much data was provided
	///
	/// ID3v2.3/4:
	///
	/// * Too much data was provided
	pub fn as_apic_bytes(&self, version: Id3v2Version) -> Result<Vec<u8>> {
		match version {
			Id3v2Version::V2 => {
				// ID3v2.2 PIC is pretty limited with formats
				let format = match self.mime_type {
					MimeType::Png => "PNG",
					MimeType::Jpeg => "JPG",
					_ => return Err(LoftyError::BadPictureFormat(self.mime_type.to_string())),
				};

				let mut data = vec![self.text_encoding as u8];

				data.write_all(format.as_bytes());
				data.write_u8(self.pic_type.as_u8());

				if let Some(description) = &self.description {
					data.write_all(&*crate::logic::id3::v2::util::text_utils::encode_text(
						description,
						self.text_encoding,
						true,
					))?;
				}

				data.write_u8(0)?;
				data.write_all(&*self.data);

				let size = data.len() - 6;

				if size as u64 > u32::MAX as u64 {
					return Err(LoftyError::TooMuchData);
				}

				let size = (size as u32).to_be_bytes();

				if size[0] != 0 {
					return Err(LoftyError::TooMuchData);
				}

				Ok(data)
			},
			_ => {
				let mut data = vec![self.text_encoding as u8];

				data.write_all(self.mime_type.as_str().as_bytes())?;
				data.write_u8(self.pic_type.as_u8())?;

				if let Some(description) = &self.description {
					data.write_all(&*crate::logic::id3::v2::util::text_utils::encode_text(
						description,
						self.text_encoding,
						true,
					))?;
				}

				data.write_u8(0)?;
				data.write_all(&*self.data)?;

				let size = data.len();

				if size as u64 > u32::MAX as u64 {
					return Err(LoftyError::TooMuchData);
				}

				Ok(data)
			},
		}
	}

	#[cfg(feature = "id3v2")]
	/// Get a [`Picture`] from ID3v2 A/PIC bytes:
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// This function will return [`NotAPicture`][LoftyError::NotAPicture] if at any point it's unable to parse the data
	pub fn from_apic_bytes(bytes: &[u8], version: Id3v2Version) -> Result<Self> {
		let mut cursor = Cursor::new(bytes);

		if let Some(encoding) = TextEncoding::from_u8(cursor.read_u8()?) {
			match version {
				Id3v2Version::V2 => {
					let mut format = [0; 3];
					cursor.read_exact(&mut format)?;

					let mime_type = match format {
						[b'P', b'N', b'G'] => MimeType::Png,
						[b'J', b'P', b'G'] => MimeType::Jpeg,
						_ => {
							return Err(LoftyError::BadPictureFormat(
								String::from_utf8_lossy(&format).to_string(),
							))
						},
					};

					let picture_type = PictureType::from_u8(cursor.read_u8()?);
					let description = crate::logic::id3::v2::util::text_utils::decode_text(
						&mut cursor,
						encoding,
						true,
					)?
					.map(|s| Cow::from(s));

					let mut data = Vec::new();
					cursor.read_to_end(&mut data)?;

					return Ok(Picture {
						pic_type: picture_type,
						text_encoding: encoding,
						mime_type,
						description,
						information: PictureInformation {
							width: 0,
							height: 0,
							color_depth: 0,
							num_colors: 0,
						},
						data: Cow::from(data),
					});
				},
				_ => {
					let mime_type = if let Some(mime_type) =
						crate::logic::id3::v2::util::text_utils::decode_text(
							&mut cursor,
							encoding,
							true,
						)? {
						MimeType::from_str(&*mime_type)
					} else {
						MimeType::None
					};

					let picture_type = PictureType::from_u8(cursor.read_u8()?);
					let description = crate::logic::id3::v2::util::text_utils::decode_text(
						&mut cursor,
						encoding,
						true,
					)?
					.map(|s| Cow::from(s));

					let mut data = Vec::new();
					cursor.read_to_end(&mut data)?;

					return Ok(Picture {
						pic_type: picture_type,
						text_encoding: encoding,
						mime_type,
						description,
						information: PictureInformation {
							width: 0,
							height: 0,
							color_depth: 0,
							num_colors: 0,
						},
						data: Cow::from(data),
					});
				},
			}
		}

		return Err(LoftyError::NotAPicture);
	}

	#[cfg(feature = "vorbis_comments")]
	/// Convert a [`Picture`] to a base64 encoded FLAC METADATA_BLOCK_PICTURE String
	///
	/// NOTES:
	///
	/// * This does not include a key (Vorbis comments) or METADATA_BLOCK_HEADER (FLAC blocks)
	/// * FLAC blocks have different size requirements than OGG Vorbis/Opus, size is not checked here
	pub fn as_flac_bytes(&self) -> String {
		let mut data = Vec::<u8>::new();

		let picture_type = (self.pic_type.as_u8() as u32).to_be_bytes();

		let mime_str = self.mime_type.to_string();
		let mime_len = mime_str.len() as u32;

		data.extend(picture_type.iter());
		data.extend(mime_len.to_be_bytes().iter());
		data.extend(mime_str.as_bytes().iter());

		if let Some(desc) = self.description.clone() {
			let desc_str = desc.to_string();
			let desc_len = desc_str.len() as u32;

			data.extend(desc_len.to_be_bytes().iter());
			data.extend(desc_str.as_bytes().iter());
		}

		data.extend(self.information.width.to_be_bytes().iter());
		data.extend(self.information.height.to_be_bytes().iter());
		data.extend(self.information.color_depth.to_be_bytes().iter());
		data.extend(self.information.num_colors.to_be_bytes().iter());

		let pic_data = &self.data;
		let pic_data_len = pic_data.len() as u32;

		data.extend(pic_data_len.to_be_bytes().iter());
		data.extend(pic_data.iter());

		base64::encode(data)
	}

	#[cfg(feature = "vorbis_comments")]
	/// Get a [`Picture`] from FLAC METADATA_BLOCK_PICTURE bytes (can be base64 encoded):
	///
	/// NOTE: This expects either the key "METADATA_BLOCK_PICTURE=" (Vorbis comments) or METADATA_BLOCK_HEADER (4 bytes, FLAC blocks) to have already been skipped
	///
	/// # Errors
	///
	/// This function will return [`NotAPicture`][LoftyError::NotAPicture] if at any point it's unable to parse the data
	pub fn from_flac_bytes(bytes: &[u8]) -> Result<Self> {
		let data = match base64::decode(bytes) {
			Ok(o) => o,
			Err(_) => bytes.to_vec(),
		};

		let mut cursor = Cursor::new(data);

		if let Ok(bytes) = cursor.read_u32::<BigEndian>() {
			let picture_type = PictureType::from_u8(bytes as u8);

			if let Ok(mime_len) = cursor.read_u32::<BigEndian>() {
				let mut buf = vec![0; mime_len as usize];
				cursor.read_exact(&mut buf)?;

				if let Ok(mime_type_str) = String::from_utf8(buf) {
					let mime_type = MimeType::from_str(&*mime_type_str);

					let mut description = None;

					if let Ok(desc_len) = cursor.read_u32::<BigEndian>() {
						if cursor.get_ref().len() >= (cursor.position() as u32 + desc_len) as usize
						{
							let mut buf = vec![0; desc_len as usize];
							cursor.read_exact(&mut buf)?;

							if let Ok(desc) = String::from_utf8(buf) {
								description = Some(Cow::from(desc));
							}
						} else {
							cursor.set_position(cursor.position() - 4)
						}
					}

					if let (Ok(width), Ok(height), Ok(color_depth), Ok(num_colors)) = (
						cursor.read_u32::<BigEndian>(),
						cursor.read_u32::<BigEndian>(),
						cursor.read_u32::<BigEndian>(),
						cursor.read_u32::<BigEndian>(),
					) {
						if let Ok(data_len) = cursor.read_u32::<BigEndian>() {
							let mut binary = vec![0; data_len as usize];

							if let Ok(()) = cursor.read_exact(&mut binary) {
								return Ok(Self {
									pic_type: picture_type,
									text_encoding: TextEncoding::UTF8,
									mime_type,
									description,
									information: PictureInformation {
										width,
										height,
										color_depth,
										num_colors,
									},
									data: Cow::from(binary.clone()),
								});
							}
						}
					}
				}
			}
		}

		Err(LoftyError::NotAPicture)
	}

	#[cfg(feature = "ape")]
	/// Convert a [`Picture`] to an APE Cover Art byte vec:
	///
	/// NOTE: This is only the picture data, a key and terminating null byte will not be prepended. To map a [`PictureType`] to an APE key see [`PictureType::as_ape_key`]
	pub fn as_ape_bytes(&self) -> Vec<u8> {
		let mut data: Vec<u8> = Vec::new();

		if let Some(desc) = &self.description {
			data.extend(desc.as_bytes().iter());
		}

		data.extend([0].iter());
		data.extend(self.data.iter());

		data
	}

	#[cfg(feature = "ape")]
	/// Get a [`Picture`] from an APEv2 binary item:
	///
	/// NOTES:
	///
	/// * This function expects the key and its trailing null byte to have been removed
	/// * Since APE tags only store the binary data, the width, height, color_depth, and num_colors fields will be zero.
	///
	/// # Errors
	///
	/// This function will return [`NotAPicture`](LoftyError::NotAPicture) if at any point it's unable to parse the data
	pub fn from_ape_bytes(key: &str, bytes: &[u8]) -> Result<Self> {
		if !bytes.is_empty() {
			let pic_type = PictureType::from_ape_key(key);

			let mut cursor = Cursor::new(bytes);

			let description = {
				let mut text = String::new();

				while let Ok(ch) = cursor.read_u8() {
					if ch != b'\0' {
						text.push(char::from(ch));
						continue;
					}

					break;
				}

				(!text.is_empty()).then(|| Cow::from(text))
			};

			let mime_type = {
				let mut identifier = [0; 4];
				cursor.read_exact(&mut identifier)?;

				cursor.seek(SeekFrom::Current(-4))?;

				match identifier {
					[0x89, b'P', b'N', b'G'] => MimeType::Png,
					[0xFF, 0xD8, ..] => MimeType::Jpeg,
					[b'G', b'I', b'F', ..] => MimeType::Gif,
					[b'B', b'M', ..] => MimeType::Bmp,
					[b'I', b'I', ..] => MimeType::Tiff,
					_ => return Err(LoftyError::NotAPicture),
				}
			};

			let pos = cursor.position() as usize;
			let data = Cow::from(cursor.into_inner()[pos..].to_vec());

			return Ok(Picture {
				pic_type,
				text_encoding: TextEncoding::UTF8,
				mime_type,
				description,
				information: PictureInformation {
					// TODO
					width: 0,
					height: 0,
					color_depth: 0,
					num_colors: 0,
				},
				data,
			});
		}

		Err(LoftyError::NotAPicture)
	}
}
