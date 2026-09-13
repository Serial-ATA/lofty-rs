use crate::temp_file;
use crate::util::get_file;

use std::borrow::Cow;
use std::io::Seek;

use lofty::TextEncoding;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::dsf::{DsfFile, FormatId};
use lofty::file::AudioFile;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::properties::ChannelMask;
use lofty::tag::Accessor;

#[test_log::test]
fn test_basic() {
	let f = get_file::<DsfFile>("tests/taglib/data/empty10ms.dsf");
	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 10);
	assert_eq!(f.properties().audio_bitrate(), 5645);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 2_822_400);
	assert_eq!(f.properties().format_id(), FormatId::DsdRaw);
	assert_eq!(f.properties().channel_mask(), ChannelMask::stereo());
	assert_eq!(f.properties().bit_depth(), 1);
	assert_eq!(f.properties().sample_count(), 28224);
	assert_eq!(f.properties().block_size(), 4096);
}

#[test_log::test]
fn test_tags() {
	const ALBUM_ARTIST: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TPE2"));

	let mut file = temp_file!("tests/taglib/data/empty10ms.dsf");

	{
		let mut f = DsfFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.id3v2().is_none());
		let mut tag = Id3v2Tag::new();
		tag.set_artist(String::from("The Artist"));
		tag.insert(Frame::Text(TextInformationFrame::new(
			ALBUM_ARTIST,
			TextEncoding::Latin1,
			"Album Artist",
		)));
		f.set_id3v2(tag);

		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = DsfFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(f.id3v2().unwrap().artist().as_deref(), Some("The Artist"));
		assert_eq!(
			f.id3v2().unwrap().get_text(&ALBUM_ARTIST),
			Some("Album Artist")
		);
		f.id3v2_mut().unwrap().clear();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
}
