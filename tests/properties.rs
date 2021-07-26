use lofty::{FileProperties, Tag};

use std::time::Duration;

const OPUS_PROPERTIES: FileProperties =
	FileProperties::new(Duration::from_millis(1428), Some(120), Some(48000), Some(2));

const VORBIS_PROPERTIES: FileProperties =
	FileProperties::new(Duration::from_millis(1450), Some(112), Some(48000), Some(2));

const FLAC_PROPERTIES: FileProperties = FileProperties::new(
	Duration::from_millis(1428),
	Some(35084),
	Some(48000),
	Some(2),
);

const AIFF_PROPERTIES: FileProperties = FileProperties::new(
	Duration::from_millis(1428),
	Some(1536),
	Some(48000),
	Some(2),
);

macro_rules! properties_test {
	($function:ident, $path:expr, $expected:ident) => {
		#[test]
		fn $function() {
			let tag = Tag::new().read_from_path_signature($path).unwrap();
			let read_properties = tag.properties();

			assert_eq!(read_properties.duration(), $expected.duration());
			assert_eq!(read_properties.sample_rate(), $expected.sample_rate());
			assert_eq!(read_properties.channels(), $expected.channels());
		}
	};
}

properties_test!(test_opus, "tests/assets/a.opus", OPUS_PROPERTIES);
properties_test!(test_vorbis, "tests/assets/a.ogg", VORBIS_PROPERTIES);
properties_test!(test_flac, "tests/assets/a.flac", FLAC_PROPERTIES);
properties_test!(test_aiff_text, "tests/assets/a_text.aiff", AIFF_PROPERTIES);
