use crate::temp_file;
use std::borrow::Cow;
use std::io::{Read, Seek};

use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst, Mp4Codec, Mp4File};
use lofty::{Accessor, AudioFile, MimeType, ParseOptions, Picture, PictureType, TagExt, TagType};

#[test]
fn test_properties_aac() {
	let mut file = temp_file!("tests/taglib/data/has-tags.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3708);
	assert_eq!(f.properties().audio_bitrate(), 3);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), Some(16));
	// TODO: Check for encryption
	// assert_eq!(f.properties().encrypted, false);
	assert_eq!(f.properties().codec(), &Mp4Codec::AAC);
}

#[test]
#[allow(clippy::needless_range_loop)]
fn test_properties_aac_without_bitrate() {
	let mut file = temp_file!("tests/taglib/data/has-tags.m4a");
	let mut aac_data = Vec::new();
	file.read_to_end(&mut aac_data).unwrap();

	assert!(aac_data.len() > 1960);
	assert_eq!(&aac_data[1890..1894], b"mp4a");
	for i in 1956..1960 {
		// Zero out the bitrate
		aac_data[i] = 0;
	}

	let f = Mp4File::read_from(&mut std::io::Cursor::new(aac_data), ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3708);
	assert_eq!(f.properties().audio_bitrate(), 3);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), Some(16));
	// assert_eq!(f.properties().encrypted, false);
	assert_eq!(f.properties().codec(), &Mp4Codec::AAC);
}

#[test]
fn test_properties_alac() {
	let mut file = temp_file!("tests/taglib/data/empty_alac.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3705);
	assert_eq!(f.properties().audio_bitrate(), 3);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), Some(16));
	// assert_eq!(f.properties().encrypted, false);
	assert_eq!(f.properties().codec(), &Mp4Codec::ALAC);
}

#[test]
#[allow(clippy::needless_range_loop)]
fn test_properties_alac_without_bitrate() {
	let mut file = temp_file!("tests/taglib/data/has-tags.m4a");
	let mut alac_data = Vec::new();
	file.read_to_end(&mut alac_data).unwrap();

	assert!(alac_data.len() > 474);
	assert_eq!(&alac_data[446..450], b"alac");
	for i in 470..474 {
		// Zero out the bitrate
		alac_data[i] = 0;
	}

	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3705);
	assert_eq!(f.properties().audio_bitrate(), 3);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), Some(16));
	// assert_eq!(f.properties().encrypted, false);
	assert_eq!(f.properties().codec(), &Mp4Codec::ALAC);
}

#[test]
fn test_properties_m4v() {
	let mut file = temp_file!("tests/taglib/data/blank_video.m4v");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 975);
	assert_eq!(f.properties().audio_bitrate(), 96);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	assert_eq!(f.properties().bit_depth(), Some(16));
	// assert_eq!(f.properties().encrypted, false);
	assert_eq!(f.properties().codec(), &Mp4Codec::AAC);
}

#[test]
fn test_check_valid() {
	let mut file = temp_file!("tests/taglib/data/empty.aiff");
	assert!(Mp4File::read_from(&mut file, ParseOptions::new()).is_err());
}

#[test]
fn test_has_tag() {
	{
		let mut file = temp_file!("tests/taglib/data/has-tags.m4a");
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.ilst().is_some());
	}

	let mut file = temp_file!("tests/taglib/data/no-tags.m4a");

	{
		let mut f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.ilst().is_none());
		let mut tag = Ilst::default();
		tag.set_title(String::from("TITLE"));
		f.set_ilst(tag);
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.ilst().is_some());
	}
}

#[test]
fn test_is_empty() {
	let mut t1 = Ilst::default();
	assert!(t1.is_empty());
	t1.set_artist(String::from("Foo"));
	assert!(!t1.is_empty());
}

#[test]
fn test_update_stco() {
	// TODO: We don't update stco atoms
}

#[test]
fn test_freeform() {
	let mut file = temp_file!("tests/taglib/data/has-tags.m4a");

	{
		let mut f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.ilst().unwrap().contains(&AtomIdent::Freeform {
			mean: Cow::Borrowed("com.apple.iTunes"),
			name: Cow::Borrowed("iTunNORM"),
		}));

		f.ilst_mut().unwrap().insert_atom(Atom::new(
			AtomIdent::Freeform {
				mean: Cow::Borrowed("org.kde.TagLib"),
				name: Cow::Borrowed("Foo"),
			},
			AtomData::UTF8(String::from("Bar")),
		));
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.ilst().unwrap().contains(&AtomIdent::Freeform {
			mean: Cow::Borrowed("org.kde.TagLib"),
			name: Cow::Borrowed("Foo"),
		}));
		assert_eq!(
			f.ilst()
				.unwrap()
				.atom(&AtomIdent::Freeform {
					mean: Cow::Borrowed("org.kde.TagLib"),
					name: Cow::Borrowed("Foo"),
				})
				.unwrap()
				.data()
				.next(),
			Some(&AtomData::UTF8(String::from("Bar")))
		);
		f.save_to(&mut file).unwrap();
	}
}

#[test]
fn test_save_existing_when_ilst_is_last() {
	let mut file = temp_file!("tests/taglib/data/ilst-is-last.m4a");

	{
		let mut f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let ilst = f.ilst_mut().unwrap();
		assert_eq!(
			ilst.atom(&AtomIdent::Freeform {
				mean: Cow::Borrowed("com.apple.iTunes"),
				name: Cow::Borrowed("replaygain_track_minmax"),
			})
			.unwrap()
			.data()
			.next()
			.unwrap(),
			&AtomData::UTF8(String::from("82,164"))
		);
		assert_eq!(ilst.artist().as_deref(), Some("Pearl Jam"));
		ilst.set_comment(String::from("foo"));
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		let ilst = f.ilst().unwrap();

		assert_eq!(
			ilst.atom(&AtomIdent::Freeform {
				mean: Cow::Borrowed("com.apple.iTunes"),
				name: Cow::Borrowed("replaygain_track_minmax"),
			})
			.unwrap()
			.data()
			.next()
			.unwrap(),
			&AtomData::UTF8(String::from("82,164"))
		);
		assert_eq!(ilst.artist().as_deref(), Some("Pearl Jam"));
		assert_eq!(ilst.comment().as_deref(), Some("foo"));
	}
}

#[test]
#[ignore] // TODO: Maybe? This just checks the moov atom's length. We don't retain any atoms we don't need.
fn test_64bit_atom() {}

#[test]
fn test_gnre() {
	let mut file = temp_file!("tests/taglib/data/gnre.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.ilst().unwrap().genre().as_deref(), Some("Ska"));
}

#[test]
fn test_covr_read() {
	let mut file = temp_file!("tests/taglib/data/has-tags.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	let tag = f.ilst().unwrap();
	assert!(tag.contains(&AtomIdent::Fourcc(*b"covr")));
	let mut covrs = tag.atom(&AtomIdent::Fourcc(*b"covr")).unwrap().data();
	let Some(AtomData::Picture(picture1)) = covrs.next() else {
		unreachable!()
	};
	let Some(AtomData::Picture(picture2)) = covrs.next() else {
		unreachable!()
	};

	assert!(covrs.next().is_none());
	assert_eq!(picture1.mime_type(), &MimeType::Png);
	assert_eq!(picture1.data().len(), 79);
	assert_eq!(picture2.mime_type(), &MimeType::Jpeg);
	assert_eq!(picture2.data().len(), 287);
}

#[test]
fn test_covr_write() {
	let mut file = temp_file!("tests/taglib/data/has-tags.m4a");

	{
		let mut f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let tag = f.ilst_mut().unwrap();
		assert!(tag.contains(&AtomIdent::Fourcc(*b"covr")));
		tag.insert_picture(Picture::new_unchecked(
			PictureType::Other,
			MimeType::Png,
			None,
			b"foo".to_vec(),
		));
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		let tag = f.ilst().unwrap();
		assert!(tag.contains(&AtomIdent::Fourcc(*b"covr")));

		let mut covrs = tag.atom(&AtomIdent::Fourcc(*b"covr")).unwrap().data();
		let Some(AtomData::Picture(picture1)) = covrs.next() else {
			unreachable!()
		};
		let Some(AtomData::Picture(picture2)) = covrs.next() else {
			unreachable!()
		};
		let Some(AtomData::Picture(picture3)) = covrs.next() else {
			unreachable!()
		};

		assert!(covrs.next().is_none());
		assert_eq!(picture1.mime_type(), &MimeType::Png);
		assert_eq!(picture1.data().len(), 79);
		assert_eq!(picture2.mime_type(), &MimeType::Jpeg);
		assert_eq!(picture2.data().len(), 287);
		assert_eq!(picture3.mime_type(), &MimeType::Png);
		assert_eq!(picture3.data().len(), 3);
	}
}

#[test]
fn test_covr_read2() {
	let mut file = temp_file!("tests/taglib/data/covr-junk.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	let tag = f.ilst().unwrap();
	assert!(tag.contains(&AtomIdent::Fourcc(*b"covr")));
	let mut covrs = tag.atom(&AtomIdent::Fourcc(*b"covr")).unwrap().data();
	let Some(AtomData::Picture(picture1)) = covrs.next() else {
		unreachable!()
	};
	let Some(AtomData::Picture(picture2)) = covrs.next() else {
		unreachable!()
	};

	assert!(covrs.next().is_none());
	assert_eq!(picture1.mime_type(), &MimeType::Png);
	assert_eq!(picture1.data().len(), 79);
	assert_eq!(picture2.mime_type(), &MimeType::Jpeg);
	assert_eq!(picture2.data().len(), 287);
}

#[test]
#[ignore]
fn test_properties() {
	// Marker test, Lofty does not replicate the properties API
}

#[test]
#[ignore]
fn test_properties_all_supported() {
	// Marker test, Lofty does not replicate the properties API
}

#[test]
#[ignore]
fn test_properties_movement() {
	// Marker test, Lofty does not replicate the properties API
}

#[test]
fn test_fuzzed_file() {
	let mut file = temp_file!("tests/taglib/data/infloop.m4a");
	let _ = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
}

#[test]
fn test_repeated_save() {
	let mut file = temp_file!("tests/taglib/data/no-tags.m4a");
	let mut f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	let mut tag = Ilst::default();
	tag.set_title(String::from("0123456789"));
	f.set_ilst(tag);

	f.save_to(&mut file).unwrap();
	file.rewind().unwrap();
	f.save_to(&mut file).unwrap();
	file.rewind().unwrap();

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes).unwrap();

	assert_eq!(
		file_bytes
			.windows(10)
			.position(|window| window == b"0123456789"),
		Some(2862)
	);
	assert_ne!(file_bytes.get(2863..2873), Some(b"0123456789".as_slice()));
}

#[test]
fn test_with_zero_length_atom() {
	let mut file = temp_file!("tests/taglib/data/infloop.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_millis(), 1115);
	assert_eq!(f.properties().sample_rate(), 22050);
}

#[test]
#[ignore]
fn test_empty_values_remove_items() {
	// Marker test, Lofty treats empty values as valid
}

#[test]
fn test_remove_metadata() {
	let mut file = temp_file!("tests/taglib/data/no-tags.m4a");

	{
		let mut f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.ilst().is_none());
		let mut tag = Ilst::default();
		assert!(tag.is_empty());
		tag.set_title(String::from("TITLE"));
		f.set_ilst(tag);
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.ilst().is_some());
		assert!(!f.ilst().unwrap().is_empty());
		TagType::MP4ilst.remove_from(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.ilst().is_none());

		let mut original_file_bytes = Vec::new();
		let mut new_file_bytes = Vec::new();

		let mut original_file = temp_file!("tests/taglib/data/no-tags.m4a");
		original_file.read_to_end(&mut original_file_bytes).unwrap();
		file.read_to_end(&mut new_file_bytes).unwrap();

		// We need to do some editing, since we preserve the `udta` and `meta` atoms unlike TagLib

		// Remove the `udta` atom, which should be 53 bytes in length
		new_file_bytes.splice(1505..1505 + 53, std::iter::empty());

		// Fix the length of the `moov` atom
		new_file_bytes[1500] = 0;

		assert_eq!(original_file_bytes, new_file_bytes);
	}
}

#[test]
fn test_non_full_meta_atom() {
	let mut file = temp_file!("tests/taglib/data/non-full-meta.m4a");
	let f = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.ilst().is_some());

	let tag = f.ilst().unwrap();
	assert!(tag.contains(&AtomIdent::Fourcc(*b"covr")));
	let mut covrs = tag.atom(&AtomIdent::Fourcc(*b"covr")).unwrap().data();
	let Some(AtomData::Picture(picture1)) = covrs.next() else {
		unreachable!()
	};
	let Some(AtomData::Picture(picture2)) = covrs.next() else {
		unreachable!()
	};

	assert!(covrs.next().is_none());
	assert_eq!(picture1.mime_type(), &MimeType::Png);
	assert_eq!(picture1.data().len(), 79);
	assert_eq!(picture2.mime_type(), &MimeType::Jpeg);
	assert_eq!(picture2.data().len(), 287);

	assert_eq!(tag.artist().as_deref(), Some("Test Artist!!!!"));
	assert_eq!(
		tag.atom(&AtomIdent::Fourcc(*b"\xa9too"))
			.unwrap()
			.data()
			.next()
			.unwrap(),
		&AtomData::UTF8(String::from("FAAC 1.24"))
	);
}
