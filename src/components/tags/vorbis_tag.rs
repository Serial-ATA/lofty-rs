#![cfg(any(
	feature = "format-vorbis",
	feature = "format-opus",
	feature = "format-flac"
))]

use crate::components::logic::constants::{
	OPUSHEAD, OPUSTAGS, VORBIS_COMMENT_HEAD, VORBIS_IDENT_HEAD,
};
use crate::components::logic::{flac, ogg};
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, LoftyError, OggFormat, Picture,
	PictureType, Result, TagType, ToAny, ToAnyTag,
};

#[cfg(feature = "format-opus")]
use crate::components::logic::ogg::read::OGGTags;

use lofty_attr::impl_tag;

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{Read, Seek};

struct VorbisInnerTag {
	format: Option<OggFormat>,
	vendor: String,
	comments: HashMap<String, String>,
	pictures: Option<Cow<'static, [Picture]>>,
}

impl Default for VorbisInnerTag {
	fn default() -> Self {
		Self {
			format: None,
			vendor: String::new(),
			comments: HashMap::default(),
			pictures: None,
		}
	}
}

impl VorbisInnerTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		self.comments.get_key_value(key).and_then(|pair| {
			if pair.1.is_empty() {
				None
			} else {
				Some(pair.1.as_str())
			}
		})
	}

	fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		let mut comments = self.comments.clone();
		let _ = comments.insert(key.to_string(), val.into());
		self.comments = comments;
	}

	fn remove_key(&mut self, key: &str) {
		let mut comments = self.comments.clone();
		comments.retain(|k, _| k != key);
		self.comments = comments;
	}

	fn read_from<R>(reader: &mut R, format: &OggFormat) -> Result<Self>
	where
		R: Read + Seek,
	{
		match format {
			OggFormat::Vorbis => {
				let tag = ogg::read::read_from(reader, &VORBIS_IDENT_HEAD, &VORBIS_COMMENT_HEAD)?;
				let vorbis_tag: VorbisTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
			OggFormat::Opus => {
				let tag = ogg::read::read_from(reader, &OPUSHEAD, &OPUSTAGS)?;
				let vorbis_tag: VorbisTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
			OggFormat::Flac => {
				let tag = metaflac::Tag::read_from(reader)?;
				let vorbis_tag: VorbisTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
		}
	}
}

#[impl_tag(VorbisInnerTag, TagType::Ogg(OggFormat::Vorbis))]
pub struct VorbisTag;

#[cfg(any(feature = "format-opus", feature = "format-vorbis"))]
impl TryFrom<OGGTags> for VorbisTag {
	type Error = LoftyError;

	fn try_from(inp: OGGTags) -> Result<Self> {
		let mut tag = Self::default();

		let pictures = inp.1;
		let comments = inp.2;

		tag.inner = VorbisInnerTag {
			format: Some(inp.3),
			vendor: inp.0,
			comments: comments.into_iter().collect(),
			pictures: if pictures.is_empty() {
				None
			} else {
				Some(Cow::from(pictures))
			},
		};

		Ok(tag)
	}
}

#[cfg(feature = "format-flac")]
impl TryFrom<metaflac::Tag> for VorbisTag {
	type Error = LoftyError;

	fn try_from(inp: metaflac::Tag) -> Result<Self> {
		let mut tag = Self::default();

		if let Some(comments) = inp.vorbis_comments() {
			let comments = comments.clone();
			let mut user_comments = comments.comments;

			let mut pictures = Vec::new();

			if let Some(data) = user_comments.remove("METADATA_BLOCK_PICTURE") {
				for item in data {
					if let Ok(pic) = Picture::from_apic_bytes(&item.as_bytes()) {
						pictures.push(pic)
					}
				}
			}

			let mut comment_collection = Vec::new();

			for (k, v) in user_comments.clone() {
				for e in v {
					comment_collection.push((k.clone(), e.clone()))
				}
			}

			let comment_collection: HashMap<String, String> =
				comment_collection.into_iter().collect();

			tag.inner = VorbisInnerTag {
				format: Some(OggFormat::Flac),
				vendor: comments.vendor_string,
				comments: comment_collection,
				pictures: Some(Cow::from(pictures)),
			};

			return Ok(tag);
		}

		Err(LoftyError::InvalidData("Flac file contains invalid data"))
	}
}

impl VorbisTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R, format: &OggFormat) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: VorbisInnerTag::read_from(reader, format)?,
			#[cfg(feature = "duration")]
			duration: None,
		})
	}
}

impl AudioTagEdit for VorbisTag {
	fn title(&self) -> Option<&str> {
		self.inner.get_value("TITLE")
	}
	fn set_title(&mut self, title: &str) {
		self.inner.set_value("TITLE", title);
	}

	fn remove_title(&mut self) {
		self.inner.remove_key("TITLE");
	}
	fn artist_str(&self) -> Option<&str> {
		self.inner.get_value("ARTIST")
	}

	fn set_artist(&mut self, artist: &str) {
		self.inner.set_value("ARTIST", artist)
	}

	fn remove_artist(&mut self) {
		self.inner.remove_key("ARTIST");
	}

	fn date(&self) -> Option<String> {
		self.inner
			.get_value("DATE")
			.map(std::string::ToString::to_string)
	}

	fn set_date(&mut self, date: &str) {
		self.inner.set_value("DATE", date)
	}

	fn remove_date(&mut self) {
		self.inner.remove_key("DATE")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self
			.inner
			.get_value("DATE")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y)
		} else if let Some(Ok(y)) = self.inner.get_value("YEAR").map(str::parse::<i32>) {
			Some(y)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: i32) {
		self.inner.set_value("DATE", &year.to_string());
		self.inner.set_value("YEAR", &year.to_string());
	}
	fn remove_year(&mut self) {
		self.inner.remove_key("YEAR");
		self.inner.remove_key("DATE");
	}

	fn album_title(&self) -> Option<&str> {
		self.inner.get_value("ALBUM")
	}
	fn set_album_title(&mut self, title: &str) {
		self.inner.set_value("ALBUM", title)
	}
	fn remove_album_title(&mut self) {
		self.inner.remove_key("ALBUM");
	}

	fn album_artist_str(&self) -> Option<&str> {
		self.inner.get_value("ALBUMARTIST")
	}

	fn album_artists_vec(&self) -> Option<Vec<&str>> {
		self.inner
			.get_value("ALBUMARTIST")
			.map(|a| a.split('/').collect())
	}

	fn set_album_artist(&mut self, artist: &str) {
		self.inner.set_value("ALBUMARTIST", artist)
	}

	fn remove_album_artists(&mut self) {
		self.inner.remove_key("ALBUMARTIST");
	}

	fn front_cover(&self) -> Option<Picture> {
		get_cover(PictureType::CoverFront, &self.inner.pictures)
	}

	fn set_front_cover(&mut self, cover: Picture) {
		if let Some(pic) = create_cover(&cover) {
			self.remove_front_cover();
			self.inner.pictures = Some(replace_pic(pic, &self.inner.pictures))
		}
	}

	fn remove_front_cover(&mut self) {
		if let Some(p) = self.inner.pictures.clone() {
			let mut p = p.to_vec();
			p.retain(|pic| Some(pic) != self.front_cover().as_ref());
			self.inner.pictures = Some(Cow::from(p));
		}
	}

	fn back_cover(&self) -> Option<Picture> {
		get_cover(PictureType::CoverBack, &self.inner.pictures)
	}

	fn set_back_cover(&mut self, cover: Picture) {
		if let Some(pic) = create_cover(&cover) {
			self.remove_back_cover();
			self.inner.pictures = Some(replace_pic(pic, &self.inner.pictures))
		}
	}

	fn remove_back_cover(&mut self) {
		if let Some(p) = self.inner.pictures.clone() {
			let mut p = p.to_vec();
			p.retain(|pic| Some(pic) != self.back_cover().as_ref());
			self.inner.pictures = Some(Cow::from(p));
		}
	}

	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		self.inner.pictures.clone()
	}

	fn track_number(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("TRACKNUMBER").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_track_number(&mut self, v: u32) {
		self.inner.set_value("TRACKNUMBER", &v.to_string())
	}
	fn remove_track_number(&mut self) {
		self.inner.remove_key("TRACKNUMBER");
	}

	// ! not standard
	fn total_tracks(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("TOTALTRACKS").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_tracks(&mut self, v: u32) {
		self.inner.set_value("TOTALTRACKS", &v.to_string())
	}
	fn remove_total_tracks(&mut self) {
		self.inner.remove_key("TOTALTRACKS");
	}

	fn disc_number(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("DISCNUMBER").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_disc_number(&mut self, v: u32) {
		self.inner.set_value("DISCNUMBER", &v.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.inner.remove_key("DISCNUMBER");
	}

	// ! not standard
	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("TOTALDISCS").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_discs(&mut self, v: u32) {
		self.inner.set_value("TOTALDISCS", &v.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.inner.remove_key("TOTALDISCS");
	}
}

fn get_cover(p_type: PictureType, pictures: &Option<Cow<'static, [Picture]>>) -> Option<Picture> {
	match pictures {
		None => None,
		Some(pictures) => {
			for pic in pictures.iter() {
				if pic.pic_type == p_type {
					return Some(pic.clone());
				}
			}

			None
		},
	}
}

fn create_cover(cover: &Picture) -> Option<Picture> {
	if cover.pic_type == PictureType::CoverFront || cover.pic_type == PictureType::CoverBack {
		if let Ok(pic) = Picture::from_apic_bytes(&cover.as_apic_bytes()) {
			return Some(pic);
		}
	}

	None
}

fn replace_pic(
	pic: Picture,
	pictures: &Option<Cow<'static, [Picture]>>,
) -> Cow<'static, [Picture]> {
	if let Some(pictures) = pictures {
		let mut pictures = pictures.to_vec();
		pictures.retain(|p| p.pic_type != pic.pic_type);

		pictures.push(pic);

		Cow::from(pictures)
	} else {
		Cow::from(vec![pic])
	}
}

impl AudioTagWrite for VorbisTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		if let Some(format) = self.inner.format.clone() {
			match format {
				OggFormat::Vorbis => {
					ogg::write::create_pages(
						file,
						&VORBIS_COMMENT_HEAD,
						&self.inner.vendor,
						&self.inner.comments,
						&self.inner.pictures,
					)?;
				},
				OggFormat::Opus => {
					ogg::write::create_pages(
						file,
						&OPUSTAGS,
						&self.inner.vendor,
						&self.inner.comments,
						&self.inner.pictures,
					)?;
				},
				OggFormat::Flac => {
					flac::write_to(
						file,
						&self.inner.vendor,
						&self.inner.comments,
						&self.inner.pictures,
					)?;
				},
			}
		}

		Ok(())
	}
}
