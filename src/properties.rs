use std::time::Duration;

#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
/// Various *immutable* audio properties
pub struct FileProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: Option<u32>,
	pub(crate) audio_bitrate: Option<u32>,
	pub(crate) sample_rate: Option<u32>,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) channels: Option<u8>,
}

impl Default for FileProperties {
	fn default() -> Self {
		Self {
			duration: Duration::ZERO,
			overall_bitrate: None,
			audio_bitrate: None,
			sample_rate: None,
			bit_depth: None,
			channels: None,
		}
	}
}

impl FileProperties {
	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> Option<u32> {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn audio_bitrate(&self) -> Option<u32> {
		self.audio_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> Option<u32> {
		self.sample_rate
	}

	/// Bits per sample (usually 16 or 24 bit)
	pub fn bit_depth(&self) -> Option<u8> {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> Option<u8> {
		self.channels
	}
}

#[cfg(test)]
mod tests {
	use crate::ape::{ApeFile, ApeProperties};
	use crate::flac::FlacFile;
	use crate::iff::{AiffFile, WavFile, WavFormat, WavProperties};
	use crate::mp3::{ChannelMode, Emphasis, Layer, Mp3File, Mp3Properties, MpegVersion};
	use crate::mp4::{AudioObjectType, Mp4Codec, Mp4File, Mp4Properties};
	use crate::ogg::{
		OpusFile, OpusProperties, SpeexFile, SpeexProperties, VorbisFile, VorbisProperties,
	};
	use crate::wavpack::{WavPackFile, WavPackProperties};
	use crate::{AudioFile, FileProperties};

	use std::fs::File;
	use std::time::Duration;

	// These values are taken from FFmpeg's ffprobe
	// They may be *slightly* different due to how ffprobe rounds

	const AIFF_PROPERTIES: FileProperties = FileProperties {
		duration: Duration::from_millis(1428),
		overall_bitrate: Some(1542),
		audio_bitrate: Some(1536),
		sample_rate: Some(48000),
		bit_depth: Some(16),
		channels: Some(2),
	};

	const APE_PROPERTIES: ApeProperties = ApeProperties {
		version: 3990,
		duration: Duration::from_millis(1428),
		overall_bitrate: 361,
		audio_bitrate: 361,
		sample_rate: 48000,
		bit_depth: 16,
		channels: 2,
	};

	const FLAC_PROPERTIES: FileProperties = FileProperties {
		duration: Duration::from_millis(1428),
		overall_bitrate: Some(321),
		audio_bitrate: Some(275),
		sample_rate: Some(48000),
		bit_depth: Some(16),
		channels: Some(2),
	};

	const MP3_PROPERTIES: Mp3Properties = Mp3Properties {
		version: MpegVersion::V1,
		layer: Layer::Layer3,
		channel_mode: ChannelMode::Stereo,
		mode_extension: None,
		copyright: false,
		original: false,
		duration: Duration::from_millis(1464),
		overall_bitrate: 64,
		audio_bitrate: 62,
		sample_rate: 48000,
		channels: 2,
		emphasis: Emphasis::None,
	};

	const MP4_AAC_PROPERTIES: Mp4Properties = Mp4Properties {
		codec: Mp4Codec::AAC,
		extended_audio_object_type: Some(AudioObjectType::AacLowComplexity),
		duration: Duration::from_millis(1449),
		overall_bitrate: 135,
		audio_bitrate: 124,
		sample_rate: 48000,
		bit_depth: None,
		channels: 2,
	};

	const MP4_ALAC_PROPERTIES: Mp4Properties = Mp4Properties {
		codec: Mp4Codec::ALAC,
		extended_audio_object_type: None,
		duration: Duration::from_millis(1428),
		overall_bitrate: 331,
		audio_bitrate: 1536,
		sample_rate: 48000,
		bit_depth: Some(16),
		channels: 2,
	};

	const MP4_ALS_PROPERTIES: Mp4Properties = Mp4Properties {
		codec: Mp4Codec::AAC,
		extended_audio_object_type: Some(AudioObjectType::AudioLosslessCoding),
		duration: Duration::from_millis(1429),
		overall_bitrate: 1083,
		audio_bitrate: 1078,
		sample_rate: 48000,
		bit_depth: None,
		channels: 2,
	};

	const OPUS_PROPERTIES: OpusProperties = OpusProperties {
		duration: Duration::from_millis(1428),
		overall_bitrate: 121,
		audio_bitrate: 120,
		channels: 2,
		version: 1,
		input_sample_rate: 48000,
	};

	const SPEEX_PROPERTIES: SpeexProperties = SpeexProperties {
		duration: Duration::from_millis(1469),
		version: 1,
		sample_rate: 32000,
		mode: 2,
		channels: 2,
		vbr: false,
		overall_bitrate: 33,
		audio_bitrate: 29,
		nominal_bitrate: 29600,
	};

	const VORBIS_PROPERTIES: VorbisProperties = VorbisProperties {
		duration: Duration::from_millis(1450),
		overall_bitrate: 96,
		audio_bitrate: 112,
		sample_rate: 48000,
		channels: 2,
		version: 0,
		bitrate_maximum: 0,
		bitrate_nominal: 112_000,
		bitrate_minimum: 0,
	};

	const WAV_PROPERTIES: WavProperties = WavProperties {
		format: WavFormat::PCM,
		duration: Duration::from_millis(1428),
		overall_bitrate: 1542,
		audio_bitrate: 1536,
		sample_rate: 48000,
		bit_depth: 16,
		channels: 2,
	};

	const WAVPACK_PROPERTIES: WavPackProperties = WavPackProperties {
		version: 1040,
		duration: Duration::from_millis(1428),
		overall_bitrate: 599,
		audio_bitrate: 598,
		sample_rate: 48000,
		channels: 2,
		bit_depth: 16,
		lossless: true,
	};

	fn get_properties<T>(path: &str) -> T::Properties
	where
		T: AudioFile,
		<T as AudioFile>::Properties: Clone,
	{
		let mut f = File::open(path).unwrap();

		let audio_file = T::read_from(&mut f, true).unwrap();

		audio_file.properties().clone()
	}

	#[test]
	fn aiff_properties() {
		assert_eq!(
			get_properties::<AiffFile>("tests/files/assets/minimal/full_test.aiff"),
			AIFF_PROPERTIES
		);
	}

	#[test]
	fn ape_properties() {
		assert_eq!(
			get_properties::<ApeFile>("tests/files/assets/minimal/full_test.ape"),
			APE_PROPERTIES
		);
	}

	#[test]
	fn flac_properties() {
		assert_eq!(
			get_properties::<FlacFile>("tests/files/assets/minimal/full_test.flac"),
			FLAC_PROPERTIES
		)
	}

	#[test]
	fn mp3_properties() {
		assert_eq!(
			get_properties::<Mp3File>("tests/files/assets/minimal/full_test.mp3"),
			MP3_PROPERTIES
		)
	}

	#[test]
	fn mp4_aac_properties() {
		assert_eq!(
			get_properties::<Mp4File>("tests/files/assets/minimal/m4a_codec_aac.m4a"),
			MP4_AAC_PROPERTIES
		)
	}

	#[test]
	fn mp4_alac_properties() {
		assert_eq!(
			get_properties::<Mp4File>("tests/files/assets/minimal/m4a_codec_alac.m4a"),
			MP4_ALAC_PROPERTIES
		)
	}

	#[test]
	fn mp4_als_properties() {
		assert_eq!(
			get_properties::<Mp4File>("tests/files/assets/minimal/mp4_codec_als.mp4"),
			MP4_ALS_PROPERTIES
		)
	}

	#[test]
	fn opus_properties() {
		assert_eq!(
			get_properties::<OpusFile>("tests/files/assets/minimal/full_test.opus"),
			OPUS_PROPERTIES
		)
	}

	#[test]
	fn speex_properties() {
		assert_eq!(
			get_properties::<SpeexFile>("tests/files/assets/minimal/full_test.spx"),
			SPEEX_PROPERTIES
		)
	}

	#[test]
	fn vorbis_properties() {
		assert_eq!(
			get_properties::<VorbisFile>("tests/files/assets/minimal/full_test.ogg"),
			VORBIS_PROPERTIES
		)
	}

	#[test]
	fn wav_properties() {
		assert_eq!(
			get_properties::<WavFile>("tests/files/assets/minimal/wav_format_pcm.wav"),
			WAV_PROPERTIES
		)
	}

	#[test]
	fn wavpack_properties() {
		assert_eq!(
			get_properties::<WavPackFile>("tests/files/assets/minimal/full_test.wv"),
			WAVPACK_PROPERTIES
		)
	}
}
