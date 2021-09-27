mod util;

use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};

#[test]
fn aiff_read() {
	// Here we have an AIFF file with both an ID3v2 chunk and text chunks
	let file = Probe::new()
		.read_from_path("tests/assets/a_mixed.aiff")
		.unwrap();

	assert_eq!(file.file_type(), &FileType::AIFF);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the text chunks
	crate::verify_artist!(file, tag, TagType::AiffText, "Bar artist", 1);
}

#[test]
fn aiff_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a_mixed.aiff")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::AIFF);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// Text chunks
	crate::set_artist!(tagged_file, tag_mut, TagType::AiffText, "Bar artist", 1 => file, "Baz artist");

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::AiffText, "Baz artist", 1 => file, "Bar artist");
}
