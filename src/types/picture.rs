use crate::{Error, Result};

use byteorder::{BigEndian, ReadBytesExt};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::io::{Cursor, Read};

#[cfg(feature = "format-ape")]
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
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
}

impl MimeType {
	/// Converts the `MimeType` to an ape str
	pub fn as_ape(self) -> &'static [u8; 4] {
		match self {
			MimeType::Png => b"PNG\0",
			MimeType::Jpeg => b"JPEG",
			MimeType::Tiff => b"TIFF",
			MimeType::Bmp => b"BMP\0",
			MimeType::Gif => b"GIF\0",
		}
	}
}

impl TryFrom<&str> for MimeType {
	type Error = Error;
	fn try_from(inp: &str) -> Result<Self> {
		Ok(match inp {
			"image/jpeg" => MimeType::Jpeg,
			"image/png" => MimeType::Png,
			"image/tiff" => MimeType::Tiff,
			"image/bmp" => MimeType::Bmp,
			"image/gif" => MimeType::Gif,
			_ => return Err(Error::UnsupportedMimeType(inp.to_owned())),
		})
	}
}

impl From<MimeType> for &'static str {
	fn from(mt: MimeType) -> Self {
		match mt {
			MimeType::Jpeg => "image/jpeg",
			MimeType::Png => "image/png",
			MimeType::Tiff => "image/tiff",
			MimeType::Bmp => "image/bmp",
			MimeType::Gif => "image/gif",
		}
	}
}

impl From<MimeType> for String {
	fn from(mt: MimeType) -> Self {
		<MimeType as Into<&'static str>>::into(mt).to_owned()
	}
}

pub trait PicType {
	#[cfg(any(
		feature = "format-id3",
		feature = "format-vorbis",
		feature = "format-opus",
		feature = "format-flac"
	))]
	fn as_u32(&self) -> u32;
	#[cfg(feature = "format-ape")]
	fn as_ape_key(&self) -> &str;
	#[cfg(any(
		feature = "format-id3",
		feature = "format-vorbis",
		feature = "format-opus",
		feature = "format-flac"
	))]
	fn from_u32(bytes: u32) -> PictureType;
	#[cfg(feature = "format-ape")]
	fn from_ape_key(key: &str) -> PictureType;
}

/// The picture type
#[cfg(not(feature = "format-id3"))]
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

/// Alias for PictureType
#[cfg(not(feature = "format-id3"))]
pub type PictureType = PictureType;
/// Alias for PictureType
#[cfg(feature = "format-id3")]
pub type PictureType = id3::frame::PictureType;

impl PicType for PictureType {
	#[cfg(any(
		feature = "format-id3",
		feature = "format-vorbis",
		feature = "format-opus",
		feature = "format-flac"
	))]
	fn as_u32(&self) -> u32 {
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
			Self::Undefined(i) => u32::from(i.to_owned()),
		}
	}

	#[cfg(feature = "format-ape")]
	fn as_ape_key(&self) -> &str {
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

	#[cfg(any(
		feature = "format-id3",
		feature = "format-vorbis",
		feature = "format-opus",
		feature = "format-flac"
	))]
	fn from_u32(bytes: u32) -> Self {
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

	#[cfg(feature = "format-ape")]
	fn from_ape_key(key: &str) -> Self {
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

/// Represents a picture, with its data and mime type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Picture {
	/// The picture type
	pub pic_type: PictureType,
	/// The picture's mimetype
	pub mime_type: MimeType,
	/// The picture's description
	pub description: Option<Cow<'static, str>>,
	/// The picture's actual data
	pub data: Cow<'static, [u8]>,
}

impl Picture {
	/// Create a new `Picture`
	pub fn new(
		pic_type: PictureType,
		mime_type: MimeType,
		description: Option<Cow<'static, str>>,
		data: Cow<'static, [u8]>,
	) -> Self {
		Self {
			pic_type,
			mime_type,
			description,
			data,
		}
	}
	/// Convert the `Picture` back to an APIC byte vec:
	///
	/// * Id3v2 APIC
	/// * Vorbis METADATA_BLOCK_PICTURE
	pub fn as_apic_bytes(&self) -> Vec<u8> {
		let picture_type = self.pic_type.as_u32().to_be_bytes();

		let mime_str = String::from(self.mime_type);
		let mime_len = mime_str.len() as u32;

		let mut data: Vec<u8> = Vec::new();
		data.extend(picture_type.iter());
		data.extend(mime_len.to_be_bytes().iter());
		data.extend(mime_str.as_bytes().iter());

		if let Some(desc) = self.description.clone() {
			let desc_str = desc.to_string();
			let desc_len = desc_str.len() as u32;

			data.extend(desc_len.to_be_bytes().iter());
			data.extend(desc_str.as_bytes().iter());
		}

		let pic_data = &self.data;
		let pic_data_len = pic_data.len() as u32;

		data.extend(pic_data_len.to_be_bytes().iter());
		data.extend(pic_data.iter());

		data
	}
	/// Get a `Picture` from APIC bytes:
	///
	/// * Id3v2 APIC
	/// * Vorbis METADATA_BLOCK_PICTURE
	///
	/// # Errors
	///
	/// This function will return [`Error::InvalidData`] if at any point it's unable to parse the data
	pub fn from_apic_bytes(bytes: &[u8]) -> Result<Self> {
		let data = match base64::decode(bytes) {
			Ok(o) => o,
			Err(_) => bytes.to_vec(),
		};

		let mut cursor = Cursor::new(data);

		if let Ok(bytes) = cursor.read_u32::<BigEndian>() {
			let picture_type = PictureType::from_u32(bytes);

			if let Ok(mime_len) = cursor.read_u32::<BigEndian>() {
				let mut buf = vec![0; mime_len as usize];
				cursor.read_exact(&mut buf)?;

				if let Ok(mime_type_str) = String::from_utf8(buf) {
					if let Ok(mime_type) = MimeType::try_from(&*mime_type_str) {
						let mut description = None;

						if let Ok(desc_len) = cursor.read_u32::<BigEndian>() {
							if cursor.get_ref().len()
								>= (cursor.position() as u32 + desc_len) as usize
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

						if let Ok(_data_len) = cursor.read_u32::<BigEndian>() {
							let pos = (cursor.position()) as usize;
							let content = &cursor.into_inner()[pos..];

							return Ok(Self {
								pic_type: picture_type,
								mime_type,
								description,
								data: Cow::from(content.to_vec()),
							});
						}
					}
				}
			}
		}

		Err(Error::InvalidData)
	}
	/// Convert the `Picture` back to an APEv2 byte vec:
	///
	/// * APEv2 Cover Art
	pub fn as_ape_bytes(&self) -> Vec<u8> {
		const NULL: [u8; 1] = [0];

		let mut data: Vec<u8> = Vec::new();

		if let Some(desc) = self.description.clone() {
			let desc_str = desc.to_string();
			data.extend(desc_str.as_bytes().iter());
			data.extend(NULL.iter());
		}

		data.extend(self.mime_type.as_ape().iter());
		data.extend(NULL.iter());
		data.extend(self.data.iter());

		data
	}
	/// Get a `Picture` from APEv2 bytes:
	///
	/// * APEv2 Cover Art
	///
	/// # Errors
	///
	/// This function will return [`Error::InvalidData`] if at any point it's unable to parse the data
	pub fn from_ape_bytes(key: &str, bytes: &[u8]) -> Result<Self> {
		if !bytes.is_empty() {
			fn read_text(c: &mut Cursor<Vec<u8>>) -> String {
				let mut text = String::new();

				while let Ok(ch) = c.read_u8() {
					if ch != b'\0' {
						text.push(char::from(ch));
						continue;
					}

					break;
				}

				text
			}

			let pic_type = PictureType::from_ape_key(key);

			let mut description = None;
			let mut mime_type = None;

			let mut cursor = Cursor::new(bytes.to_vec());

			let mut i = 0;

			loop {
				i += 1;

				if i == 3 {
					break;
				}

				let text = read_text(&mut cursor);

				let mime = match text.as_bytes() {
					b"PNG\0" => Some(MimeType::Png),
					b"JPEG" => Some(MimeType::Jpeg),
					_ => None,
				};

				if mime.is_none() {
					description = Some(Cow::from(text));

					continue;
				}

				mime_type = mime;
				break;
			}

			if let Some(mime_type) = mime_type {
				let pos = cursor.position() as usize;
				let data = Cow::from(cursor.into_inner()[pos..].to_vec());

				return Ok(Picture {
					pic_type,
					mime_type,
					description,
					data,
				});
			}
		}

		Err(Error::InvalidData)
	}
}
