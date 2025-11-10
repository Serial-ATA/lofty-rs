use crate::temp_file;
use crate::util::get_file;

use std::io::Seek;

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::ogg::OpusFile;
use lofty::tag::Accessor;

#[test_log::test]
fn test_audio_properties() {
	let f = get_file::<OpusFile>("tests/taglib/data/correctness_gain_silent_output.opus");
	assert_eq!(f.properties().duration().as_secs(), 7);
	assert_eq!(f.properties().duration().as_millis(), 7737);
	assert_eq!(f.properties().audio_bitrate(), 36);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().input_sample_rate(), 48000);
	assert_eq!(f.properties().version(), 1);
}

#[test_log::test]
fn test_read_comments() {
	let f = get_file::<OpusFile>("tests/taglib/data/correctness_gain_silent_output.opus");
	assert_eq!(
		f.vorbis_comments().get("ENCODER"),
		Some("Xiph.Org Opus testvectormaker")
	);
	assert!(f.vorbis_comments().get("TESTDESCRIPTION").is_some());
	assert!(f.vorbis_comments().artist().is_none());
	assert_eq!(f.vorbis_comments().vendor(), "libopus 0.9.11-66-g64c2dd7");
}

#[test_log::test]
fn test_write_comments() {
	let mut file = temp_file!("tests/taglib/data/correctness_gain_silent_output.opus");

	{
		let mut f = OpusFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		f.vorbis_comments_mut()
			.set_artist(String::from("Your Tester"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = OpusFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(
			f.vorbis_comments().get("ENCODER"),
			Some("Xiph.Org Opus testvectormaker")
		);
		assert!(f.vorbis_comments().get("TESTDESCRIPTION").is_some());
		assert_eq!(f.vorbis_comments().artist().as_deref(), Some("Your Tester"));
		assert_eq!(f.vorbis_comments().vendor(), "libopus 0.9.11-66-g64c2dd7");
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not retain packet information"]
fn test_split_packets() {}
