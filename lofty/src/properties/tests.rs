use crate::aac::{AACProperties, AacFile};
use crate::ape::{ApeFile, ApeProperties};
use crate::config::ParseOptions;
use crate::file::AudioFile;
use crate::flac::{FlacFile, FlacProperties};
use crate::iff::aiff::{AiffFile, AiffProperties};
use crate::iff::wav::{WavFile, WavFormat, WavProperties};
use crate::mp4::{AudioObjectType, Mp4Codec, Mp4File, Mp4Properties};
use crate::mpeg::{ChannelMode, Layer, MpegFile, MpegProperties, MpegVersion};
use crate::musepack::sv4to6::MpcSv4to6Properties;
use crate::musepack::sv7::{Link, MpcSv7Properties, Profile};
use crate::musepack::sv8::{EncoderInfo, MpcSv8Properties, ReplayGain, StreamHeader};
use crate::musepack::{MpcFile, MpcProperties};
use crate::ogg::{
	OpusFile, OpusProperties, SpeexFile, SpeexProperties, VorbisFile, VorbisProperties,
};
use crate::properties::ChannelMask;
use crate::wavpack::{WavPackFile, WavPackProperties};

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
	channel_mask: Some(ChannelMask::stereo()),
	copyright: false,
	original: false,
};

const AIFF_PROPERTIES: AiffProperties = AiffProperties {
	duration: Duration::from_millis(1428),
	overall_bitrate: 1542,
	audio_bitrate: 1536,
	sample_rate: 48000,
	sample_size: 16,
	channels: 2,
	compression_type: None,
};

const APE_PROPERTIES: ApeProperties = ApeProperties {
	version: 3990,
	duration: Duration::from_millis(1428),
	overall_bitrate: 361,
	audio_bitrate: 360,
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

const MP1_PROPERTIES: MpegProperties = MpegProperties {
	version: MpegVersion::V1,
	layer: Layer::Layer1,
	channel_mode: ChannelMode::Stereo,
	mode_extension: None,
	copyright: false,
	original: true,
	duration: Duration::from_millis(588), // FFmpeg reports 576, possibly an issue
	overall_bitrate: 384,                 // TODO: FFmpeg reports 392
	audio_bitrate: 384,
	sample_rate: 32000,
	channels: 2,
	emphasis: None,
};

const MP2_PROPERTIES: MpegProperties = MpegProperties {
	version: MpegVersion::V1,
	layer: Layer::Layer2,
	channel_mode: ChannelMode::Stereo,
	mode_extension: None,
	copyright: false,
	original: true,
	duration: Duration::from_millis(1440),
	overall_bitrate: 384,
	audio_bitrate: 384,
	sample_rate: 48000,
	channels: 2,
	emphasis: None,
};

const MP3_PROPERTIES: MpegProperties = MpegProperties {
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
	emphasis: None,
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
	drm_protected: false,
};

const MP4_ALAC_PROPERTIES: Mp4Properties = Mp4Properties {
	codec: Mp4Codec::ALAC,
	extended_audio_object_type: None,
	duration: Duration::from_millis(1428),
	overall_bitrate: 331,
	audio_bitrate: 326,
	sample_rate: 48000,
	bit_depth: Some(16),
	channels: 2,
	drm_protected: false,
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
	drm_protected: false,
};

const MP4_FLAC_PROPERTIES: Mp4Properties = Mp4Properties {
	codec: Mp4Codec::FLAC,
	extended_audio_object_type: None,
	duration: Duration::from_millis(1428),
	overall_bitrate: 280,
	audio_bitrate: 275,
	sample_rate: 48000,
	bit_depth: Some(16),
	channels: 2,
	drm_protected: false,
};

// Properties verified with libmpcdec 1.2.2
const MPC_SV5_PROPERTIES: MpcSv4to6Properties = MpcSv4to6Properties {
	duration: Duration::from_millis(26347),
	average_bitrate: 119,
	channels: 2,
	frame_count: 1009,
	mid_side_stereo: true,
	stream_version: 5,
	max_band: 31,
	sample_rate: 44100,
};

const MPC_SV7_PROPERTIES: MpcSv7Properties = MpcSv7Properties {
	duration: Duration::from_millis(1440),
	average_bitrate: 86,
	channels: 2,
	frame_count: 60,
	intensity_stereo: false,
	mid_side_stereo: true,
	max_band: 26,
	profile: Profile::Standard,
	link: Link::VeryLowStartOrEnd,
	sample_freq: 48000,
	max_level: 0,
	title_gain: 0,
	title_peak: 0,
	album_gain: 0,
	album_peak: 0,
	true_gapless: true,
	last_frame_length: 578,
	fast_seeking_safe: false,
	encoder_version: 192,
};

const MPC_SV8_PROPERTIES: MpcSv8Properties = MpcSv8Properties {
	duration: Duration::from_millis(1428),
	average_bitrate: 82,
	stream_header: StreamHeader {
		crc: 4_252_559_415,
		stream_version: 8,
		sample_count: 68546,
		beginning_silence: 0,
		sample_rate: 48000,
		max_used_bands: 26,
		channels: 2,
		ms_used: true,
		audio_block_frames: 64,
	},
	replay_gain: ReplayGain {
		version: 1,
		title_gain: 16655,
		title_peak: 21475,
		album_gain: 16655,
		album_peak: 21475,
	},
	encoder_info: Some(EncoderInfo {
		profile: 10.0,
		pns_tool: false,
		major: 1,
		minor: 30,
		build: 1,
	}),
};

const OPUS_PROPERTIES: OpusProperties = OpusProperties {
	duration: Duration::from_millis(1428),
	overall_bitrate: 120,
	audio_bitrate: 120,
	channels: 2,
	channel_mask: ChannelMask::stereo(),
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
	duration: Duration::from_millis(1451),
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
	channel_mask: ChannelMask::stereo(),
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

#[test_log::test]
fn aac_properties() {
	assert_eq!(
		get_properties::<AacFile>("tests/files/assets/minimal/full_test.aac"),
		AAC_PROPERTIES
	);
}

#[test_log::test]
fn aiff_properties() {
	assert_eq!(
		get_properties::<AiffFile>("tests/files/assets/minimal/full_test.aiff"),
		AIFF_PROPERTIES
	);
}

#[test_log::test]
fn ape_properties() {
	assert_eq!(
		get_properties::<ApeFile>("tests/files/assets/minimal/full_test.ape"),
		APE_PROPERTIES
	);
}

#[test_log::test]
fn flac_properties() {
	assert_eq!(
		get_properties::<FlacFile>("tests/files/assets/minimal/full_test.flac"),
		FLAC_PROPERTIES
	)
}

#[test_log::test]
fn mp1_properties() {
	assert_eq!(
		get_properties::<MpegFile>("tests/files/assets/minimal/full_test.mp1"),
		MP1_PROPERTIES
	)
}

#[test_log::test]
fn mp2_properties() {
	assert_eq!(
		get_properties::<MpegFile>("tests/files/assets/minimal/full_test.mp2"),
		MP2_PROPERTIES
	)
}

#[test_log::test]
fn mp3_properties() {
	assert_eq!(
		get_properties::<MpegFile>("tests/files/assets/minimal/full_test.mp3"),
		MP3_PROPERTIES
	)
}

#[test_log::test]
fn mp4_aac_properties() {
	assert_eq!(
		get_properties::<Mp4File>("tests/files/assets/minimal/m4a_codec_aac.m4a"),
		MP4_AAC_PROPERTIES
	)
}

#[test_log::test]
fn mp4_alac_properties() {
	assert_eq!(
		get_properties::<Mp4File>("tests/files/assets/minimal/m4a_codec_alac.m4a"),
		MP4_ALAC_PROPERTIES
	)
}

#[test_log::test]
fn mp4_als_properties() {
	assert_eq!(
		get_properties::<Mp4File>("tests/files/assets/minimal/mp4_codec_als.mp4"),
		MP4_ALS_PROPERTIES
	)
}

#[test_log::test]
fn mp4_flac_properties() {
	assert_eq!(
		get_properties::<Mp4File>("tests/files/assets/minimal/mp4_codec_flac.mp4"),
		MP4_FLAC_PROPERTIES
	)
}

#[test_log::test]
fn mpc_sv5_properties() {
	assert_eq!(
		get_properties::<MpcFile>("tests/files/assets/minimal/mpc_sv5.mpc"),
		MpcProperties::Sv4to6(MPC_SV5_PROPERTIES)
	)
}

#[test_log::test]
fn mpc_sv7_properties() {
	assert_eq!(
		get_properties::<MpcFile>("tests/files/assets/minimal/mpc_sv7.mpc"),
		MpcProperties::Sv7(MPC_SV7_PROPERTIES)
	)
}

#[test_log::test]
fn mpc_sv8_properties() {
	assert_eq!(
		get_properties::<MpcFile>("tests/files/assets/minimal/mpc_sv8.mpc"),
		MpcProperties::Sv8(MPC_SV8_PROPERTIES)
	)
}

#[test_log::test]
fn opus_properties() {
	assert_eq!(
		get_properties::<OpusFile>("tests/files/assets/minimal/full_test.opus"),
		OPUS_PROPERTIES
	)
}

#[test_log::test]
fn speex_properties() {
	assert_eq!(
		get_properties::<SpeexFile>("tests/files/assets/minimal/full_test.spx"),
		SPEEX_PROPERTIES
	)
}

#[test_log::test]
fn vorbis_properties() {
	assert_eq!(
		get_properties::<VorbisFile>("tests/files/assets/minimal/full_test.ogg"),
		VORBIS_PROPERTIES
	)
}

#[test_log::test]
fn wav_properties() {
	assert_eq!(
		get_properties::<WavFile>("tests/files/assets/minimal/wav_format_pcm.wav"),
		WAV_PROPERTIES
	)
}

#[test_log::test]
fn wavpack_properties() {
	assert_eq!(
		get_properties::<WavPackFile>("tests/files/assets/minimal/full_test.wv"),
		WAVPACK_PROPERTIES
	)
}
