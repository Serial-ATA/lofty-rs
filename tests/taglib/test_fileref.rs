use crate::temp_file;

use std::fs::File;
use std::io::{Read, Seek};

use lofty::error::ErrorKind;
use lofty::resolve::FileResolver;
use lofty::{
	Accessor, AudioFile, FileProperties, FileType, ParseOptions, Tag, TagExt, TagType, TaggedFile,
	TaggedFileExt,
};

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
		tag.set_year(2020);
		tag.save_to(&mut file).unwrap();
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
		assert_eq!(tag.year(), Some(2020));
		tag.set_artist(String::from("ttest artist"));
		tag.set_title(String::from("ytest title"));
		tag.set_genre(String::from("uTest!"));
		tag.set_album(String::from("ialbummmm"));
		tag.set_comment(String::from("another comment"));
		tag.set_track(7);
		tag.set_year(2080);
		tag.save_to(&mut file).unwrap();
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
		assert_eq!(tag.year(), Some(2080));
	}

	// NOTE: All tests following this in the TagLib suite are doing the exact same procedures, just
	//       using their other types: `FileStream` and `ByteVectorStream`. We do not have similar types,
	//       no need to replicate these.
}

#[test]
#[ignore]
fn test_musepack() {
	file_ref_save("click.mpc", FileType::Mpc);
}

#[test]
#[ignore]
fn test_asf() {
	// TODO: We don't support ASF yet
	// file_ref_save("silence-1.asf", FileType::ASF);
}

#[test]
fn test_vorbis() {
	file_ref_save("empty.ogg", FileType::Vorbis);
}

#[test]
fn test_speex() {
	file_ref_save("empty.spx", FileType::Speex);
}

#[test]
fn test_flac() {
	file_ref_save("no-tags.flac", FileType::Flac);
}

#[test]
fn test_mp3() {
	file_ref_save("xing.mp3", FileType::Mpeg);
}

#[test]
#[ignore]
fn test_true_audio() {
	// TODO: We don't support TTA yet
	// file_ref_save("empty.tta", FileType::TrueAudio);
}

#[test]
fn test_mp4_1() {
	file_ref_save("has-tags.m4a", FileType::Mp4);
}

#[test]
#[ignore] // TODO: The file has a malformed `free` atom. How does TagLib handle this? Currently we mess up entirely and just write a duplicate tag.
fn test_mp4_2() {
	file_ref_save("no-tags.m4a", FileType::Mp4);
}

#[test]
#[ignore] // TODO: We are able to write the first tag and even reread, but the second save causes a `SizeMismatch`.
fn test_mp4_3() {
	file_ref_save("no-tags.3g2", FileType::Mp4);
}

#[test]
fn test_mp4_4() {
	file_ref_save("blank_video.m4v", FileType::Mp4);
}

#[test]
fn test_wav() {
	file_ref_save("empty.wav", FileType::Wav);
}

#[test]
#[ignore] // TODO: We don't yet support FLAC in oga
fn test_oga_flac() {
	file_ref_save("empty_flac.oga", FileType::Flac);
}

#[test]
fn test_oga_vorbis() {
	file_ref_save("empty_vorbis.oga", FileType::Vorbis);
}

#[test]
fn test_ape() {
	file_ref_save("mac-399.ape", FileType::Ape);
}

#[test]
fn test_aiff_1() {
	file_ref_save("empty.aiff", FileType::Aiff);
}

#[test]
fn test_aiff_2() {
	file_ref_save("alaw.aifc", FileType::Aiff);
}

#[test]
fn test_wavpack() {
	file_ref_save("click.wv", FileType::WavPack);
}

#[test]
fn test_opus() {
	file_ref_save("correctness_gain_silent_output.opus", FileType::Opus);
}

#[test]
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

#[test]
#[ignore]
fn test_create() {
	// Marker test, Lofty does not replicate this API
}

#[test]
#[ignore] // TODO: We're off by over 200ms
fn test_audio_properties() {
	let file = lofty::read_from_path("tests/taglib/data/xing.mp3").unwrap();
	let properties = file.properties();
	assert_eq!(properties.duration().as_secs(), 2);
	assert_eq!(properties.duration().as_millis(), 2064);
}

#[test]
#[ignore]
fn test_default_file_extensions() {
	// Marker test, Lofty does not replicate this API
}

#[test]
#[ignore] // TODO: We need to check resolvers *first* and then resort to our default implementations
fn test_file_resolver() {
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

		fn read_from<R>(_: &mut R, _: ParseOptions) -> lofty::Result<Self>
		where
			R: Read + Seek,
			Self: Sized,
		{
			Ok(Self)
		}

		fn save_to(&self, _: &mut File) -> lofty::Result<()> {
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

		fn supported_tag_types() -> &'static [TagType] {
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
