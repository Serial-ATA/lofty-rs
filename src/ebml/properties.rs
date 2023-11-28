use crate::properties::FileProperties;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlHeaderProperties {
	pub(crate) version: u64,
	pub(crate) read_version: u64,
	pub(crate) max_id_length: u8,
	pub(crate) max_size_length: u8,
	pub(crate) doc_type: String,
	pub(crate) doc_type_version: u64,
	pub(crate) doc_type_read_version: u64,
}

impl EbmlHeaderProperties {
	/// The EBML version, should be `1`
	pub fn version(&self) -> u64 {
		self.version
	}

	/// The minimum EBML version required to read the file, <= [`version`]
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

	/// A string that describes the type of document
	pub fn doc_type(&self) -> &str {
		&self.doc_type
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegmentInfo {
	pub(crate) timestamp_scale: u64,
	pub(crate) muxing_app: String,
	pub(crate) writing_app: String,
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
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioTrackDescriptor {
	pub(crate) number: u64,
	pub(crate) uid: u64,
	pub(crate) language: String,
	pub(crate) default_duration: u64,
	pub(crate) codec_id: String,
	pub(crate) codec_private: Vec<u8>,
	pub(crate) settings: AudioTrackSettings,
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

	/// The language of the track, in the Matroska languages form
	///
	/// NOTE: See [basics](https://matroska.org/technical/basics.html#language-codes) on language codes.
	pub fn language(&self) -> &str {
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
	pub fn codec_private(&self) -> &[u8] {
		&self.codec_private
	}

	/// The audio settings of the track
	pub fn settings(&self) -> &AudioTrackSettings {
		&self.settings
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioTrackSettings {
	pub(crate) sampling_frequency: u32,
	pub(crate) output_sampling_frequency: u32,
	pub(crate) channels: u8,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) emphasis: Option<EbmlAudioTrackEmphasis>,
}

impl AudioTrackSettings {
	/// The sampling frequency of the track
	pub fn sampling_frequency(&self) -> u32 {
		self.sampling_frequency
	}

	/// Real output sampling frequency in Hz (used for SBR techniques).
	///
	/// The default value for `output_sampling_frequency` of the same TrackEntry is equal to the [`Self::sampling_frequency`].
	pub fn output_sampling_frequency(&self) -> u32 {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EbmlAudioTrackEmphasis {
	None = 0,
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

/// EBML audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlProperties {
	pub(crate) header: EbmlHeaderProperties,
	pub(crate) extensions: Vec<EbmlExtension>,
	pub(crate) segment_info: SegmentInfo,
	pub(crate) default_audio_track: AudioTrackDescriptor,
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

	/// Information from the Matroska `\EBML\Segment\Info` element
	pub fn segment_info(&self) -> &SegmentInfo {
		&self.segment_info
	}

	/// Information about the default audio track
	///
	/// The information is extracted from the first audio track with its default flag set
	/// in the Matroska `\EBML\Segment\Tracks` element.
	pub fn default_audio_track(&self) -> &AudioTrackDescriptor {
		&self.default_audio_track
	}
}

impl From<EbmlProperties> for FileProperties {
	fn from(input: EbmlProperties) -> Self {
		Self {
			duration: todo!("Support duration"),
			overall_bitrate: todo!("Support bitrate"),
			audio_bitrate: todo!("Support bitrate"),
			sample_rate: Some(input.default_audio_track.settings.sampling_frequency),
			bit_depth: input.default_audio_track.settings.bit_depth,
			channels: Some(input.default_audio_track.settings.channels),
			channel_mask: todo!("Channel mask"),
		}
	}
}
