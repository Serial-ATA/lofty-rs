use super::Language;
use crate::properties::FileProperties;

use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

/// The supported EBML document types
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DocumentType {
	/// Matroska (`audio/x-matroska` / `video/x-matroska`)
	Matroska,
	/// WebM (`audio/webm` / `video/webm`)
	Webm,
}

impl FromStr for DocumentType {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"matroska" => Ok(DocumentType::Matroska),
			"webm" => Ok(DocumentType::Webm),
			_ => Err(()),
		}
	}
}

impl Display for DocumentType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			DocumentType::Matroska => write!(f, "matroska"),
			DocumentType::Webm => write!(f, "webm"),
		}
	}
}

/// Properties from the EBML header
///
/// These are present for all EBML formats.
#[derive(Debug, Clone, PartialEq)]
pub struct EbmlHeaderProperties {
	pub(crate) version: u64,
	pub(crate) read_version: u64,
	pub(crate) max_id_length: u8,
	pub(crate) max_size_length: u8,
	pub(crate) doc_type: DocumentType,
	pub(crate) doc_type_version: u64,
	pub(crate) doc_type_read_version: u64,
}

impl Default for EbmlHeaderProperties {
	fn default() -> Self {
		Self {
			version: 0,
			read_version: 0,
			max_id_length: 0,
			max_size_length: 0,
			doc_type: DocumentType::Matroska,
			doc_type_version: 0,
			doc_type_read_version: 0,
		}
	}
}

impl EbmlHeaderProperties {
	/// The EBML version, should be `1`
	pub fn version(&self) -> u64 {
		self.version
	}

	/// The minimum EBML version required to read the file, <= [`Self::version()`]
	pub fn read_version(&self) -> u64 {
		self.read_version
	}

	/// The maximum length of an EBML element ID, in octets
	pub fn max_id_length(&self) -> u8 {
		self.max_id_length
	}

	/// The maximum length of an EBML element size, in octets
	pub fn max_size_length(&self) -> u8 {
		self.max_size_length
	}

	/// The type of document
	pub fn doc_type(&self) -> DocumentType {
		self.doc_type
	}

	/// The version of DocType interpreter used to create the EBML Document
	pub fn doc_type_version(&self) -> u64 {
		self.doc_type_version
	}

	/// The minimum DocType interpreter version needed to read the EBML Document
	pub fn doc_type_read_version(&self) -> u64 {
		self.doc_type_read_version
	}
}

/// An EBML DocType extension
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlExtension {
	pub(crate) name: String,
	pub(crate) version: u64,
}

impl EbmlExtension {
	/// The name of the extension
	pub fn name(&self) -> &str {
		&self.name
	}

	/// The version of the extension
	pub fn version(&self) -> u64 {
		self.version
	}
}

/// Information about a segment
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentInfo {
	pub(crate) timestamp_scale: u64,
	pub(crate) muxing_app: String,
	pub(crate) writing_app: String,
	pub(crate) duration: Option<Duration>,
}

impl SegmentInfo {
	/// Base unit for Segment Ticks and Track Ticks, in nanoseconds.
	///
	/// A TimestampScale value of 1000000 means scaled timestamps in the Segment are expressed in milliseconds.
	pub fn timestamp_scale(&self) -> u64 {
		self.timestamp_scale
	}

	/// Muxing application or library (example: "libmatroska-0.4.3").
	///
	/// Includes the full name of the application or library followed by the version number.
	pub fn muxing_app(&self) -> &str {
		&self.muxing_app
	}

	/// Writing application (example: "mkvmerge-0.3.3").
	///
	/// Includes the full name of the application followed by the version number.
	pub fn writing_app(&self) -> &str {
		&self.writing_app
	}

	/// The duration of the segment
	///
	/// NOTE: This information is not always present in the segment, in which case
	///       [`EbmlProperties::duration`] should be used.
	pub fn duration(&self) -> Option<Duration> {
		self.duration
	}
}

impl Default for SegmentInfo {
	fn default() -> Self {
		Self {
			// https://matroska.org/technical/elements.html
			timestamp_scale: 1_000_000,
			muxing_app: String::new(),
			writing_app: String::new(),
			duration: None,
		}
	}
}

/// A full descriptor for an audio track
#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrackDescriptor {
	pub(crate) number: u64,
	pub(crate) uid: u64,
	pub(crate) enabled: bool,
	pub(crate) default: bool,
	pub(crate) language: Language,
	pub(crate) default_duration: u64,
	pub(crate) codec_id: String,
	pub(crate) codec_private: Option<Vec<u8>>,
	pub(crate) codec_name: Option<String>,
	pub(crate) settings: AudioTrackSettings,
}

impl Default for AudioTrackDescriptor {
	fn default() -> Self {
		AudioTrackDescriptor {
			// Note, these values are not spec compliant and will hopefully be overwritten when
			// parsing. It doesn't really matter though, since we aren't an encoder.
			number: 0,
			uid: 0,
			default_duration: 0,
			codec_id: String::new(),

			// Spec-compliant defaults
			enabled: true,
			default: true,
			language: Language::Iso639_2(String::from("eng")),
			codec_private: None,
			codec_name: None,
			settings: AudioTrackSettings::default(),
		}
	}
}

impl AudioTrackDescriptor {
	/// The track number
	pub fn number(&self) -> u64 {
		self.number
	}

	/// A unique ID to identify the track
	pub fn uid(&self) -> u64 {
		self.uid
	}

	/// Whether the track is usable
	pub fn is_enabled(&self) -> bool {
		self.enabled
	}

	/// Whether the track is eligible for automatic selection
	pub fn is_default(&self) -> bool {
		self.default
	}

	/// The language of the track, in the Matroska languages form
	///
	/// NOTE: See [basics](https://matroska.org/technical/basics.html#language-codes) on language codes.
	pub fn language(&self) -> &Language {
		&self.language
	}

	/// The default duration of the track
	pub fn default_duration(&self) -> u64 {
		self.default_duration
	}

	/// The codec ID of the track
	///
	/// NOTE: See [Matroska codec RFC] for more info.
	///
	/// [Matroska codec RFC]: https://matroska.org/technical/codec_specs.html
	pub fn codec_id(&self) -> &str {
		&self.codec_id
	}

	/// Private data only known to the codec
	pub fn codec_private(&self) -> Option<&[u8]> {
		self.codec_private.as_deref()
	}

	/// A human-readable string for the [codec_id](AudioTrackDescriptor::codec_id)
	pub fn codec_name(&self) -> Option<&str> {
		self.codec_name.as_deref()
	}

	/// The audio settings of the track
	pub fn settings(&self) -> &AudioTrackSettings {
		&self.settings
	}
}

/// Settings for an audio track
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioTrackSettings {
	// Provided to us for free
	pub(crate) sampling_frequency: f64,
	pub(crate) output_sampling_frequency: f64,
	pub(crate) channels: u8,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) emphasis: Option<EbmlAudioTrackEmphasis>,

	// Need to be calculated
	pub(crate) bitrate: Option<u32>,
}

impl AudioTrackSettings {
	/// The sampling frequency of the track
	pub fn sampling_frequency(&self) -> f64 {
		self.sampling_frequency
	}

	/// Real output sampling frequency in Hz (used for SBR techniques).
	///
	/// The default value for `output_sampling_frequency` of the same TrackEntry is equal to the [`Self::sampling_frequency`].
	pub fn output_sampling_frequency(&self) -> f64 {
		self.output_sampling_frequency
	}

	/// The number of channels in the track
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// The bit depth of the track
	pub fn bit_depth(&self) -> Option<u8> {
		self.bit_depth
	}

	/// Audio emphasis applied on audio samples
	pub fn emphasis(&self) -> Option<EbmlAudioTrackEmphasis> {
		self.emphasis
	}
}

/// A rarely-used decoder hint that the file must be de-emphasized
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EbmlAudioTrackEmphasis {
	CdAudio = 1,
	Reserved = 2,
	CcitJ17 = 3,
	Fm50 = 4,
	Fm75 = 5,
	PhonoRiaa = 10,
	PhonoIecN78 = 11,
	PhonoTeldec = 12,
	PhonoEmi = 13,
	PhonoColumbiaLp = 14,
	PhonoLondon = 15,
	PhonoNartb = 16,
}

impl EbmlAudioTrackEmphasis {
	/// Get the audio emphasis from a `u8`
	pub fn from_u8(value: u8) -> Option<Self> {
		match value {
			1 => Some(Self::CdAudio),
			2 => Some(Self::Reserved),
			3 => Some(Self::CcitJ17),
			4 => Some(Self::Fm50),
			5 => Some(Self::Fm75),
			10 => Some(Self::PhonoRiaa),
			11 => Some(Self::PhonoIecN78),
			12 => Some(Self::PhonoTeldec),
			13 => Some(Self::PhonoEmi),
			14 => Some(Self::PhonoColumbiaLp),
			15 => Some(Self::PhonoLondon),
			16 => Some(Self::PhonoNartb),
			_ => None,
		}
	}
}

/// EBML audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlProperties {
	pub(crate) header: EbmlHeaderProperties,
	pub(crate) extensions: Vec<EbmlExtension>,
	pub(crate) segment_info: SegmentInfo,
	pub(crate) audio_tracks: Vec<AudioTrackDescriptor>,
}

impl EbmlProperties {
	/// The EBML header properties
	///
	/// This includes the properties that are part of the EBML base specification.
	/// All Matroska-specific properties are in [`Self::segment_info`] and [`Self::default_audio_track`].
	pub fn header(&self) -> &EbmlHeaderProperties {
		&self.header
	}

	/// The DocType extensions
	pub fn extensions(&self) -> &[EbmlExtension] {
		&self.extensions
	}

	/// Information from the `\Segment\Info` element
	pub fn segment_info(&self) -> &SegmentInfo {
		&self.segment_info
	}

	/// All audio tracks in the file
	///
	/// This includes all audio tracks in the Matroska `\Segment\Tracks` element.
	pub fn audio_tracks(&self) -> &[AudioTrackDescriptor] {
		&self.audio_tracks
	}

	/// Information about the default audio track
	///
	/// The "default" track is selected as:
	/// 1. The first audio track with its `default` flag set
	/// 2. If 1 fails, just grab the first audio track with its `enabled` flag set
	pub fn default_audio_track(&self) -> Option<&AudioTrackDescriptor> {
		if let Some(position) = self.default_audio_track_position() {
			return self.audio_tracks.get(position);
		}

		None
	}

	// TODO: Actually calculate from cluster
	/// The duration of the default audio track
	///
	/// NOTE: see [`EbmlProperties::default_audio_track`]
	///
	/// This will always use the duration written in `\Segment\Info` if present. Otherwise, it will
	/// be manually calculated using `\Segment\Cluster` data.
	pub fn duration(&self) -> Duration {
		self.segment_info.duration().unwrap()
	}

	/// Audio bitrate (kbps)
	///
	/// NOTE: This is the bitrate of the default audio track see [`EbmlProperties::default_audio_track`]
	///       for what this means.
	pub fn bitrate(&self) -> Option<u32> {
		self.default_audio_track()
			.and_then(|track| track.settings.bitrate)
	}

	pub(crate) fn default_audio_track_position(&self) -> Option<usize> {
		self.audio_tracks
			.iter()
			.position(|track| track.default)
			.or_else(|| {
				// Otherwise, it's normal to just pick the first enabled track
				self.audio_tracks.iter().position(|track| track.enabled)
			})
	}
}

impl From<EbmlProperties> for FileProperties {
	fn from(input: EbmlProperties) -> Self {
		let Some(default_audio_track) = input.default_audio_track() else {
			let mut properties = FileProperties::default();
			if let Some(duration) = input.segment_info.duration {
				properties.duration = duration;
			}

			return properties;
		};

		Self {
			duration: input.duration(),
			overall_bitrate: input.bitrate(),
			audio_bitrate: input.bitrate(),
			sample_rate: Some(default_audio_track.settings.sampling_frequency as u32),
			bit_depth: default_audio_track.settings.bit_depth,
			channels: Some(default_audio_track.settings.channels),
			channel_mask: None, // TODO: Will require reading into track data
		}
	}
}
