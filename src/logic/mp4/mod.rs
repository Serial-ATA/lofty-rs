mod atom_info;
#[cfg(feature = "mp4_atoms")]
pub(crate) mod ilst;
mod moov;
mod properties;
mod read;
mod trak;

use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::{FileProperties, Result, TagType};
#[cfg(feature = "mp4_atoms")]
use ilst::Ilst;

use std::io::{Read, Seek};
use std::time::Duration;

#[allow(missing_docs)]
/// An MP4 file's audio codec
pub enum Mp4Codec {
	AAC,
	ALAC,
	Unknown(String),
}

/// An MP4 file's audio properties
pub struct Mp4Properties {
	codec: Mp4Codec,
	duration: Duration,
	bitrate: u32,
	sample_rate: u32,
	channels: u8,
}

impl From<Mp4Properties> for FileProperties {
	fn from(input: Mp4Properties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl Mp4Properties {
	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Audio codec
	pub fn codec(&self) -> &Mp4Codec {
		&self.codec
	}
}

/// An MP4 file
pub struct Mp4File {
	/// The file format from ftyp's "major brand" (Ex. "M4A ")
	pub(crate) ftyp: String,
	#[cfg(feature = "mp4_atoms")]
	/// The [`Tag`] parsed from the ilst atom, not guaranteed
	pub(crate) ilst: Option<Ilst>,
	/// The file's audio properties
	pub(crate) properties: Mp4Properties,
}

impl From<Mp4File> for TaggedFile {
	fn from(input: Mp4File) -> Self {
		Self {
			ty: FileType::MP4,
			properties: FileProperties::from(input.properties),
			tags: if let Some(ilst) = input.ilst {
				vec![ilst.into()]
			} else {
				Vec::new()
			},
		}
	}
}

impl AudioFile for Mp4File {
	type Properties = Mp4Properties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		self::read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.ilst.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Mp4Atom => self.ilst.is_some(),
			_ => false,
		}
	}
}

impl Mp4File {
	/// Returns the file format from ftyp's "major brand" (Ex. "M4A ")
	pub fn ftyp(&self) -> &str {
		self.ftyp.as_ref()
	}
}

tag_methods! {
	Mp4File => ilst, ilst, Ilst
}
