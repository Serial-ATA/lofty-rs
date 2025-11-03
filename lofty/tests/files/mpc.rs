use crate::{set_artist, temp_file, verify_artist};
use lofty::config::ParseOptions;
use lofty::file::{FileType, TaggedFile};
use lofty::musepack::MpcFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

// Marker test so IntelliJ Rust recognizes this as a test module
#[test_log::test]
fn fake() {}

macro_rules! generate_tests {
	($stream_version:ident, $path:literal) => {
		pastey::paste! {
			#[test_log::test]
			fn [<read_ $stream_version>]() {
				// Here we have an MPC file with an ID3v2, ID3v1, and an APEv2 tag
				let file = Probe::open($path)
					.unwrap()
					.options(ParseOptions::new().read_properties(false))
					.read()
					.unwrap();

				assert_eq!(file.file_type(), FileType::Mpc);

				// Verify the APE tag first
				crate::verify_artist!(file, primary_tag, "Foo artist", 1);

				// Now verify ID3v1 (read only)
				crate::verify_artist!(file, tag, TagType::Id3v1, "Bar artist", 1);

				// Finally, verify ID3v2 (read only)
				crate::verify_artist!(file, tag, TagType::Id3v2, "Baz artist", 1);
			}


			#[test_log::test]
			fn [<write_ $stream_version>]() {
				let mut file = temp_file!($path);

				let mut tagged_file = Probe::new(&mut file)
					.options(ParseOptions::new().read_properties(false))
					.guess_file_type()
					.unwrap()
					.read()
					.unwrap();

				assert_eq!(tagged_file.file_type(), FileType::Mpc);

				// APE
				crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

				// Now reread the file
				file.rewind().unwrap();
				let mut tagged_file = Probe::new(&mut file)
					.options(ParseOptions::new().read_properties(false))
					.guess_file_type()
					.unwrap()
					.read()
					.unwrap();

				crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");
			}

			#[test_log::test]
			fn [<remove_id3v2_ $stream_version>]() {
				crate::remove_tag!($path, TagType::Id3v2);
			}

			#[test_log::test]
			fn [<remove_id3v1_ $stream_version>]() {
				crate::remove_tag!($path, TagType::Id3v1);
			}

			#[test_log::test]
			fn [<remove_ape_ $stream_version>]() {
				crate::remove_tag!($path, TagType::Ape);
			}

			#[test_log::test]
			fn [<read_no_properties_ $stream_version>]() {
				crate::no_properties_test!($path);
			}

			#[test_log::test]
			fn [<read_no_tags_ $stream_version>]() {
				crate::no_tag_test!($path);
			}
		}
	};
}

generate_tests!(sv8, "tests/files/assets/minimal/mpc_sv8.mpc");
generate_tests!(sv7, "tests/files/assets/minimal/mpc_sv7.mpc");

// We have to use `MpcFile::read_from` for stream versions <= 6

#[test_log::test]
fn read_sv5() {
	let mut file = temp_file!("tests/files/assets/minimal/mpc_sv5.mpc");

	// Here we have an MPC file with an ID3v2, ID3v1, and an APEv2 tag
	let file: TaggedFile = MpcFile::read_from(&mut file, ParseOptions::new())
		.unwrap()
		.into();

	assert_eq!(file.file_type(), FileType::Mpc);

	// Verify the APE tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify ID3v1 (read only)
	crate::verify_artist!(file, tag, TagType::Id3v1, "Bar artist", 1);

	// Finally, verify ID3v2 (read only)
	crate::verify_artist!(file, tag, TagType::Id3v2, "Baz artist", 1);
}
