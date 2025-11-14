use crate::temp_file;

use std::io::{Seek, SeekFrom};

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::flac::FlacFile;
use lofty::ogg::VorbisComments;
use lofty::tag::Accessor;

// TODO: We don't support FLAC in OGA (#172)
#[test_log::test]
#[ignore = "FLAC in OGA isn't supported yet"]
fn test_framing_bit() {
	let mut file = temp_file!("tests/taglib/data/empty_flac.oga");

	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut vorbis_comments = VorbisComments::new();
		vorbis_comments.set_artist(String::from("The Artist"));
		f.set_vorbis_comments(vorbis_comments);
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(
			f.vorbis_comments().unwrap().artist().as_deref(),
			Some("The Artist")
		);

		assert_eq!(file.seek(SeekFrom::End(0)).unwrap(), 9134);
	}
}

// TODO: We don't support FLAC in OGA (#172)
#[test_log::test]
#[ignore = "FLAC in OGA isn't supported yet"]
fn test_fuzzed_file() {
	let mut file = temp_file!("tests/taglib/data/segfault.oga");
	let f = FlacFile::read_from(&mut file, ParseOptions::new());
	assert!(f.is_err());
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not retain the packet information"]
fn test_split_packets() {}
