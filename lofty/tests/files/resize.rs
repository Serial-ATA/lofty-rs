//! Tests to verify we can handle growing/shrinking tags.

use crate::util::{named_temp_file, tool_installed};

use std::io::Seek;
use std::path::Path;
use std::process::Command;

use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::tag::{Accessor, Tag, TagExt, TagType};

fn tag(ty: TagType, size: usize) -> Tag {
	let mut tag = Tag::new(ty);
	tag.set_title("Test".repeat(size));
	tag.set_artist("Serial-ATA".repeat(size));
	tag.set_album("Lofty".repeat(size));
	tag.set_comment("Size test".repeat(size));
	tag
}

/// This does the following:
///
/// 1. Writes an initial large tag
/// 2. Writes a much smaller tag
/// 3. Checks the file with FFmpeg for validity
/// 4. Rewrites the large tag
/// 5. Checks the file again with FFmpeg
fn tag_resize_test(path: &str, tag_type: TagType) {
	fn check_file(path: &Path) {
		let output = Command::new("ffmpeg")
			.arg("-i")
			.arg(path)
			.args(["-f", "null", "-"])
			.output()
			.unwrap();
		if !output.status.success() {
			eprintln!("{}", String::from_utf8_lossy(&output.stderr));
			panic!("ffmpeg exited with error");
		}
	}

	if !tool_installed("ffmpeg") {
		return;
	}

	let mut f = named_temp_file(path);

	// Need to clear out all of the tags. Our test assets use *every* supported tag type, but
	// FFmpeg doesn't like APE files with ID3v2 tags.
	let tagged_file = lofty::read_from(f.as_file_mut()).unwrap();
	f.rewind().unwrap();
	for tag in tagged_file.tags() {
		tag.tag_type().remove_from(f.as_file_mut()).unwrap();
		f.rewind().unwrap();
	}

	let big_tag = tag(tag_type, 1000);
	big_tag
		.save_to(f.as_file_mut(), WriteOptions::default())
		.unwrap();
	check_file(f.path());

	f.rewind().unwrap();
	let shrunk_tag = tag(tag_type, 1);
	shrunk_tag
		.save_to(f.as_file_mut(), WriteOptions::default())
		.unwrap();
	check_file(f.path());

	f.rewind().unwrap();
	big_tag
		.save_to(f.as_file_mut(), WriteOptions::default())
		.unwrap();
	check_file(f.path());
}

#[test_log::test]
fn ape_resize() {
	tag_resize_test("tests/files/assets/minimal/full_test.ape", TagType::Ape);
}

#[test_log::test]
fn aiff_resize() {
	tag_resize_test(
		"tests/files/assets/minimal/full_test.aiff",
		TagType::AiffText,
	);
}

#[test_log::test]
fn id3v2_resize() {
	tag_resize_test("tests/files/assets/minimal/full_test.mp3", TagType::Id3v2);
}

#[test_log::test]
fn id3v1_resize() {
	tag_resize_test("tests/files/assets/minimal/full_test.mp3", TagType::Id3v1);
}

#[test_log::test]
fn ilst_resize() {
	tag_resize_test(
		"tests/files/assets/minimal/m4a_codec_aac.m4a",
		TagType::Mp4Ilst,
	);
}

#[test_log::test]
fn riff_info_resize() {
	tag_resize_test(
		"tests/files/assets/minimal/wav_format_pcm.wav",
		TagType::RiffInfo,
	);
}

#[test_log::test]
fn vorbis_comments_resize() {
	tag_resize_test(
		"tests/files/assets/minimal/full_test.opus",
		TagType::VorbisComments,
	);
}
