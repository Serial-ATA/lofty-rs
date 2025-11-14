use crate::temp_file;

use std::io::{Read, Seek};

use lofty::config::{GlobalOptions, ParseOptions, WriteOptions};
use lofty::error::{ErrorKind, LoftyError};
use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::resolve::FileResolver;
use lofty::tag::{Accessor, Tag, TagExt, TagSupport, TagType};

fn file_ref_save(path: &str, expected_file_type: FileType) {
	let path = format!("tests/taglib/data/{path}");
	let mut file = temp_file!(path);
	{
		let mut f = lofty::read_from(&mut file).unwrap();
		file.rewind().unwrap();

		assert_eq!(f.file_type(), expected_file_type);

		let tag = match f.primary_tag_mut() {
			Some(tag) => tag,
			None => {
				f.insert_tag(Tag::new(f.primary_tag_type()));
				f.primary_tag_mut().unwrap()
			},
		};
		tag.set_artist(String::from("test artist"));
		tag.set_title(String::from("test title"));
		tag.set_genre(String::from("Test!"));
		tag.set_album(String::from("albummmm"));
		tag.set_comment(String::from("a comment"));
		tag.set_track(5);
		tag.set_date(Timestamp {
			year: 2020,
			..Timestamp::default()
		});
		tag.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = lofty::read_from(&mut file).unwrap();
		file.rewind().unwrap();

		let tag = f.primary_tag_mut().unwrap();
		assert_eq!(tag.artist().as_deref(), Some("test artist"));
		assert_eq!(tag.title().as_deref(), Some("test title"));
		assert_eq!(tag.genre().as_deref(), Some("Test!"));
		assert_eq!(tag.album().as_deref(), Some("albummmm"));
		assert_eq!(tag.comment().as_deref(), Some("a comment"));
		assert_eq!(tag.track(), Some(5));
		assert_eq!(tag.date().map(|date| date.year), Some(2020));
		tag.set_artist(String::from("ttest artist"));
		tag.set_title(String::from("ytest title"));
		tag.set_genre(String::from("uTest!"));
		tag.set_album(String::from("ialbummmm"));
		tag.set_comment(String::from("another comment"));
		tag.set_track(7);
		tag.set_date(Timestamp {
			year: 2080,
			..Timestamp::default()
		});
		tag.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = lofty::read_from(&mut file).unwrap();
		file.rewind().unwrap();

		let tag = f.primary_tag_mut().unwrap();
		assert_eq!(tag.artist().as_deref(), Some("ttest artist"));
		assert_eq!(tag.title().as_deref(), Some("ytest title"));
		assert_eq!(tag.genre().as_deref(), Some("uTest!"));
		assert_eq!(tag.album().as_deref(), Some("ialbummmm"));
		assert_eq!(tag.comment().as_deref(), Some("another comment"));
		assert_eq!(tag.track(), Some(7));
		assert_eq!(tag.date().map(|date| date.year), Some(2080));
	}

	// NOTE: All tests following this in the TagLib suite are doing the exact same procedures, just
	//       using their other types: `FileStream` and `ByteVectorStream`. We do not have similar types,
	//       no need to replicate these.
}

#[test_log::test]
fn test_musepack() {
	file_ref_save("click.mpc", FileType::Mpc);
}

// TODO: We don't support ASF yet
#[test_log::test]
#[ignore = "AFK is not supported yet"]
fn test_asf() {
	// file_ref_save("silence-1.asf", FileType::ASF);
}

#[test_log::test]
fn test_vorbis() {
	file_ref_save("empty.ogg", FileType::Vorbis);
}

#[test_log::test]
fn test_speex() {
	file_ref_save("empty.spx", FileType::Speex);
}

#[test_log::test]
fn test_flac() {
	file_ref_save("no-tags.flac", FileType::Flac);
}

#[test_log::test]
fn test_mp3() {
	file_ref_save("xing.mp3", FileType::Mpeg);
}

// TODO: We don't support TTA yet
#[test_log::test]
#[ignore = "TTA is not supported yet"]
fn test_true_audio() {
	// file_ref_save("empty.tta", FileType::TrueAudio);
}

#[test_log::test]
fn test_mp4_1() {
	file_ref_save("has-tags.m4a", FileType::Mp4);
}

#[test_log::test]
fn test_mp4_2() {
	file_ref_save("no-tags.m4a", FileType::Mp4);
}

#[test_log::test]
fn test_mp4_3() {
	file_ref_save("no-tags.3g2", FileType::Mp4);
}

#[test_log::test]
fn test_mp4_4() {
	file_ref_save("blank_video.m4v", FileType::Mp4);
}

#[test_log::test]
fn test_wav() {
	file_ref_save("empty.wav", FileType::Wav);
}

// TODO: We don't yet support FLAC in oga
#[test_log::test]
#[ignore = "FLAC in OGA isn't supported yet"]
fn test_oga_flac() {
	file_ref_save("empty_flac.oga", FileType::Flac);
}

#[test_log::test]
fn test_oga_vorbis() {
	file_ref_save("empty_vorbis.oga", FileType::Vorbis);
}

#[test_log::test]
fn test_ape() {
	file_ref_save("mac-399.ape", FileType::Ape);
}

#[test_log::test]
fn test_aiff_1() {
	file_ref_save("empty.aiff", FileType::Aiff);
}

#[test_log::test]
fn test_aiff_2() {
	file_ref_save("alaw.aifc", FileType::Aiff);
}

#[test_log::test]
fn test_wavpack() {
	file_ref_save("click.wv", FileType::WavPack);
}

#[test_log::test]
fn test_opus() {
	file_ref_save("correctness_gain_silent_output.opus", FileType::Opus);
}

#[test_log::test]
fn test_unsupported() {
	let f1 = lofty::read_from_path("tests/taglib/data/no-extension");
	match f1 {
		Err(err) if matches!(err.kind(), ErrorKind::UnknownFormat) => {},
		_ => panic!("File with no extension got through `read_from_path!`"),
	}

	let f2 = lofty::read_from_path("tests/taglib/data/unsupported-extension.xx");
	match f2 {
		Err(err) if matches!(err.kind(), ErrorKind::UnknownFormat) => {},
		_ => panic!("File with unsupported extension got through `read_from_path!`"),
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate this API"]
fn test_create() {}

#[test_log::test]
fn test_audio_properties() {
	let file = lofty::read_from_path("tests/taglib/data/xing.mp3").unwrap();
	let properties = file.properties();
	assert_eq!(properties.duration().as_secs(), 2);
	assert_eq!(properties.duration().as_millis(), 2064);
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate this API"]
fn test_default_file_extensions() {}

use lofty::io::{FileLike, Length, Truncate};
use lofty::properties::FileProperties;
use lofty::tag::items::Timestamp;
use rusty_fork::rusty_fork_test;

rusty_fork_test! {
	#[test_log::test]
	fn test_file_resolver() {
		lofty::config::apply_global_options(GlobalOptions::new().use_custom_resolvers(true));

		{
			let file = lofty::read_from_path("tests/taglib/data/xing.mp3").unwrap();
			assert_eq!(file.file_type(), FileType::Mpeg);
		}

		struct DummyResolver;
		impl Into<TaggedFile> for DummyResolver {
			fn into(self) -> TaggedFile {
				TaggedFile::new(FileType::Vorbis, FileProperties::default(), Vec::new())
			}
		}

		impl AudioFile for DummyResolver {
			type Properties = ();

			fn read_from<R>(_: &mut R, _: ParseOptions) -> lofty::error::Result<Self>
			where
				R: Read + Seek,
				Self: Sized,
			{
				Ok(Self)
			}

			fn save_to<F>(&self, _: &mut F, _: WriteOptions) -> lofty::error::Result<()>
			where
				F: FileLike,
				LoftyError: From<<F as Truncate>::Error>,
				LoftyError: From<<F as Length>::Error>
			{
				unimplemented!()
			}

			fn properties(&self) -> &Self::Properties {
				unimplemented!()
			}

			fn contains_tag(&self) -> bool {
				unimplemented!()
			}

			fn contains_tag_type(&self, _: TagType) -> bool {
				unimplemented!()
			}
		}

		impl FileResolver for DummyResolver {
			fn extension() -> Option<&'static str> {
				Some("mp3")
			}

			fn primary_tag_type() -> TagType {
				unimplemented!()
			}

			fn tag_support(_tag_type: TagType) -> TagSupport {
				unimplemented!()
			}

			fn guess(_: &[u8]) -> Option<FileType> {
				Some(FileType::Vorbis)
			}
		}

		lofty::resolve::register_custom_resolver::<DummyResolver>("Dummy");

		{
			let file = lofty::read_from_path("tests/taglib/data/xing.mp3").unwrap();
			assert_eq!(file.file_type(), FileType::Vorbis);
		}
	}
}
