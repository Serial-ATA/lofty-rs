//! Conversions to and from generic types
//!
//! NOTE: We can **ONLY** convert `SimpleTags` that come from a target with **NO** uids

use super::{Language, MatroskaTag, SimpleTag, TargetType, TOMBSTONE_SIMPLE_TAG};
use crate::tag::items::Lang;
use crate::tag::{ItemKey, ItemValue, Tag, TagItem, TagType};

use std::collections::HashMap;
use std::sync::LazyLock;

macro_rules! matroska_mapping_tables {
	(
        $($target:ident => [
            $($matroska_key:literal <=> $item_key:ident),* $(,)?
        ]);+ $(;)?
    ) => {
		const _: () = {
			match TargetType::Album {
				$(
					TargetType::$target => {}
				),+
			}
		};

		pub(crate) const SUPPORTED_ITEMKEYS: &[ItemKey] = &[
			$(
				$(
					ItemKey::$item_key,
				)*
			)+
		];

		static MAPPINGS: LazyLock<HashMap<(TargetType, &'static str), ItemKey>> = LazyLock::new(|| {
			let mut m = HashMap::new();
			$(
				$(
					m.insert((TargetType::$target, $matroska_key), ItemKey::$item_key);
				)*
			)+
			m
		});

		static REVERSE_MAPPINGS: LazyLock<HashMap<ItemKey, (TargetType, &'static str)>> = LazyLock::new(|| {
			let mut m = HashMap::new();
			$(
				$(
					m.insert(ItemKey::$item_key, (TargetType::$target, $matroska_key));
				)*
			)+
			m
		});
	};
}

matroska_mapping_tables!(
	Shot => [];
	Scene => [];
	Track => [
		// Organization Information
		"PART_NUMBER"         <=> TrackNumber,

		// Titles
		"TITLE"               <=> TrackTitle,
		"SUBTITLE"            <=> TrackSubtitle,

		// Nested Information
		"SORT_WITH"           <=> TrackTitleSortOrder,

		// Entities
		"ARTIST"              <=> TrackArtist,
		"LYRICS"              <=> Lyrics,
		"COMPOSER"            <=> Composer,
		"ARRANGER"            <=> Arranger,
		"LYRICIST"            <=> Lyricist,
		"CONDUCTOR"           <=> Conductor,
		"DIRECTOR"            <=> Director,
		"PRODUCER"            <=> Producer,
		"ENCODED_BY"          <=> EncodedBy,
		"MIXED_BY"            <=> MixDj,
		"REMIXED_BY"          <=> Remixer,
		"PUBLISHER"           <=> Publisher,
		"LABEL"               <=> Label,

		// Search and Classification
		"GENRE"               <=> Genre,
		"MOOD"                <=> Mood,
		"INITIAL_KEY"         <=> InitialKey,
		"ORIGINAL_MEDIA_TYPE" <=> OriginalMediaType,

		// Technical Information
		"ENCODER"             <=> EncoderSoftware,
		"ENCODER_SETTINGS"    <=> EncoderSettings,
		"BPM"                 <=> Bpm,
		// TODO: ReplayGain? The values are binary in Matroska

		// Identifiers
		"ISRC"                <=> Isrc,
		"BARCODE"             <=> Barcode,
		"CATALOG_NUMBER"      <=> CatalogNumber,
	];
	Part => [];
	Album => [
		// Organization Information
		"TOTAL_PARTS"    <=> TrackTotal,

		// Titles
		"TITLE"          <=> AlbumTitle,

		// Nested Information
		"SORT_WITH"      <=> AlbumTitleSortOrder,

		// Entities
		"ARTIST"         <=> AlbumArtist,

		// Temporal Information
		"DATE_RELEASED"  <=> ReleaseDate,
		"DATE_RECORDED"  <=> RecordingDate,

		// Technical Information
		// TODO: ReplayGain? The values are binary in Matroska

		// Commercial
		"PURCHASE_ITEM"  <=> PaymentUrl,
		"PURCHASE_INFO"  <=> CommercialInformationUrl,
		"PURCHASE_OWNER" <=> FileOwner,

		// Legal
		"COPYRIGHT"      <=> CopyrightMessage,
		"LICENSE"        <=> License,
	];
	Edition => [];
	Collection => [];
);

const TAG_RETAINED: bool = true;
const TAG_CONSUMED: bool = false;

pub(super) fn split_tag(mut matroska_tag: MatroskaTag) -> (MatroskaTag, Tag) {
	let mut tag = Tag::new(TagType::Matroska);

	// TODO: Pictures, can they be handled in a generic way?
	//       - What about the uid and referral?
	//       - It seems like the "standard" way of adding cover art is to name it "cover.{ext}"
	//       - Maybe only support front covers? who knows.

	matroska_tag.tags.retain_mut(|t| {
		let target_type = match &t.target {
			Some(t) if !t.has_uids() => t.target_type,
			// We cannot use any tags bound to uids
			Some(_) => return TAG_RETAINED,
			None => TargetType::default(),
		};

		t.simple_tags
			.retain_mut(|simple_tag| split_simple_tags(target_type, simple_tag, &mut tag));
		if t.simple_tags.is_empty() {
			return TAG_CONSUMED;
		}

		return TAG_RETAINED;
	});

	(matroska_tag, tag)
}

fn split_simple_tags(
	target_type: TargetType,
	simple_tag: &mut SimpleTag<'_>,
	tag: &mut Tag,
) -> bool {
	let lang: Lang;
	let Language::Iso639_2(l) = &simple_tag.language else {
		return TAG_RETAINED;
	};

	// `Lang` doesn't support anything outside of a 3 character ISO-639-2 code.
	if l.len() != 3 {
		return TAG_CONSUMED;
	}

	lang = l.as_bytes().try_into().unwrap(); // Infallible

	let Some(item_key) = MAPPINGS.get(&(target_type, &*simple_tag.name)).cloned() else {
		return TAG_RETAINED;
	};

	if simple_tag.value.is_none() {
		// Ignore empty items, `TagItem` is not made to handle them.
		return TAG_RETAINED;
	}

	let simple_tag = std::mem::replace(simple_tag, TOMBSTONE_SIMPLE_TAG);
	tag.push(TagItem {
		lang,
		description: String::new(),
		item_key,
		item_value: simple_tag.value.unwrap().into(), // Infallible
	});

	return TAG_CONSUMED;
}

pub(super) fn merge_tag(tag: Tag, mut matroska_tag: MatroskaTag) -> MatroskaTag {
	for item in tag.items {
		let Some((simple_tag, target_type)) = simple_tag_for_item(item) else {
			continue;
		};

		let tag = matroska_tag.get_or_insert_tag_for_type(target_type);

		tag.simple_tags.push(simple_tag);
	}

	matroska_tag
}

fn simple_tag_for_item(item: TagItem) -> Option<(SimpleTag<'static>, TargetType)> {
	let TagItem {
		mut lang,
		item_key,
		item_value: ItemValue::Text(text) | ItemValue::Locator(text),
		..
	} = item
	else {
		return None;
	};

	let Some((target_type, simple_tag_name)) = REVERSE_MAPPINGS.get(&item_key) else {
		return None;
	};

	// Matroska uses "und" for unknown languages
	if lang == *b"XXX" {
		lang = *b"und";
	}

	let lang_str = std::str::from_utf8(lang.as_slice()).unwrap_or("und");

	let mut simple_tag = SimpleTag::new(simple_tag_name.to_string(), text);
	simple_tag.language = Language::Iso639_2(lang_str.to_string());

	Some((simple_tag, *target_type))
}
