use lofty::config::ParsingMode;
use lofty::error::LoftyError;
use lofty::id3::v1::Id3v1Tag;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug, Default)]
struct ExpectedTag {
	description: String,
	title: String,
	artist: String,
	album: String,
	year: String,
	comment: String,
	track: Option<u8>,
	genre: u8,
}

#[derive(Copy, Clone, PartialEq)]
enum Expectation {
	Pass,
	Warning,
	Failure,
}

fn assets_path() -> PathBuf {
	let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	for component in ["tests", "tags", "id3v1", "assets"] {
		dir.push(component);
	}
	dir
}

fn parse_generation_log() -> HashMap<String, ExpectedTag> {
	let log_path = assets_path().join("generation.log");
	let generation_log = std::fs::read_to_string(log_path).unwrap();

	let mut expectations = HashMap::new();

	let pattern = r#"(?sx)
		Generated\s+test\s+file\s+"(?P<file>[^"]+)"\s+
		(?P<description>.*?)
        (?:\n\s*)?Tag\s+structure\s+
        .*?head\s+:\s+"(?P<head>[^"]+)"
        .*?title\s+:\s+"(?P<title>[^"]*)"
        .*?artist\s+:\s+"(?P<artist>[^"]*)"
        .*?album\s+:\s+"(?P<album>[^"]*)"
        .*?year\s+:\s+"(?P<year>[^"]*)"
        .*?comment:\s+"(?P<comment>[^"]*)"
        (?:\s+track\s+:\s+(?P<track>\d+))?
        .*?genre\s+:\s+(?P<genre_id>\d+)
    "#;

	let re = Regex::new(pattern).unwrap();

	for caps in re.captures_iter(&generation_log) {
		let filename = caps["file"].to_string();

		let tag = ExpectedTag {
			description: caps["description"].to_string(),
			title: caps["title"].to_string(),
			artist: caps["artist"].to_string(),
			album: caps["album"].to_string(),
			year: caps["year"].to_string(),
			comment: caps["comment"].to_string(),
			track: caps.name("track").and_then(|m| m.as_str().parse().ok()),
			genre: caps["genre_id"].parse().unwrap_or(0),
		};

		expectations.insert(filename, tag);
	}

	expectations
}

#[test_log::test]
fn test_id3v1_suite() {
	let expectations = parse_generation_log();

	for entry in std::fs::read_dir(&assets_path()).unwrap() {
		let entry = entry.unwrap();
		let path = entry.path();
		if path.extension().and_then(OsStr::to_str) != Some("mp3") {
			continue;
		}

		let filename = path.file_name().unwrap().to_str().unwrap();

		let Some(expected_data) = expectations.get(filename) else {
			panic!("No entry found for file: {filename}");
		};
		println!("{filename}: {}", expected_data.description);

		let expectation = if filename.ends_with("_F.mp3") {
			Expectation::Failure
		} else if filename.ends_with("_W.mp3") {
			Expectation::Warning
		} else {
			Expectation::Pass
		};

		// The genre warning/error tests aren't useful. They're based on the assumption that genres >80 are
		// unknown, but the Winamp extensions (defining up to genre index 191) have been widely accepted.
		if filename.contains("genre")
			&& (expectation == Expectation::Warning || expectation == Expectation::Failure)
		{
			println!("Skipping '{filename}'...");
			continue;
		}

		// This tests UTF-8 strings. While not standardized, most popular apps/libraries use Latin-1.
		if filename == "id3v1_272_extra.mp3" {
			println!("Skipping '{filename}'...");
			continue;
		}

		let mut tag_bytes = [0; 128];
		let mut f = File::open(&path).unwrap();
		f.seek(SeekFrom::End(-128)).unwrap();
		f.read_exact(&mut tag_bytes).unwrap();

		let res: Result<_, LoftyError> = Id3v1Tag::parse(tag_bytes, ParsingMode::Strict);

		match expectation {
			Expectation::Failure => {
				assert!(
					res.is_err(),
					"Expected failure for '{filename}': {}",
					expected_data.description
				);
			},
			_ => {
				// Some tests have null bytes in the strings
				macro_rules! cmp_text {
					($parsed_tag:ident, $expected_data:ident, $field:ident, $filename:ident) => {
						let normalized_expected =
							$expected_data.$field.split("\\0").next().unwrap();
						let parsed_val = $parsed_tag.$field.as_deref().unwrap_or_default();

						assert_eq!(
							parsed_val,
							normalized_expected,
							"Field '{}' mismatch in file '{}'",
							stringify!($field),
							$filename
						);
					};
				}

				let tag = res.expect("Should have parsed successfully");
				cmp_text!(tag, expected_data, title, filename);
				cmp_text!(tag, expected_data, artist, filename);
				cmp_text!(tag, expected_data, album, filename);
				assert_eq!(
					tag.year
						.map_or_else(|| String::from("0000"), |y| format!("{y:04}")),
					expected_data.year,
					"File '{filename}' failed"
				);
				cmp_text!(tag, expected_data, comment, filename);
				assert_eq!(
					tag.track_number, expected_data.track,
					"File '{filename}' failed"
				);
				assert_eq!(
					tag.genre.unwrap_or_default(),
					expected_data.genre,
					"File '{filename}' failed"
				);
			},
		}
	}
}
