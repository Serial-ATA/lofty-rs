//! Tests to verify we can handle growing/shrinking tags.

use crate::util::{named_temp_file, tool_installed};

use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use std::process::Command;

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::flac::FlacFile;
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
		tag.tag_type()
			.remove_from(f.as_file_mut(), WriteOptions::default())
			.unwrap();
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

fn minimal_assets() -> Vec<(FileType, &'static str)> {
	FileType::VARIANTS
		.iter()
		.copied()
		.map(|ty| {
			let asset = match ty {
				FileType::Aac => "tests/files/assets/minimal/full_test.aac",
				FileType::Aiff => "tests/files/assets/minimal/full_test.aiff",
				FileType::Ape => "tests/files/assets/minimal/full_test.ape",
				FileType::Dsf => "tests/files/assets/minimal/full_test.dsf",
				FileType::Flac => "tests/files/assets/minimal/full_test.flac",
				FileType::Mpeg => "tests/files/assets/minimal/full_test.mp3",
				FileType::Mp4 => "tests/files/assets/minimal/m4a_codec_alac.m4a",
				FileType::Mpc => "tests/files/assets/minimal/mpc_sv8.mpc",
				FileType::Opus => "tests/files/assets/minimal/full_test.opus",
				FileType::Vorbis => "tests/files/assets/minimal/full_test.ogg",
				FileType::Speex => "tests/files/assets/minimal/full_test.spx",
				FileType::Wav => "tests/files/assets/minimal/wav_format_pcm.wav",
				FileType::WavPack => "tests/files/assets/minimal/full_test.wv",
				_ => panic!("need an asset for {ty:?}!"),
			};

			(ty, asset)
		})
		.collect()
}

macro_rules! resize_tests {
	($($test_fn:ident => $tag_type:ident),* $(,)?) => {
		const _: () = {
			let specified = [$(TagType::$tag_type),*];
			assert!(
				specified.len() == TagType::VARIANTS.len(),
				"missing tag type(s) in resize test!",
			);
		};

		$(
			#[test_log::test]
			fn $test_fn() {
				for (file_ty, asset) in minimal_assets() {
					if !file_ty.tag_support(TagType::$tag_type).is_writable() {
						continue;
					}

					log::info!("Testing `TagType::{:?}` in `FileType::{:?}`", TagType::$tag_type, file_ty);
					tag_resize_test(asset, TagType::$tag_type);
				}
			}
		)*
	}
}

resize_tests!(
	ape_resize => Ape,
	aiff_resize => AiffText,
	id3v2_resize => Id3v2,
	id3v1_resize => Id3v1,
	ilst_resize => Mp4Ilst,
	riff_info_resize => RiffInfo,
	vorbis_comments_resize => VorbisComments,
);

fn flac_metadata_end(file: &mut File) -> (u64, Vec<usize>) {
	file.rewind().unwrap();
	let mut data = Vec::new();
	file.read_to_end(&mut data).unwrap();
	assert_eq!(&data[..4], b"fLaC");

	let mut offset = 4;
	let mut padding = Vec::new();
	loop {
		let header = &data[offset..offset + 4];
		let is_last = header[0] & 0x80 != 0;
		let block_type = header[0] & 0x7F;
		let content_len =
			usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]);
		if block_type == 1 {
			padding.push(content_len);
		}
		offset += 4 + content_len;
		if is_last {
			break;
		}
	}

	(offset as u64, padding)
}

fn flac_with_large_tag() -> File {
	let mut file = crate::util::temp_file("tests/files/assets/stream_info_last.flac");
	tag(TagType::VorbisComments, 1000)
		.save_to(&mut file, WriteOptions::default().preferred_padding(0))
		.unwrap();
	file.rewind().unwrap();
	file
}

#[test_log::test]
fn flac_preferred_padding_keeps_a_larger_shrink_gap() {
	let mut file = flac_with_large_tag();
	let original_len = file.metadata().unwrap().len();
	let (original_metadata_end, _) = flac_metadata_end(&mut file);
	file.rewind().unwrap();

	tag(TagType::VorbisComments, 1)
		.save_to(&mut file, WriteOptions::default().preferred_padding(16))
		.unwrap();

	let (metadata_end, padding) = flac_metadata_end(&mut file);
	assert_eq!(file.metadata().unwrap().len(), original_len);
	assert_eq!(metadata_end, original_metadata_end);
	assert!(padding.iter().sum::<usize>() > 16);

	file.rewind().unwrap();
	FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
}

#[test_log::test]
fn flac_preferred_padding_grows_when_shrink_gap_is_smaller() {
	let mut file = flac_with_large_tag();
	let original_len = file.metadata().unwrap().len();
	file.rewind().unwrap();

	tag(TagType::VorbisComments, 1)
		.save_to(
			&mut file,
			WriteOptions::default().preferred_padding(100_000),
		)
		.unwrap();

	let (_, padding) = flac_metadata_end(&mut file);
	assert!(file.metadata().unwrap().len() > original_len);
	assert!(padding.iter().sum::<usize>() >= 100_000);

	file.rewind().unwrap();
	FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
}

#[test_log::test]
fn flac_preferred_padding_is_written_when_metadata_grows() {
	let mut file = crate::util::temp_file("tests/files/assets/stream_info_last.flac");
	let original_len = file.metadata().unwrap().len();

	tag(TagType::VorbisComments, 1000)
		.save_to(&mut file, WriteOptions::default().preferred_padding(16))
		.unwrap();

	let (_, padding) = flac_metadata_end(&mut file);
	assert!(file.metadata().unwrap().len() > original_len);
	assert!(padding.iter().sum::<usize>() >= 16);

	file.rewind().unwrap();
	FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
}

#[test_log::test]
fn flac_without_preferred_padding_still_shrinks() {
	let mut file = flac_with_large_tag();
	let original_len = file.metadata().unwrap().len();
	file.rewind().unwrap();

	tag(TagType::VorbisComments, 1)
		.save_to(&mut file, WriteOptions::default().preferred_padding(0))
		.unwrap();

	assert!(file.metadata().unwrap().len() < original_len);
}
