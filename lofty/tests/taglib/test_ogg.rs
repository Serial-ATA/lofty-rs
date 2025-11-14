use crate::temp_file;
use crate::util::get_file;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::ogg::VorbisFile;
use lofty::tag::Accessor;

#[test_log::test]
fn test_simple() {
	let mut file = temp_file!("tests/taglib/data/empty.ogg");

	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.set_artist(String::from("The Artist"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.vorbis_comments().artist().as_deref(), Some("The Artist"));
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't retain packet information"]
fn test_split_packets1() {}

#[test_log::test]
fn test_split_packets2() {
	let mut file = temp_file!("tests/taglib/data/empty.ogg");

	let text = "X".repeat(60890);
	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut().set_title(text.clone());
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(f.vorbis_comments().title().as_deref(), Some(&*text));

		f.vorbis_comments_mut().set_title(String::from("ABCDE"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.vorbis_comments().title().as_deref(), Some("ABCDE"));
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't replicate the dictionary interface"]
fn test_dict_interface1() {}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't replicate the dictionary interface"]
fn test_dict_interface2() {}

#[test_log::test]
fn test_audio_properties() {
	let f = get_file::<VorbisFile>("tests/taglib/data/empty.ogg");
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3685);
	assert_eq!(f.properties().audio_bitrate(), 112); // TagLib reports 1? That is not correct.
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().version(), 0);
	assert_eq!(f.properties().bitrate_max(), 0);
	assert_eq!(f.properties().bitrate_nominal(), 112_000);
	assert_eq!(f.properties().bitrate_min(), 0);
}

// TODO: Need to look into this one, not sure why there's a difference in checksums
#[test_log::test]
#[ignore = "Needs to be looked into more"]
fn test_page_checksum() {
	let mut file = temp_file!("tests/taglib/data/empty.ogg");

	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.set_title(String::from("The Artist"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();

		file.seek(SeekFrom::Start(0x50)).unwrap();
		assert_eq!(file.read_u32::<LittleEndian>().unwrap(), 0x3D3B_D92D);
	}
	file.rewind().unwrap();
	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.set_title(String::from("The Artist 2"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();

		file.seek(SeekFrom::Start(0x50)).unwrap();
		assert_eq!(file.read_u32::<LittleEndian>().unwrap(), 0xD985_291C);
	}
}

#[test_log::test]
fn test_page_granule_position() {
	let mut file = temp_file!("tests/taglib/data/empty.ogg");

	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		// Force the Vorbis comment packet to span more than one page and
		// check if the granule position is -1 indicating that no packets
		// finish on this page.
		f.vorbis_comments_mut().set_comment("A".repeat(70000));
		f.save_to(&mut file, WriteOptions::default()).unwrap();

		file.seek(SeekFrom::Start(0x3A)).unwrap();
		let mut buf = [0; 6];
		file.read_exact(&mut buf).unwrap();
		assert_eq!(buf, *b"OggS\0\0");
		assert_eq!(file.read_i64::<LittleEndian>().unwrap(), -1);
	}
	file.rewind().unwrap();
	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		// Use a small Vorbis comment package which ends on the seconds page and
		// check if the granule position is zero.
		f.vorbis_comments_mut()
			.set_comment(String::from("A small comment"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();

		file.seek(SeekFrom::Start(0x3A)).unwrap();
		let mut buf = [0; 6];
		file.read_exact(&mut buf).unwrap();
		assert_eq!(buf, *b"OggS\0\0");
		assert_eq!(file.read_i64::<LittleEndian>().unwrap(), 0);
	}
}
