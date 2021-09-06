mod util;

use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};

#[test]
fn wav_read() {
	// Here we have a WAV file with both an ID3v2 chunk and a RIFF INFO chunk
	let file = Probe::new()
		.read_from_path("tests/assets/a_mixed.wav")
		.unwrap();

	assert_eq!(file.file_type(), &FileType::WAV);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the RIFF INFO chunk
	crate::verify_artist!(file, tag, TagType::RiffInfo, "Bar artist", 1);
}

#[test]
fn wav_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a_mixed.wav")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::WAV);

	assert!(tagged_file.primary_tag().is_some());
	assert!(tagged_file.tag(&TagType::RiffInfo).is_some());

	// ID3v2
	// TODO
	// crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// RIFF INFO
	crate::set_artist!(tagged_file, tag_mut, TagType::RiffInfo, "Bar artist", 1 => file, "Baz artist");

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	// TODO
	// crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::RiffInfo, "Baz artist", 1 => file, "Bar artist");
}
