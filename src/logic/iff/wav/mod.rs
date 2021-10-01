mod read;
mod tag;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek};
use std::time::Duration;

#[allow(missing_docs, non_camel_case_types)]
/// A WAV file's format
pub enum WavFormat {
	PCM,
	IEEE_FLOAT,
	Other(u16),
}

/// A WAV file's audio properties
pub struct WavProperties {
	format: WavFormat,
	duration: Duration,
	bitrate: u32,
	sample_rate: u32,
	channels: u8,
}

impl From<WavProperties> for FileProperties {
	fn from(input: WavProperties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl WavProperties {
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

	/// WAV format
	pub fn format(&self) -> &WavFormat {
		&self.format
	}
}

/// A WAV file
pub struct WavFile {
	#[cfg(feature = "riff_info_list")]
	/// A RIFF INFO LIST
	pub(crate) riff_info: Option<Tag>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2: Option<Tag>,
	/// The file's audio properties
	pub(crate) properties: WavProperties,
}

impl From<WavFile> for TaggedFile {
	fn from(input: WavFile) -> Self {
		Self {
			ty: FileType::WAV,
			properties: FileProperties::from(input.properties),
			tags: vec![input.riff_info, input.id3v2]
				.into_iter()
				.flatten()
				.collect(),
		}
	}
}

impl AudioFile for WavFile {
	type Properties = WavProperties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.id3v2.is_some() || self.riff_info.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Id3v2 => self.id3v2.is_some(),
			TagType::RiffInfo => self.riff_info.is_some(),
			_ => false,
		}
	}
}

impl WavFile {
	#[cfg(feature = "id3v2")]
	/// Returns a reference to the ID3v2 tag if it exists
	pub fn id3v2_tag(&self) -> Option<&Tag> {
		self.id3v2.as_ref()
	}

	#[cfg(feature = "id3v2")]
	/// Returns a mutable reference to the ID3v2 tag if it exists
	pub fn id3v2_tag_mut(&mut self) -> Option<&mut Tag> {
		self.id3v2.as_mut()
	}

	#[cfg(feature = "riff_info_list")]
	/// Returns a reference to the RIFF INFO tag if it exists
	pub fn riff_info(&self) -> Option<&Tag> {
		self.riff_info.as_ref()
	}

	#[cfg(feature = "riff_info_list")]
	/// Returns a mutable reference to the RIFF INFO tag if it exists
	pub fn riff_info_mut(&mut self) -> Option<&mut Tag> {
		self.riff_info.as_mut()
	}
}
