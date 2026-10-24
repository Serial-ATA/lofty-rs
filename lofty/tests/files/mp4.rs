use lofty::config::{ParseOptions, ParsingMode, WriteOptions};
use lofty::file::FileType;
use lofty::mp4::Mp4File;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};

use std::fs::File;
use std::io::{Read, Seek};

#[test_log::test]
fn read() {
	// This file contains an ilst atom
	let file = Probe::open("tests/files/assets/minimal/m4a_codec_aac.m4a")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Mp4);

	// Verify the ilst tag
	crate::util::verify_artist(&file, TagType::Mp4Ilst, "Foo artist", 1);
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/m4a_codec_aac.m4a");

	assert_eq!(tagged_file.file_type(), FileType::Mp4);

	// ilst
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Mp4Ilst,
		"Foo artist",
		"Bar artist",
		1,
	);

	// Now reread the file
	let mut file = tagged_file.into_inner();
	file.rewind().unwrap();

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read_bound()
		.unwrap();

	crate::util::set_artist(
		&mut tagged_file,
		TagType::Mp4Ilst,
		"Bar artist",
		"Foo artist",
		1,
	);
}

#[test_log::test]
fn remove() {
	crate::util::remove_tag_test(
		"tests/files/assets/minimal/m4a_codec_aac.m4a",
		TagType::Mp4Ilst,
	);
}

#[test_log::test]
fn read_no_properties() {
	crate::util::no_properties_test("tests/files/assets/minimal/m4a_codec_aac.m4a");
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/m4a_codec_aac.m4a", None);
}

#[test_log::test]
fn extended_atoms() {
	const ARTIST: &str = "Foo artist";

	fn atom_size(file: &mut File, fourcc: &[u8]) -> u64 {
		file.rewind().unwrap();

		let mut file_content = Vec::new();
		file.read_to_end(&mut file_content).unwrap();

		let atom_pos = file_content
			.windows(4)
			.position(|window| window == fourcc)
			.expect("should have mdat");
		let atom_size =
			u32::from_be_bytes(file_content[atom_pos - 4..atom_pos].try_into().unwrap());
		assert_eq!(atom_size, 1, "`{}` should be 64-bit", fourcc.escape_ascii());

		u64::from_be_bytes(
			file_content[atom_pos + 4..atom_pos + 12]
				.try_into()
				.unwrap(),
		)
	}

	// We should be able to:
	//
	// 1. Parse files with extended atom sizes
	// 2. Write them back

	// `mdat` and `moov` both have extended sizes
	let mut f = File::open("tests/files/assets/issue_746_extended_mdat.m4a").unwrap();
	assert_eq!(atom_size(&mut f, b"mdat"), 22508);
	let original_moov_size = atom_size(&mut f, b"moov");
	assert_eq!(original_moov_size, 986);

	let mut tagged_file = crate::util::read("tests/files/assets/issue_746_extended_mdat.m4a");

	let mut ilst = Tag::new(TagType::Mp4Ilst);
	ilst.set_artist(String::from(ARTIST));
	tagged_file.insert_tag(ilst);

	tagged_file
		.save(WriteOptions::new().preferred_padding(0))
		.unwrap();

	let mut f = tagged_file.into_inner();
	let new_moov_size = atom_size(&mut f, b"moov");
	let ilst_size = 8 /* udta.meta.ilst */ + (8 + 16 + ARTIST.len() as u64) /* ©ART */;

	assert_eq!(
		new_moov_size,
		original_moov_size + 8 /* udta */ + 12 /* udta.meta */ + 33 /* udta.meta.hdlr */ + ilst_size /* udta.meta.ilst */
	);

	// Sanity check that the tag is actually readable...

	f.rewind().unwrap();
	let f = Mp4File::read_from(
		&mut f,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();
	let ilst = f.ilst().unwrap();

	assert_eq!(ilst.artist().as_deref(), Some(ARTIST));
}
