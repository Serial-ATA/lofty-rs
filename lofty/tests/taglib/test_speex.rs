use crate::temp_file;
use crate::util::get_file;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::ogg::{SpeexFile, VorbisComments};
use lofty::tag::Accessor;

use std::io::Seek;

#[test_log::test]
fn test_audio_properties() {
	let f = get_file::<SpeexFile>("tests/taglib/data/empty.spx");

	assert_eq!(f.properties().duration().as_secs(), 3);
	// TODO: We report 3684, we're off by one
	assert_eq!(f.properties().duration().as_millis(), 3685);
	// TODO: We report zero, we aren't properly calculating bitrates for Speex
	assert_eq!(f.properties().audio_bitrate(), 53);
	assert_eq!(f.properties().nominal_bitrate(), -1);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

// TODO: This test doesn't work, it's very specific with file/packet sizes. Have to determine whether or not to even keep this one.
#[test_log::test]
#[ignore = "Needs to be looked into more"]
fn test_split_packets() {
	let mut file = temp_file!("tests/taglib/data/empty.spx");

	let text = String::from_utf8(vec![b'X'; 128 * 1024]).unwrap();

	{
		let f = SpeexFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = VorbisComments::default();
		tag.set_title(text.clone());
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = SpeexFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(file.metadata().unwrap().len(), 156_330);
		assert_eq!(f.vorbis_comments().title().as_deref(), Some(text.as_str()));

		// NOTE: TagLib exposes the packets and page headers through `Speex::File`.
		//       Lofty does not keep this information around, so we just double check with `ogg_pager`.
		let packets = ogg_pager::Packets::read(&mut file).unwrap();
		assert_eq!(packets.get(0).unwrap().len(), 80);
		assert_eq!(packets.get(1).unwrap().len(), 131_116);
		assert_eq!(packets.get(2).unwrap().len(), 93);
		assert_eq!(packets.get(3).unwrap().len(), 93);

		assert_eq!(f.properties().duration().as_millis(), 3685);

		f.vorbis_comments_mut().set_title(String::from("ABCDE"));
		file.rewind().unwrap();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = SpeexFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(file.metadata().unwrap().len(), 24317);
		assert_eq!(f.vorbis_comments().title().as_deref(), Some("ABCDE"));

		let packets = ogg_pager::Packets::read(&mut file).unwrap();
		assert_eq!(packets.get(0).unwrap().len(), 80);
		assert_eq!(packets.get(1).unwrap().len(), 49);
		assert_eq!(packets.get(2).unwrap().len(), 93);
		assert_eq!(packets.get(3).unwrap().len(), 93);

		assert_eq!(f.properties().duration().as_millis(), 3685);
	}
}
