use crate::temp_file;
use crate::util::get_file;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::id3::v2::{Id3v2Tag, Id3v2Version};
use lofty::iff::wav::{RiffInfoList, WavFile, WavFormat};
use lofty::tag::{Accessor, TagType};

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

#[test_log::test]
fn test_pcm_properties() {
	let f = get_file::<WavFile>("tests/taglib/data/empty.wav");
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3675);
	assert_eq!(f.properties().bitrate(), 32);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 1000);
	assert_eq!(f.properties().bit_depth(), 16);
	// TODO: assert_eq!(f.properties().total_samples(), 3675);
	assert_eq!(*f.properties().format(), WavFormat::PCM);
}

#[test_log::test]
fn test_alaw_properties() {
	let f = get_file::<WavFile>("tests/taglib/data/alaw.wav");
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3550);
	assert_eq!(f.properties().bitrate(), 128);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 8000);
	assert_eq!(f.properties().bit_depth(), 8);
	// TODO: assert_eq!(f.properties().total_samples(), 28400);
	assert_eq!(*f.properties().format(), WavFormat::Other(6));
}

#[test_log::test]
fn test_float_properties() {
	let f = get_file::<WavFile>("tests/taglib/data/float64.wav");
	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 97);
	assert_eq!(f.properties().bitrate(), 5645);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), 64);
	// TODO: assert_eq!(f.properties().total_samples(), 4281);
	assert_eq!(*f.properties().format(), WavFormat::IEEE_FLOAT);
}

#[test_log::test]
fn test_float_without_fact_chunk_properties() {
	let mut wav_data = std::fs::read("tests/taglib/data/float64.wav").unwrap();
	assert_eq!(&wav_data[36..40], b"fact");

	// Remove the fact chunk by renaming it to fakt
	wav_data[38] = b'k';

	let f = WavFile::read_from(&mut Cursor::new(wav_data), ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 97);
	assert_eq!(f.properties().bitrate(), 5645);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), 64);
	// TODO: assert_eq!(f.properties().total_samples(), 4281);
	assert_eq!(*f.properties().format(), WavFormat::IEEE_FLOAT);
}

#[test_log::test]
fn test_zero_size_data_chunk() {
	let mut file = temp_file!("tests/taglib/data/zero-size-chunk.wav");
	let _f = WavFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
}

#[test_log::test]
fn test_id3v2_tag() {
	let mut file = temp_file!("tests/taglib/data/empty.wav");

	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_none());

		let mut id3v2 = Id3v2Tag::default();
		id3v2.set_title(String::from("Title"));
		id3v2.set_artist(String::from("Artist"));
		f.set_id3v2(id3v2);
		f.save_to(&mut file, WriteOptions::default()).unwrap();
		assert!(f.id3v2().is_some());
	}
	file.rewind().unwrap();
	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_some());

		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some("Title"));
		assert_eq!(f.id3v2().unwrap().artist().as_deref(), Some("Artist"));

		f.id3v2_mut().unwrap().remove_title();
		f.id3v2_mut().unwrap().remove_artist();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_none());
	}
}

#[test_log::test]
fn test_save_id3v23() {
	let mut file = temp_file!("tests/taglib/data/empty.wav");

	let xxx = "X".repeat(254);
	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_none());

		let mut id3v2 = Id3v2Tag::new();
		id3v2.set_title(xxx.clone());
		id3v2.set_artist(String::from("Artist A"));
		f.set_id3v2(id3v2);

		f.save_to(&mut file, WriteOptions::new().use_id3v23(true))
			.unwrap();
		assert!(f.id3v2().is_some());
	}
	file.rewind().unwrap();
	{
		let f2 = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let tag = f2.id3v2().unwrap();
		assert_eq!(tag.original_version(), Id3v2Version::V3);
		assert_eq!(tag.artist().as_deref(), Some("Artist A"));
		assert_eq!(tag.title().as_deref(), Some(&*xxx));
	}
}

#[test_log::test]
fn test_info_tag() {
	let mut file = temp_file!("tests/taglib/data/empty.wav");

	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.riff_info().is_none());

		let mut riff_info = RiffInfoList::default();
		riff_info.set_title(String::from("Title"));
		riff_info.set_artist(String::from("Artist"));
		f.set_riff_info(riff_info);

		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.riff_info().is_some());
		assert_eq!(f.riff_info().unwrap().title().as_deref(), Some("Title"));
		assert_eq!(f.riff_info().unwrap().artist().as_deref(), Some("Artist"));

		f.riff_info_mut().unwrap().remove_title();
		f.riff_info_mut().unwrap().remove_artist();

		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.riff_info().is_none());
	}
}

#[test_log::test]
fn test_strip_tags() {
	let mut file = temp_file!("tests/taglib/data/empty.wav");

	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut id3v2 = Id3v2Tag::default();
		id3v2.set_title(String::from("test title"));
		f.set_id3v2(id3v2);

		let mut riff_info = RiffInfoList::default();
		riff_info.set_title(String::from("test title"));
		f.set_riff_info(riff_info);

		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_some());
		assert!(f.riff_info().is_some());

		TagType::RiffInfo.remove_from(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_some());
		assert!(f.riff_info().is_none());

		let mut riff_info = RiffInfoList::default();
		riff_info.set_title(String::from("test title"));
		f.set_riff_info(riff_info);

		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_some());
		assert!(f.riff_info().is_some());

		TagType::Id3v2.remove_from(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_none());
		assert!(f.riff_info().is_some());
	}
}

#[test_log::test]
#[ignore = "Marker test, TagLib will ignore any tag except for the first."]
fn test_duplicate_tags() {
	// Lofty will *not* do this. Every tag in the stream is read and merged into the previous one. Whichever tag ends up
	// being the latest in the stream will have precedence.
}

#[test_log::test]
fn test_fuzzed_file1() {
	let f1 = get_file::<WavFile>("tests/taglib/data/infloop.wav");
	// The file has problems:
	// Chunk 'ISTt' has invalid size (larger than the file size).
	// Its properties can nevertheless be read.
	let properties = f1.properties();
	assert_eq!(1, properties.channels());
	assert_eq!(88, properties.bitrate());
	assert_eq!(8, properties.bit_depth());
	assert_eq!(11025, properties.sample_rate());
	assert!(f1.riff_info().is_none());
	assert!(f1.id3v2().is_none());
}

#[test_log::test]
fn test_fuzzed_file2() {
	let mut file = temp_file!("tests/taglib/data/segfault.wav");
	let _f2 = WavFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
}

#[test_log::test]
fn test_file_with_garbage_appended() {
	let mut file = temp_file!("tests/taglib/data/empty.wav");
	let contents_before_modification;
	{
		file.seek(SeekFrom::End(0)).unwrap();

		let garbage = b"12345678";
		file.write_all(garbage).unwrap();
		file.rewind().unwrap();

		let mut file_contents = Vec::new();
		file.read_to_end(&mut file_contents).unwrap();

		contents_before_modification = file_contents;
	}
	file.rewind().unwrap();
	{
		let mut f = WavFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut id3v2 = Id3v2Tag::default();
		id3v2.set_title(String::from("ID3v2 Title"));
		f.set_id3v2(id3v2);

		let mut riff_info = RiffInfoList::default();
		riff_info.set_title(String::from("INFO Title"));
		f.set_riff_info(riff_info);

		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		TagType::Id3v2.remove_from(&mut file).unwrap();
		file.rewind().unwrap();
		TagType::RiffInfo.remove_from(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut contents_after_modification = Vec::new();
		file.read_to_end(&mut contents_after_modification).unwrap();
		assert_eq!(contents_before_modification, contents_after_modification);
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the properties API"]
fn test_strip_and_properties() {}

#[test_log::test]
fn test_pcm_with_fact_chunk() {
	let f = get_file::<WavFile>("tests/taglib/data/pcm_with_fact_chunk.wav");
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3675);
	assert_eq!(f.properties().bitrate(), 32);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 1000);
	assert_eq!(f.properties().bit_depth(), 16);
	// TODO: assert_eq!(f.properties().total_samples(), 3675);
	assert_eq!(*f.properties().format(), WavFormat::PCM);
}

#[test_log::test]
fn test_wave_format_extensible() {
	let f = get_file::<WavFile>("tests/taglib/data/uint8we.wav");
	assert_eq!(f.properties().duration().as_secs(), 2);
	assert_eq!(f.properties().duration().as_millis(), 2937);
	assert_eq!(f.properties().bitrate(), 128);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 8000);
	assert_eq!(f.properties().bit_depth(), 8);
	// TODO: assert_eq!(f.properties().total_samples(), 23493);
	assert_eq!(*f.properties().format(), WavFormat::PCM);
}
