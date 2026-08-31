use crate::ape::ApeTag;
use crate::config::WriteOptions;
use crate::id3::v1::Id3v1Tag;
use crate::id3::v2::{Frame, Id3v2Tag};
use crate::iff::aiff::AiffTextChunks;
use crate::iff::wav::RiffInfoList;
use crate::mp4::Ilst;
use crate::ogg::tag::VorbisComments;
use crate::picture::{Picture, PictureType};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::items::popularimeter::{Popularimeter, StarRating};
use crate::tag::{Accessor, Tag, TagExt, TagType, try_parse_timestamp};

use crate::tag::items::Timestamp;
use std::collections::HashSet;
use std::io::{Seek, Write};
use std::process::Command;

#[test_log::test]
fn issue_37() {
	let file_contents = std::fs::read("tests/files/assets/issue_37.ogg").unwrap();
	let mut temp_file = tempfile::NamedTempFile::new().unwrap();
	temp_file.write_all(&file_contents).unwrap();
	temp_file.rewind().unwrap();

	let mut tag = Tag::new(TagType::VorbisComments);

	let mut picture =
		Picture::from_reader(&mut &*std::fs::read("tests/files/assets/issue_37.jpg").unwrap())
			.unwrap();
	picture.set_pic_type(PictureType::CoverFront);

	tag.push_picture(picture);
	tag.save_to(temp_file.as_file_mut(), WriteOptions::default())
		.unwrap();

	let cmd_output = Command::new("ffprobe")
		.arg(temp_file.path().to_str().unwrap())
		.output()
		.unwrap();

	assert!(cmd_output.status.success());

	let stdout = String::from_utf8(cmd_output.stdout).unwrap();

	assert!(!stdout.contains("CRC mismatch!"));
	assert!(!stdout.contains("Header processing failed: Invalid data found when processing input"));
}

#[test_log::test]
fn issue_130_huge_picture() {
	// Verify we have opus-tools available, otherwise skip
	match Command::new("opusinfo").output() {
		Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => {
			eprintln!("Skipping test, `opus-tools` is not installed!");
			return;
		},
		Err(e) => panic!("{}", e),
		_ => {},
	}

	let file_contents = std::fs::read("tests/files/assets/minimal/full_test.opus").unwrap();
	let mut temp_file = tempfile::NamedTempFile::new().unwrap();
	temp_file.write_all(&file_contents).unwrap();
	temp_file.rewind().unwrap();

	let mut tag = Tag::new(TagType::VorbisComments);

	// 81KB picture, which is big enough to surpass the maximum page size
	let mut picture =
		Picture::from_reader(&mut &*std::fs::read("tests/files/assets/issue_37.jpg").unwrap())
			.unwrap();
	picture.set_pic_type(PictureType::CoverFront);

	tag.push_picture(picture);
	tag.save_to(temp_file.as_file_mut(), WriteOptions::default())
		.unwrap();

	let cmd_output = Command::new("opusinfo")
		.arg(temp_file.path().to_str().unwrap())
		.output()
		.unwrap();

	let stdout = String::from_utf8(cmd_output.stdout).unwrap();

	assert!(cmd_output.status.success(), "{stdout}");
	assert!(!stdout.contains("WARNING:"));
}

#[test_log::test]
fn should_preserve_empty_title() {
	let mut tag = Tag::new(TagType::Id3v2);
	tag.set_title(String::from("Foo title"));

	assert_eq!(tag.title().as_deref(), Some("Foo title"));

	tag.set_title(String::new());
	assert_eq!(tag.title().as_deref(), Some(""));

	tag.remove_title();
	assert_eq!(tag.title(), None);
}

#[test_log::test]
fn should_not_parse_year_from_less_than_4_digits() {
	assert!(try_parse_timestamp("198").is_none());
	assert!(try_parse_timestamp("19").is_none());
	assert!(try_parse_timestamp("1").is_none());
}

/// Setup a [`Tag`] with *every* [`ItemKey`] variant, including ones unsupported by the given [`TagType`].
fn setup_tag(ty: TagType) -> (Tag, HashSet<ItemKey>) {
	let mut tag = Tag::new(ty);
	for variant in ItemKey::VARIANTS {
		if variant.is_timestamp() {
			tag.push(TagItem::new(
				*variant,
				ItemValue::Text(
					Timestamp {
						year: 2026,
						month: Some(8),
						day: Some(31),
						hour: Some(1),
						minute: Some(2),
						second: Some(3),
					}
					.to_string(),
				),
			));
			continue;
		}

		if variant.is_numeric() || variant.is_flag() {
			tag.push(TagItem::new(*variant, ItemValue::Text("1".to_string())));
			continue;
		}

		if *variant == ItemKey::Popularimeter {
			tag.push(TagItem::new(
				*variant,
				ItemValue::Text(Popularimeter::musicbee(StarRating::Five, 10).to_string()),
			));
			continue;
		}

		if *variant == ItemKey::Genre {
			tag.push(TagItem::new(
				*variant,
				ItemValue::Text("Classical".to_string()),
			));
			continue;
		}

		tag.push(TagItem::new(*variant, ItemValue::Text("foo".to_string())));
	}

	let supported_keys = ItemKey::supported_keys(ty)
		.iter()
		.copied()
		.collect::<HashSet<_>>();

	(tag, supported_keys)
}

/// Verifies that all `ItemKeys` listed in the format maps in `src/tag/item.rs` are *actually* supported
/// by the tag, converting both to and from the concrete type.
fn verify_round_trip<C, ToConcrete, FromConcrete, ConcreteKeys, GenericKeys>(
	tag_type: TagType,
	to_concrete: ToConcrete,
	from_concrete: FromConcrete,
	concrete_keys: ConcreteKeys,
	generic_keys: GenericKeys,
) where
	ToConcrete: FnOnce(Tag) -> C,
	FromConcrete: FnOnce(C) -> Tag,
	ConcreteKeys: FnOnce(&C) -> HashSet<String>,
	GenericKeys: FnOnce(&HashSet<ItemKey>) -> HashSet<ItemKey>,
{
	fn verify_converted_keys<C>(actual: &HashSet<String>, expected: &HashSet<String>) {
		let missing_keys = expected.difference(actual).collect::<Vec<_>>();
		assert!(
			missing_keys.is_empty(),
			"conversion to `{}` dropped supported items!\n\nMissing expected keys: \
			 {missing_keys:?}",
			std::any::type_name::<C>()
		);
	}

	fn verify_item_keys<C>(actual: &HashSet<ItemKey>, expected: &HashSet<ItemKey>) {
		let missing_keys = expected.difference(actual).collect::<Vec<_>>();
		assert!(
			missing_keys.is_empty(),
			"conversion from `{}` dropped supported items!\n\nMissing expected keys: \
			 {missing_keys:?}",
			std::any::type_name::<C>()
		);
	}

	let (tag, supported_keys) = setup_tag(tag_type);
	let concrete = to_concrete(tag);

	let actual_keys = concrete_keys(&concrete);
	let expected_keys = supported_keys
		.iter()
		.filter_map(|key| key.map_key(tag_type).map(String::from))
		.collect::<HashSet<_>>();

	verify_converted_keys::<C>(&actual_keys, &expected_keys);

	let tag = from_concrete(concrete);
	let actual_item_keys = tag.items().map(TagItem::key).collect::<HashSet<_>>();
	let expected_item_keys = generic_keys(&supported_keys);

	verify_item_keys::<C>(&actual_item_keys, &expected_item_keys);
	assert!(tag.items().all(|item| supported_keys.contains(&item.key())));
}

#[test_log::test]
fn aiff_text_supported_items_are_consistent() {
	verify_round_trip(
		TagType::AiffText,
		AiffTextChunks::from,
		Tag::from,
		|aiff| {
			let AiffTextChunks {
				name,
				author,
				copyright,
				annotations,
				comments,
			} = aiff;

			assert!(
				comments.is_none(),
				"only annotations are supported through `Tag`"
			);

			[
				name.as_ref().map(|_| String::from("NAME")),
				author.as_ref().map(|_| String::from("AUTH")),
				copyright.as_ref().map(|_| String::from("(c) ")),
				annotations.as_ref().map(|_| String::from("ANNO")),
			]
			.into_iter()
			.flatten()
			.collect()
		},
		Clone::clone,
	);
}

#[test_log::test]
fn ape_supported_items_are_consistent() {
	verify_round_trip(
		TagType::Ape,
		ApeTag::from,
		Tag::from,
		|ape| ape.into_iter().map(|item| item.key().to_string()).collect(),
		// APE is special, it supports both `RecordingDate` and `Year`, but they map to the same thing.
		// So just filter one of them out.
		|keys| {
			keys.iter()
				.copied()
				.filter(|key| *key != ItemKey::Year)
				.collect()
		},
	);
}

#[test_log::test]
fn id3v1_supported_items_are_consistent() {
	verify_round_trip(
		TagType::Id3v1,
		Id3v1Tag::from,
		Tag::from,
		|id3v1| {
			[
				id3v1.title.as_ref().map(|_| String::from("TrackTitle")),
				id3v1.artist.as_ref().map(|_| String::from("TrackArtist")),
				id3v1.album.as_ref().map(|_| String::from("AlbumTitle")),
				id3v1.year.as_ref().map(|_| String::from("Year")),
				id3v1.comment.as_ref().map(|_| String::from("Comment")),
				id3v1
					.track_number
					.as_ref()
					.map(|_| String::from("TrackNumber")),
				id3v1.genre.as_ref().map(|_| String::from("Genre")),
			]
			.into_iter()
			.flatten()
			.collect()
		},
		|keys| {
			keys.iter()
				.copied()
				// ID3v1 is special, it supports both `RecordingDate` and `Year`, but they map to the same thing.
				// So just filter one of them out.
				.filter(|key| *key != ItemKey::RecordingDate)
				.collect()
		},
	);
}

#[test_log::test]
fn id3v2_supported_items_are_consistent() {
	verify_round_trip(
		TagType::Id3v2,
		Id3v2Tag::from,
		Tag::from,
		|id3v2| {
			id3v2
				.iter()
				.map(|frame| match frame {
					Frame::UserText(frame) => frame.description.to_string(),
					Frame::UserUrl(frame) => frame.description.to_string(),
					_ => frame.id_str().to_string(),
				})
				.collect()
		},
		|keys| {
			// Filter out duplicate keys
			const DUPLICATE_KEYS: &[ItemKey] = &[
				// Same as `Lyricist`
				ItemKey::Writer,
				// Same as `Publisher`
				ItemKey::Label,
				// Same as `EncoderSettings`
				ItemKey::EncoderSoftware,
			];

			keys.iter()
				.copied()
				.filter(|k| !DUPLICATE_KEYS.contains(k))
				.collect()
		},
	);
}

#[test_log::test]
fn ilst_supported_items_are_consistent() {
	verify_round_trip(
		TagType::Mp4Ilst,
		Ilst::from,
		Tag::from,
		|ilst| {
			ilst.atoms
				.iter()
				.map(|atom| {
					let ident = atom.ident().to_string();
					if let Some(ident) = ident.strip_prefix("\\xa9") {
						// `AtomIdent`'s `Display` impl escapes the `©` character, and the `ItemKey` map doesn't
						return format!("©{ident}");
					}

					ident
				})
				.collect::<HashSet<_>>()
		},
		|keys| {
			keys.iter()
				.copied()
				// Ilst supports both `Lyrics` and `UnsyncLyrics`, but they map to the same thing.
				// So just filter one of them out.
				.filter(|key| *key != ItemKey::UnsyncLyrics)
				.collect()
		},
	);
}

#[test_log::test]
fn riff_info_supported_items_are_consistent() {
	verify_round_trip(
		TagType::RiffInfo,
		RiffInfoList::from,
		Tag::from,
		|riff| {
			riff.items
				.iter()
				.map(|(key, _)| String::from(key))
				.collect()
		},
		|keys| keys.iter().copied().collect(),
	);
}

#[test_log::test]
fn vorbis_comments_supported_items_are_consistent() {
	verify_round_trip(
		TagType::VorbisComments,
		VorbisComments::from,
		Tag::from,
		|vorbis| {
			vorbis
				.items()
				.map(|(key, _)| {
					if key.strip_prefix("RATING:").is_some() {
						String::from("RATING") // Hack for ratings, since they're stored as `RATING:<email>`
					} else {
						String::from(key)
					}
				})
				// The encoder is stored as the vendor string
				.chain((vorbis.vendor == "foo").then_some(String::from("ENCODER")))
				.collect()
		},
		|keys| keys.iter().copied().collect(),
	);
}
