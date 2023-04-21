use std::time::Duration;

/// Various *immutable* audio properties
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct FileProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: Option<u32>,
	pub(crate) audio_bitrate: Option<u32>,
	pub(crate) sample_rate: Option<u32>,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) channels: Option<u8>,
	pub(crate) channel_mask: Option<ChannelMask>,
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
			channel_mask: None,
		}
	}
}

impl FileProperties {
	/// Create a new `FileProperties`
	#[must_use]
	pub const fn new(
		duration: Duration,
		overall_bitrate: Option<u32>,
		audio_bitrate: Option<u32>,
		sample_rate: Option<u32>,
		bit_depth: Option<u8>,
		channels: Option<u8>,
		channel_mask: Option<ChannelMask>,
	) -> Self {
		Self {
			duration,
			overall_bitrate,
			audio_bitrate,
			sample_rate,
			bit_depth,
			channels,
			channel_mask,
		}
	}

	/// Duration of the audio
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

	/// Channel mask
	pub fn channel_mask(&self) -> Option<ChannelMask> {
		self.channel_mask
	}
}

/// Channel mask
///
/// A mask of (at least) 18 bits, one for each channel.
///
/// * Standard speaker channels: <https://www.wikipedia.org/wiki/Surround_sound>
/// * CAF channel bitmap: <https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_spec/CAF_spec.html#//apple_ref/doc/uid/TP40001862-CH210-BCGBHHHI>
/// * WAV default channel ordering: <https://learn.microsoft.com/en-us/previous-versions/windows/hardware/design/dn653308(v=vs.85)?redirectedfrom=MSDN#default-channel-ordering>
/// * FFmpeg: <https://ffmpeg.org/doxygen/trunk/group__channel__masks.html>
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct ChannelMask(pub(crate) u32);

impl ChannelMask {
	/// A single front center channel
	#[must_use]
	pub const fn mono() -> Self {
		Self(0x4) // front center
	}

	/// Front left+right channels
	#[must_use]
	pub const fn stereo() -> Self {
		Self(0x3) // front left (0x1) + front right (0x2)
	}

	/// The bit mask
	#[must_use]
	pub const fn bits(self) -> u32 {
		self.0
	}
}

#[cfg(test)]
mod tests {
	use crate::aac::{AACProperties, AacFile};
	use crate::ape::{ApeFile, ApeProperties};
	use crate::flac::{FlacFile, FlacProperties};
	use crate::iff::aiff::AiffFile;
	use crate::iff::wav::{WavFile, WavFormat, WavProperties};
	use crate::mp4::{AudioObjectType, Mp4Codec, Mp4File, Mp4Properties};
	use crate::mpeg::{ChannelMode, Emphasis, Layer, MPEGFile, MPEGProperties, MpegVersion};
	use crate::ogg::{
		OpusFile, OpusProperties, SpeexFile, SpeexProperties, VorbisFile, VorbisProperties,
	};
	use crate::probe::ParseOptions;
	use crate::wavpack::{WavPackFile, WavPackProperties};
	use crate::{AudioFile, FileProperties};

	use std::fs::File;
	use std::time::Duration;

	// These values are taken from FFmpeg's ffprobe
	// There is a chance they will be +/- 1, anything greater (for real world files)
	// is an issue.

	const AAC_PROPERTIES: AACProperties = AACProperties {
		version: MpegVersion::V4,
		audio_object_type: AudioObjectType::AacLowComplexity,
		duration: Duration::from_millis(1474), /* TODO: This is ~100ms greater than FFmpeg's report, can we do better? */
		overall_bitrate: 117,                  // 9 less than FFmpeg reports
		audio_bitrate: 117,                    // 9 less than FFmpeg reports
		sample_rate: 48000,
		channels: 2,
		copyright: false,
		original: false,
	};

	const AIFF_PROPERTIES: FileProperties = FileProperties {
		duration: Duration::from_millis(1428),
		overall_bitrate: Some(1542),
		audio_bitrate: Some(1536),
		sample_rate: Some(48000),
		bit_depth: Some(16),
		channels: Some(2),
		channel_mask: None,
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

	const FLAC_PROPERTIES: FlacProperties = FlacProperties {
		duration: Duration::from_millis(1428),
		overall_bitrate: 321,
		audio_bitrate: 275,
		sample_rate: 48000,
		bit_depth: 16,
		channels: 2,
		signature: 164_506_065_180_489_231_127_156_351_872_182_799_315,
	};

	const MP1_PROPERTIES: MPEGProperties = MPEGProperties {
		version: MpegVersion::V1,
		layer: Layer::Layer1,
		channel_mode: ChannelMode::Stereo,
		mode_extension: None,
		copyright: false,
		original: true,
		duration: Duration::from_millis(588), // FFmpeg reports 576, possibly an issue
		overall_bitrate: 383,                 // TODO: FFmpeg reports 392
		audio_bitrate: 384,
		sample_rate: 32000,
		channels: 2,
		emphasis: Emphasis::None,
	};

	const MP2_PROPERTIES: MPEGProperties = MPEGProperties {
		version: MpegVersion::V1,
		layer: Layer::Layer2,
		channel_mode: ChannelMode::Stereo,
		mode_extension: None,
		copyright: false,
		original: true,
		duration: Duration::from_millis(1344), // TODO: FFmpeg reports 1440 here
		overall_bitrate: 411,                  // FFmpeg reports 384, related to above issue
		audio_bitrate: 384,
		sample_rate: 48000,
		channels: 2,
		emphasis: Emphasis::None,
	};

	const MP3_PROPERTIES: MPEGProperties = MPEGProperties {
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

	const MP4_FLAC_PROPERTIES: Mp4Properties = Mp4Properties {
		codec: Mp4Codec::FLAC,
		extended_audio_object_type: None,
		duration: Duration::from_millis(1428),
		overall_bitrate: 280, // TODO: FFmpeg reports 279
		audio_bitrate: 275,
		sample_rate: 48000,
		bit_depth: Some(16),
		channels: 2,
	};

	const OPUS_PROPERTIES: OpusProperties = OpusProperties {
		duration: Duration::from_millis(1428),
		overall_bitrate: 120,
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
		overall_bitrate: 32,
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
		channel_mask: None,
	};

	const WAVPACK_PROPERTIES: WavPackProperties = WavPackProperties {
		version: 1040,
		duration: Duration::from_millis(1428),
		overall_bitrate: 598,
		audio_bitrate: 597,
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

		let audio_file = T::read_from(&mut f, ParseOptions::default()).unwrap();

		audio_file.properties().clone()
	}

	#[test]
	fn aac_properties() {
		assert_eq!(
			get_properties::<AacFile>("tests/files/assets/minimal/full_test.aac"),
			AAC_PROPERTIES
		);
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
	fn mp1_properties() {
		assert_eq!(
			get_properties::<MPEGFile>("tests/files/assets/minimal/full_test.mp1"),
			MP1_PROPERTIES
		)
	}

	#[test]
	fn mp2_properties() {
		assert_eq!(
			get_properties::<MPEGFile>("tests/files/assets/minimal/full_test.mp2"),
			MP2_PROPERTIES
		)
	}

	#[test]
	fn mp3_properties() {
		assert_eq!(
			get_properties::<MPEGFile>("tests/files/assets/minimal/full_test.mp3"),
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
	fn mp4_flac_properties() {
		assert_eq!(
			get_properties::<Mp4File>("tests/files/assets/minimal/mp4_codec_flac.mp4"),
			MP4_FLAC_PROPERTIES
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
