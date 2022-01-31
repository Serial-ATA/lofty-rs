use lofty::ape::{ApeFile, ApeProperties};
use lofty::iff::{AiffFile, WavFile, WavFormat, WavProperties};
use lofty::mp3::{ChannelMode, Layer, Mp3File, Mp3Properties, MpegVersion};
use lofty::mp4::{Mp4Codec, Mp4File, Mp4Properties};
use lofty::ogg::{
	FlacFile, OpusFile, OpusProperties, SpeexFile, SpeexProperties, VorbisFile, VorbisProperties,
};
use lofty::{AudioFile, FileProperties};

use std::fs::File;
use std::time::Duration;

const AIFF_PROPERTIES: FileProperties = FileProperties::new(
	Duration::from_millis(1428),
	Some(1542),
	Some(1536),
	Some(48000),
	Some(16),
	Some(2),
);

const APE_PROPERTIES: ApeProperties =
	ApeProperties::new(3990, Duration::from_millis(1428), 360, 360, 48000, 16, 2);

const FLAC_PROPERTIES: FileProperties = FileProperties::new(
	Duration::from_millis(1428),
	Some(321),
	Some(275),
	Some(48000),
	Some(16),
	Some(2),
);

const MP3_PROPERTIES: Mp3Properties = Mp3Properties::new(
	MpegVersion::V1,
	Layer::Layer3,
	ChannelMode::Stereo,
	Duration::from_millis(1464),
	64,
	62,
	48000,
	2,
);

const MP4_AAC_PROPERTIES: Mp4Properties = Mp4Properties::new(
	Mp4Codec::AAC,
	Duration::from_millis(1449),
	135,
	124,
	48000,
	None,
	2,
);

const MP4_ALAC_PROPERTIES: Mp4Properties = Mp4Properties::new(
	Mp4Codec::ALAC,
	Duration::from_millis(1428),
	331,
	1536,
	48000,
	Some(16),
	2,
);

const OPUS_PROPERTIES: OpusProperties =
	OpusProperties::new(Duration::from_millis(1428), 120, 120, 2, 1, 48000);

const SPEEX_PROPERTIES: SpeexProperties = SpeexProperties::new(
	Duration::from_millis(1469),
	1,
	32000,
	2,
	2,
	false,
	32,
	29,
	29600,
);

const VORBIS_PROPERTIES: VorbisProperties = VorbisProperties::new(
	Duration::from_millis(1450),
	96,
	112,
	48000,
	2,
	0,
	0,
	112000,
	0,
);

const WAV_PROPERTIES: WavProperties = WavProperties::new(
	WavFormat::PCM,
	Duration::from_millis(1428),
	1542,
	1536,
	48000,
	16,
	2,
);

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
		get_properties::<AiffFile>("tests/files/assets/full_test.aiff"),
		AIFF_PROPERTIES
	);
}

#[test]
fn ape_properties() {
	assert_eq!(
		get_properties::<ApeFile>("tests/files/assets/full_test.ape"),
		APE_PROPERTIES
	);
}

#[test]
fn flac_properties() {
	assert_eq!(
		get_properties::<FlacFile>("tests/files/assets/full_test.flac"),
		FLAC_PROPERTIES
	)
}

#[test]
fn mp3_properties() {
	assert_eq!(
		get_properties::<Mp3File>("tests/files/assets/full_test.mp3"),
		MP3_PROPERTIES
	)
}

#[test]
fn mp4_aac_properties() {
	assert_eq!(
		get_properties::<Mp4File>("tests/files/assets/m4a_codec_aac.m4a"),
		MP4_AAC_PROPERTIES
	)
}

#[test]
fn mp4_alac_properties() {
	assert_eq!(
		get_properties::<Mp4File>("tests/files/assets/m4a_codec_alac.m4a"),
		MP4_ALAC_PROPERTIES
	)
}

#[test]
fn opus_properties() {
	assert_eq!(
		get_properties::<OpusFile>("tests/files/assets/full_test.opus"),
		OPUS_PROPERTIES
	)
}

#[test]
fn speex_properties() {
	assert_eq!(
		get_properties::<SpeexFile>("tests/files/assets/full_test.spx"),
		SPEEX_PROPERTIES
	)
}

#[test]
fn vorbis_properties() {
	assert_eq!(
		get_properties::<VorbisFile>("tests/files/assets/full_test.ogg"),
		VORBIS_PROPERTIES
	)
}

#[test]
fn wav_properties() {
	assert_eq!(
		get_properties::<WavFile>("tests/files/assets/wav_format_pcm.wav"),
		WAV_PROPERTIES
	)
}
