use crate::TextEncoding;
use crate::error::{Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::frame::{EMPTY_CONTENT_DESCRIPTOR, FrameRef, MUSICBRAINZ_UFID_OWNER};
use crate::id3::v2::tag::{
	new_binary_frame, new_comment_frame, new_text_frame, new_unsync_text_frame, new_url_frame,
	new_user_text_frame, new_user_url_frame,
};
use crate::id3::v2::{
	ExtendedTextFrame, ExtendedUrlFrame, Frame, FrameFlags, FrameId, PopularimeterFrame,
	UniqueFileIdentifierFrame,
};
use crate::macros::err;
use crate::tag::{ItemKey, ItemValue, TagItem, TagType};

use std::borrow::Cow;

fn frame_from_unknown_item(id: FrameId<'_>, item_value: ItemValue) -> Result<Frame<'_>> {
	match item_value {
		ItemValue::Text(text) => Ok(new_text_frame(id, text)),
		ItemValue::Locator(locator) => {
			if TextEncoding::verify_latin1(&locator) {
				Ok(new_url_frame(id, locator))
			} else {
				err!(TextDecode("ID3v2 URL frames must be Latin-1"));
			}
		},
		ItemValue::Binary(binary) => Ok(new_binary_frame(id, binary.clone())),
	}
}

impl From<TagItem> for Option<Frame<'static>> {
	fn from(input: TagItem) -> Self {
		let value;
		if let Ok(id) = input.key().try_into().map(FrameId::into_owned) {
			return frame_from_unknown_item(id, input.item_value).ok();
		}

		match input.item_key.map_key(TagType::Id3v2, true) {
			Some(desc) => match input.item_value {
				ItemValue::Text(text) => {
					value = Frame::UserText(ExtendedTextFrame::new(
						TextEncoding::UTF8,
						String::from(desc),
						text,
					))
				},
				ItemValue::Locator(locator) => {
					value = Frame::UserUrl(ExtendedUrlFrame::new(
						TextEncoding::UTF8,
						String::from(desc),
						locator,
					))
				},
				ItemValue::Binary(_) => return None,
			},
			None => match (input.item_key, input.item_value) {
				(ItemKey::MusicBrainzRecordingId, ItemValue::Text(recording_id)) => {
					if !recording_id.is_ascii() {
						return None;
					}
					let frame = UniqueFileIdentifierFrame::new(
						MUSICBRAINZ_UFID_OWNER.to_owned(),
						recording_id.into_bytes(),
					);
					value = Frame::UniqueFileIdentifier(frame);
				},
				_ => {
					return None;
				},
			},
		}

		Some(value)
	}
}

impl<'a> TryFrom<&'a TagItem> for FrameRef<'a> {
	type Error = LoftyError;

	fn try_from(tag_item: &'a TagItem) -> std::result::Result<Self, Self::Error> {
		let id: crate::error::Result<FrameId<'a>> = tag_item.key().try_into();
		let value: Frame<'_>;
		match id {
			Ok(id) => {
				let id_str = id.as_str();

				match (id_str, tag_item.value()) {
					("COMM", ItemValue::Text(text)) => {
						value = new_comment_frame(text.clone());
					},
					("USLT", ItemValue::Text(text)) => {
						value = new_unsync_text_frame(text.clone());
					},
					("WXXX", ItemValue::Locator(text) | ItemValue::Text(text)) => {
						value = new_user_url_frame(EMPTY_CONTENT_DESCRIPTOR, text.clone());
					},
					(locator_id, ItemValue::Locator(text)) if locator_id.len() > 4 => {
						value = new_user_url_frame(String::from(locator_id), text.clone());
					},
					("TXXX", ItemValue::Text(text)) => {
						value = new_user_text_frame(EMPTY_CONTENT_DESCRIPTOR, text.clone());
					},
					(text_id, ItemValue::Text(text)) if text_id.len() > 4 => {
						value = new_user_text_frame(String::from(text_id), text.clone());
					},
					("POPM", ItemValue::Binary(contents)) => {
						value = Frame::Popularimeter(PopularimeterFrame::parse(
							&mut &contents[..],
							FrameFlags::default(),
						)?);
					},
					(_, item_value) => value = frame_from_unknown_item(id, item_value.clone())?,
				};
			},
			Err(_) => {
				let item_key = tag_item.key();
				let Some(desc) = item_key.map_key(TagType::Id3v2, true) else {
					return Err(Id3v2Error::new(Id3v2ErrorKind::UnsupportedFrameId(
						item_key.clone(),
					))
					.into());
				};

				match tag_item.value() {
					ItemValue::Text(text) => {
						value = new_user_text_frame(String::from(desc), text.clone());
					},
					ItemValue::Locator(locator) => {
						value = new_user_url_frame(String::from(desc), locator.clone());
					},
					_ => {
						return Err(Id3v2Error::new(Id3v2ErrorKind::UnsupportedFrameId(
							item_key.clone(),
						))
						.into());
					},
				}
			},
		}

		Ok(FrameRef(Cow::Owned(value)))
	}
}
