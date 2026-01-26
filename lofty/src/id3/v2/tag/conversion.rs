use crate::TextEncoding;
use crate::config::WriteOptions;
use crate::error::LoftyError;
use crate::id3::v2::frame::MUSICBRAINZ_UFID_OWNER;
use crate::id3::v2::util::pairs::new_number_pair_frame;
use crate::id3::v2::{
	AttachedPictureFrame, CommentFrame, Frame, FrameId, Id3v2TagFlags, KeyValueFrame,
	PopularimeterFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame, write,
};
use crate::io::{FileLike, Length, Truncate};
use crate::prelude::ItemKey;
use crate::tag::companion_tag::CompanionTag;
use crate::tag::{Tag, TagItem, TagType};

use super::V4_MULTI_VALUE_SEPARATOR;
use crate::id3::v2::tag::{
	new_text_frame, new_timestamp_frame, new_url_frame, new_user_text_frame,
};
use crate::id3::v2::util::mappings::TIPL_MAPPINGS;
use crate::mp4::AdvisoryRating;
use crate::tag::items::popularimeter::Popularimeter;
use crate::tag::items::{Lang, Timestamp};
use crate::util::flag_item;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::iter::Peekable;
use std::str::FromStr;

pub(crate) struct Id3v2TagRef<'a, I: Iterator<Item = Frame<'a>> + 'a> {
	pub(crate) flags: Id3v2TagFlags,
	pub(crate) frames: Peekable<I>,
}

impl<'a> Id3v2TagRef<'a, std::iter::Empty<Frame<'a>>> {
	pub(crate) fn empty() -> Self {
		Self {
			flags: Id3v2TagFlags::default(),
			frames: std::iter::empty().peekable(),
		}
	}
}

/// Converts [`TagItem`]s to [`Frame`]s
///
/// This is used in both the `MergeTag` impl for `Id3v2Tag`, as well as the by-ref conversion used in
/// `Tag::save_to()`.
///
/// In the by-ref case, this will only allocate if absolutely necessary (joining text frames, etc.).
/// Otherwise, the returned [`Frame`]s will be tied to the data in the source `Tag`.
///
/// For owned values (in the `MergeTag` case), this will just move the data out of the [`TagItem`]s
/// without cloning.
pub(crate) fn from_tag<'a>(
	items: impl ExactSizeIterator<Item = Cow<'a, TagItem>>,
) -> impl IntoIterator<Item = Frame<'a>> {
	fn item_to_frame<'a>(
		ctx: &mut GenericConversionContext<'a>,
		item: Cow<'a, TagItem>,
	) -> Option<Frame<'a>> {
		fn take_item_text_and_description(
			item: Cow<'_, TagItem>,
		) -> Option<(Cow<'_, str>, Cow<'_, str>)> {
			match item {
				Cow::Owned(TagItem {
					item_key,
					description,
					item_value,
					..
				}) => {
					let Some(text) = item_value.into_string() else {
						log::warn!("Expected a text item for {item_key:?}");
						return None;
					};

					Some((Cow::Owned(text), Cow::Owned(description)))
				},
				Cow::Borrowed(TagItem {
					item_key,
					description,
					item_value,
					..
				}) => {
					let Some(text) = item_value.text() else {
						log::warn!("Expected a text item for {item_key:?}");
						return None;
					};

					Some((Cow::Borrowed(text), Cow::Borrowed(description)))
				},
			}
		}

		fn extend_text(current: &mut Cow<'_, str>, value: &str) {
			match current {
				Cow::Owned(v) => {
					v.push(V4_MULTI_VALUE_SEPARATOR);
					v.push_str(value);
				},
				Cow::Borrowed(v) => {
					*current = Cow::Owned((**v).to_string());
					extend_text(current, value);
				},
			}
		}

		fn parse_number(item: &TagItem) -> Option<u32> {
			let text = item.item_value.text()?;
			text.parse().ok()
		}

		let item_key = item.key();
		match item_key {
			// Multi-valued text key-to-frame mappings
			// TODO: Extend this list of item keys as needed or desired
			ItemKey::TrackArtist
			| ItemKey::AlbumArtist
			| ItemKey::TrackTitle
			| ItemKey::AlbumTitle
			| ItemKey::SetSubtitle
			| ItemKey::TrackSubtitle
			| ItemKey::OriginalAlbumTitle
			| ItemKey::OriginalArtist
			| ItemKey::OriginalLyricist
			| ItemKey::ContentGroup
			| ItemKey::AppleId3v2ContentGroup
			| ItemKey::Genre
			| ItemKey::Mood
			| ItemKey::Composer
			| ItemKey::Conductor
			| ItemKey::Writer
			| ItemKey::Lyricist
			| ItemKey::MusicianCredits
			| ItemKey::InternetRadioStationName
			| ItemKey::InternetRadioStationOwner
			| ItemKey::Remixer
			| ItemKey::Work
			| ItemKey::Movement
			| ItemKey::FileOwner
			| ItemKey::CopyrightMessage
			| ItemKey::Language
			| ItemKey::Publisher => {
				let (value, _) = take_item_text_and_description(item)?;

				let frame_id = item_key.map_key(TagType::Id3v2).expect("valid frame id");
				ctx.text_frames
					.entry(frame_id)
					.and_modify(|current| extend_text(current, &value))
					.or_insert(value);

				// Collected at the end
				None
			},

			// Multi-valued TXXX key-to-frame mappings
			ItemKey::TrackArtists
			| ItemKey::Director
			| ItemKey::AcoustId
			| ItemKey::AcoustIdFingerprint
			| ItemKey::CatalogNumber
			| ItemKey::MusicBrainzArtistId
			| ItemKey::MusicBrainzReleaseArtistId
			| ItemKey::MusicBrainzWorkId
			| ItemKey::ReleaseCountry => {
				let (value, _) = take_item_text_and_description(item)?;

				let frame_id = item_key.map_key(TagType::Id3v2).expect("valid frame id");
				ctx.txxx_frames
					.entry(frame_id)
					.and_modify(|current| extend_text(current, &value))
					.or_insert(value);

				// Collected at the end
				None
			},

			// Comment/Unsync text
			ItemKey::Comment | ItemKey::Lyrics => {
				let lang = item.lang;
				let (value, description) = take_item_text_and_description(item)?;

				let map;
				match item_key {
					ItemKey::Comment => map = &mut ctx.comments,
					ItemKey::Lyrics => map = &mut ctx.unsync_text,
					_ => unreachable!(),
				}

				map.entry(LanguageAndDescription { lang, description })
					.and_modify(|current| extend_text(current, &value))
					.or_insert(value);

				// Collected at the end
				None
			},

			// Flag items
			ItemKey::FlagCompilation | ItemKey::FlagPodcast => {
				let text = item.item_value.text()?;
				let flag_value = flag_item(text)?;

				let frame_id = item.key().map_key(TagType::Id3v2).expect("valid frame id");

				Some(new_text_frame(
					FrameId::Valid(Cow::Borrowed(frame_id)),
					Cow::Owned(u8::from(flag_value).to_string()),
				))
			},

			// iTunes advisory rating
			ItemKey::ParentalAdvisory => {
				let advisory_rating = item.item_value.text()?;

				let Ok(rating) = advisory_rating.parse::<u8>() else {
					log::warn!(
						"Parental advisory rating is not a number: {advisory_rating}, discarding"
					);
					return None;
				};

				let Ok(parsed_rating) = AdvisoryRating::try_from(rating) else {
					log::warn!("Parental advisory rating is out of range: {rating}, discarding");
					return None;
				};

				Some(new_user_text_frame(
					Cow::Borrowed("ITUNESADVISORY"),
					Cow::Owned(parsed_rating.as_u8().to_string()),
				))
			},

			// Timestamps
			ItemKey::RecordingDate | ItemKey::OriginalReleaseDate => {
				let (text, _) = take_item_text_and_description(item)?;

				let frame_id = item_key.map_key(TagType::Id3v2).expect("valid frame id");

				let frame;
				match Timestamp::from_str(&text) {
					Ok(timestamp) => {
						frame =
							new_timestamp_frame(FrameId::Valid(Cow::Borrowed(frame_id)), timestamp);
					},
					Err(_) => {
						// We can just preserve it as a text frame
						frame = new_text_frame(FrameId::Valid(Cow::Borrowed(frame_id)), text);
					},
				}

				Some(frame)
			},

			ItemKey::TrackNumber => {
				ctx.track_number = parse_number(&item);
				None
			},
			ItemKey::TrackTotal => {
				ctx.track_total = parse_number(&item);
				None
			},
			ItemKey::DiscNumber => {
				ctx.disc_number = parse_number(&item);
				None
			},
			ItemKey::DiscTotal => {
				ctx.disc_total = parse_number(&item);
				None
			},

			ItemKey::MusicBrainzRecordingId => {
				let (recording_id, _) = take_item_text_and_description(item)?;

				if !recording_id.is_ascii() {
					return None;
				}

				Some(Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame::new(
					MUSICBRAINZ_UFID_OWNER,
					match recording_id {
						Cow::Owned(v) => Cow::Owned(v.into_bytes()),
						Cow::Borrowed(v) => Cow::Borrowed(v.as_bytes()),
					},
				)))
			},

			// POPM
			ItemKey::Popularimeter => {
				let (encoded_popm, _) = take_item_text_and_description(item)?;

				let Ok(popm) = Popularimeter::from_str(&encoded_popm) else {
					log::warn!("Failed to parse popularimeter during tag merge, skipping");
					return None;
				};

				Some(Frame::Popularimeter(PopularimeterFrame::from(popm)))
			},

			// TIPL key-value mappings
			_ if TIPL_MAPPINGS.iter().any(|(k, _)| *k == item_key) => {
				let (_, tipl_key) = TIPL_MAPPINGS.iter().find(|(k, _)| *k == item_key)?;

				let (value, _) = take_item_text_and_description(item)?;
				ctx.tipl
					.key_value_pairs
					.push((Cow::Borrowed(tipl_key), value));

				// TIPL is collected at the end
				None
			},

			// Anything else
			_ => {
				let Ok(id) = FrameId::try_from(item_key) else {
					return None;
				};

				if id.as_str().starts_with('T') {
					let (value, _) = take_item_text_and_description(item)?;
					return Some(new_text_frame(id, value));
				}

				if id.as_str().starts_with('W') {
					let (value, _) = take_item_text_and_description(item)?;
					return Some(new_url_frame(id, value));
				}

				None
			},
		}
	}

	struct GenericConversionContext<'a> {
		tipl: KeyValueFrame<'a>,

		// Mappings for text frames that can have multiple values. The values are `Cow`, since there
		// will only be an allocation if there are multiple occurrences of a single item.
		text_frames: HashMap<&'static str, Cow<'a, str>>,
		txxx_frames: HashMap<&'static str, Cow<'a, str>>,

		// Then frames with languages are special, as they need to be distinguished by both their description
		// and their language.
		//
		// Like normal text frames, there will only be an allocation if there are multiple frames with
		// the same language and description.
		comments: HashMap<LanguageAndDescription<'a>, Cow<'a, str>>,
		unsync_text: HashMap<LanguageAndDescription<'a>, Cow<'a, str>>,

		// Number frames need to be merged
		track_number: Option<u32>,
		track_total: Option<u32>,
		disc_number: Option<u32>,
		disc_total: Option<u32>,
	}

	#[derive(Hash, PartialEq, Eq)]
	struct LanguageAndDescription<'a> {
		lang: Lang,
		description: Cow<'a, str>,
	}

	let mut ctx = GenericConversionContext {
		tipl: KeyValueFrame::new(
			FrameId::Valid(Cow::Borrowed("TIPL")),
			TextEncoding::UTF8,
			Vec::new(),
		),
		text_frames: HashMap::new(),
		txxx_frames: HashMap::new(),
		comments: HashMap::new(),
		unsync_text: HashMap::new(),
		track_number: None,
		track_total: None,
		disc_number: None,
		disc_total: None,
	};

	let mut frames = HashSet::with_capacity(items.len());
	for item in items {
		if let Some(frame) = item_to_frame(&mut ctx, item) {
			frames.insert(frame);
		}
	}

	if !ctx.tipl.key_value_pairs.is_empty() {
		frames.insert(Frame::KeyValue(ctx.tipl));
	}

	for (frame_id, frame_value) in ctx.text_frames {
		frames.insert(new_text_frame(
			FrameId::Valid(Cow::Borrowed(frame_id)),
			frame_value,
		));
	}

	for (frame_id, frame_value) in ctx.txxx_frames {
		frames.insert(new_user_text_frame(Cow::Borrowed(frame_id), frame_value));
	}

	for (LanguageAndDescription { lang, description }, content) in ctx.comments {
		frames.insert(Frame::Comment(CommentFrame::new(
			TextEncoding::UTF8,
			lang,
			description,
			content,
		)));
	}

	for (LanguageAndDescription { lang, description }, content) in ctx.unsync_text {
		frames.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
			TextEncoding::UTF8,
			lang,
			description,
			content,
		)));
	}

	if let Some(track_frame) =
		new_number_pair_frame(super::TRACK_ID, ctx.track_number, ctx.track_total)
	{
		frames.insert(track_frame);
	}

	if let Some(disc_frame) = new_number_pair_frame(super::DISC_ID, ctx.disc_number, ctx.disc_total)
	{
		frames.insert(disc_frame);
	}

	frames
}

// Create an iterator of FrameRef from a Tag's items for Id3v2TagRef::new
pub(crate) fn tag_frames(tag: &Tag) -> impl Iterator<Item = Frame<'_>> {
	#[derive(Clone)]
	enum CompanionTagIter<F, E> {
		Filled(F),
		Empty(E),
	}

	impl<'a, I> Iterator for CompanionTagIter<I, std::iter::Empty<Frame<'_>>>
	where
		I: Iterator<Item = Frame<'a>>,
	{
		type Item = Frame<'a>;

		fn next(&mut self) -> Option<Self::Item> {
			match self {
				CompanionTagIter::Filled(iter) => iter.next(),
				CompanionTagIter::Empty(_) => None,
			}
		}
	}

	fn create_framerefs_for_companion_tag(
		companion: Option<&CompanionTag>,
	) -> impl IntoIterator<Item = Frame<'_>> + Clone {
		match companion {
			Some(CompanionTag::Id3v2(companion)) => {
				CompanionTagIter::Filled(companion.frames.iter().map(Frame::downgrade))
			},
			_ => CompanionTagIter::Empty(std::iter::empty()),
		}
	}

	let items = from_tag(tag.items().map(Cow::Borrowed)).into_iter().chain(
		create_framerefs_for_companion_tag(tag.companion_tag.as_ref()),
	);

	let pictures = tag
		.pictures()
		.iter()
		.map(|p| Frame::Picture(AttachedPictureFrame::new(TextEncoding::UTF8, p)));

	items.chain(pictures)
}

impl<'a, I: Iterator<Item = Frame<'a>> + 'a> Id3v2TagRef<'a, I> {
	pub(crate) fn write_to<F>(
		&mut self,
		file: &mut F,
		write_options: WriteOptions,
	) -> crate::error::Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		write::write_id3v2(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> crate::error::Result<()> {
		let temp = write::create_tag(self, write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}
