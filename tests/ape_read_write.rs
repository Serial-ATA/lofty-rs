mod util;

use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};

#[test]
fn ape_read() {
	// Here we have an APE file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::new().read_from_path("tests/assets/a.ape").unwrap();

	assert_eq!(file.file_type(), &FileType::APE);

	// Verify the APEv2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify ID3v1
	crate::verify_artist!(file, tag, TagType::Id3v1, "Bar artist", 1);

	// TODO
	// Finally, verify ID3v2
	// crate::verify_artist!(file, tag, TagType::Id3v2, "Baz artist", 1);
}

#[test]
fn ape_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a.ape")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::APE);

	// APEv2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// ID3v1
	crate::set_artist!(tagged_file, tag_mut, TagType::Id3v1, "Bar artist", 1 => file, "Baz artist");

	// ID3v2
	// crate::set_artist!(tagged_file, tag_mut, TagType::Id3v2, "Baz artist", 1 => file, "Qux artist");
	// TODO

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::Id3v1, "Baz artist", 1 => file, "Bar artist");

	// crate::set_artist!(tagged_file, tag_mut, TagType::Id3v2, "Qux artist", 1 => file, "Baz artist");
	// TODO
}
