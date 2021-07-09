#[cfg(feature = "format-flac")]
use crate::components::logic::flac;
#[cfg(any(feature = "format-opus", feature = "format-vorbis"))]
use crate::components::logic::ogg;
#[cfg(feature = "format-opus")]
use crate::components::logic::ogg::constants::{OPUSHEAD, OPUSTAGS};
#[cfg(feature = "format-vorbis")]
use crate::components::logic::ogg::constants::{VORBIS_COMMENT_HEAD, VORBIS_IDENT_HEAD};
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, LoftyError, OggFormat, Picture,
	PictureType, Result, TagType, ToAny, ToAnyTag,
};

#[cfg(any(feature = "format-opus", feature = "format-vorbis"))]
use crate::components::logic::ogg::read::OGGTags;

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use lofty_attr::impl_tag;

struct OggInnerTag {
	vendor: String,
	comments: HashMap<String, String>,
	pictures: Option<Cow<'static, [Picture]>>,
}

impl Default for OggInnerTag {
	fn default() -> Self {
		Self {
			vendor: String::new(),
			comments: HashMap::default(),
			pictures: None,
		}
	}
}

impl OggInnerTag {
	fn read_from<R>(reader: &mut R, format: &OggFormat) -> Result<Self>
	where
		R: Read + Seek,
	{
		match format {
			#[cfg(feature = "format-vorbis")]
			OggFormat::Vorbis => {
				let tag = ogg::read::read_from(
					reader,
					&VORBIS_IDENT_HEAD,
					&VORBIS_COMMENT_HEAD,
					OggFormat::Vorbis,
				)?;
				let vorbis_tag: OggTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
			#[cfg(feature = "format-opus")]
			OggFormat::Opus => {
				let tag = ogg::read::read_from(reader, &OPUSHEAD, &OPUSTAGS, OggFormat::Opus)?;
				let vorbis_tag: OggTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
			#[cfg(feature = "format-flac")]
			OggFormat::Flac => {
				let tag = metaflac::Tag::read_from(reader)?;
				let vorbis_tag: OggTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
		}
	}

	fn get_value(&self, key: &str) -> Option<&str> {
		self.comments.get_key_value(key).map(|(_, v)| v.as_str())
	}

	fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		self.comments.insert(key.to_string(), val.into());
	}

	fn remove_key(&mut self, key: &str) {
		self.comments.remove(key);
	}
}

cfg_if::cfg_if! {
	if #[cfg(feature = "format-opus")] {
		#[impl_tag(OggInnerTag, TagType::Ogg(OggFormat::Opus))]
		pub struct OggTag;
	} else if #[cfg(feature = "format-vorbis")] {
		#[impl_tag(OggInnerTag, TagType::Ogg(OggFormat::Vorbis))]
		pub struct OggTag;
	} else {
		#[impl_tag(OggInnerTag, TagType::Ogg(OggFormat::Flac))]
		pub struct OggTag;
	}
}

#[cfg(any(feature = "format-opus", feature = "format-vorbis"))]
impl TryFrom<OGGTags> for OggTag {
	type Error = LoftyError;

	fn try_from(inp: OGGTags) -> Result<Self> {
		let mut tag = Self::default();

		let vendor = inp.0;
		let pictures = inp.1;
		let comments = inp.2;

		tag.inner = OggInnerTag {
			vendor,
			comments,
			pictures: (!pictures.is_empty()).then(|| Cow::from(pictures)),
		};

		Ok(tag)
	}
}

#[cfg(feature = "format-flac")]
impl TryFrom<metaflac::Tag> for OggTag {
	type Error = LoftyError;

	fn try_from(inp: metaflac::Tag) -> Result<Self> {
		let mut tag = Self::default();

		if let Some(comments) = inp.vorbis_comments() {
			let mut user_comments = comments.comments.clone();

			let mut pictures = Vec::new();

			if let Some(pics) = user_comments.remove("METADATA_BLOCK_PICTURE") {
				pics.iter().for_each(|item| {
					if let Ok(pic) = Picture::from_apic_bytes(item.as_bytes()) {
						pictures.push(pic)
					}
				})
			}

			let mut comment_collection: HashMap<String, String> = HashMap::new();

			for (k, v) in user_comments.clone() {
				for e in v {
					comment_collection.insert(k.clone(), e.clone());
				}
			}

			tag.inner = OggInnerTag {
				vendor: comments.vendor_string.clone(),
				comments: comment_collection,
				pictures: Some(Cow::from(pictures)),
			};

			return Ok(tag);
		}

		Err(LoftyError::InvalidData(
			"Flac file contains no vorbis comment blocks",
		))
	}
}

impl OggTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R, format: &OggFormat) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: OggInnerTag::read_from(reader, format)?,
		})
	}
}

impl AudioTagEdit for OggTag {
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
		if let Some(Ok(y)) = self.inner.get_value("YEAR").map(str::parse::<i32>) {
			return Some(y);
		} else if let Some(Ok(y)) = self
			.inner
			.get_value("DATE")
			.map(|d| (&d[..4]).parse::<i32>())
		{
			return Some(y);
		}

		None
	}
	fn set_year(&mut self, year: i32) {
		self.inner.set_value("YEAR", &year.to_string());
	}
	fn remove_year(&mut self) {
		self.inner.remove_key("YEAR");
	}

	fn copyright(&self) -> Option<&str> {
		self.inner.get_value("COPYRIGHT")
	}
	fn set_copyright(&mut self, copyright: &str) {
		self.inner.set_value("COPYRIGHT", copyright)
	}
	fn remove_copyright(&mut self) {
		self.inner.remove_key("COPYRIGHT")
	}

	fn genre(&self) -> Option<&str> {
		self.inner.get_value("GENRE")
	}
	fn set_genre(&mut self, genre: &str) {
		self.inner.set_value("GENRE", genre)
	}
	fn remove_genre(&mut self) {
		self.inner.remove_key("GENRE")
	}

	fn lyrics(&self) -> Option<&str> {
		self.inner.get_value("LYRICS")
	}
	fn set_lyrics(&mut self, lyrics: &str) {
		self.inner.set_value("LYRICS", lyrics)
	}
	fn remove_lyrics(&mut self) {
		self.inner.remove_key("LYRICS")
	}

	fn bpm(&self) -> Option<&str> {
		self.inner.get_value("BPM")
	}
	fn set_bpm(&mut self, bpm: &str) {
		self.inner.set_value("BPM", bpm)
	}
	fn remove_bpm(&mut self) {
		self.inner.remove_key("BPM")
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

	fn set_album_artist(&mut self, artist: &str) {
		self.inner.set_value("ALBUMARTIST", artist)
	}

	fn remove_album_artists(&mut self) {
		self.inner.remove_key("ALBUMARTIST");
	}

	fn front_cover(&self) -> Option<Picture> {
		if let Some(p) = &self.inner.pictures {
			return p
				.iter()
				.find(|c| c.pic_type == PictureType::CoverFront)
				.cloned();
		}

		None
	}

	#[allow(clippy::collapsible_if)]
	fn set_front_cover(&mut self, cover: Picture) {
		if PictureType::CoverFront == cover.pic_type {
			if let Ok(pic) = Picture::from_apic_bytes(&cover.as_apic_bytes()) {
				self.remove_front_cover();

				if let Some(p) = self.inner.pictures.as_mut().map(std::borrow::Cow::to_mut) {
					p.push(pic)
				} else {
					self.inner.pictures = Some(Cow::from(vec![pic]))
				}
			}
		}
	}

	fn remove_front_cover(&mut self) {
		if let Some(p) = self.inner.pictures.as_mut().map(std::borrow::Cow::to_mut) {
			p.retain(|pic| pic.pic_type != PictureType::CoverFront)
		}
	}

	fn back_cover(&self) -> Option<Picture> {
		if let Some(p) = &self.inner.pictures {
			return p
				.iter()
				.find(|c| c.pic_type == PictureType::CoverBack)
				.cloned();
		}

		None
	}

	#[allow(clippy::collapsible_if)]
	fn set_back_cover(&mut self, cover: Picture) {
		if PictureType::CoverBack == cover.pic_type {
			if let Ok(pic) = Picture::from_apic_bytes(&cover.as_apic_bytes()) {
				self.remove_back_cover();

				if let Some(p) = self.inner.pictures.as_mut().map(std::borrow::Cow::to_mut) {
					p.push(pic)
				} else {
					self.inner.pictures = Some(Cow::from(vec![pic]))
				}
			}
		}
	}

	fn remove_back_cover(&mut self) {
		if let Some(p) = self.inner.pictures.as_mut().map(std::borrow::Cow::to_mut) {
			p.retain(|pic| pic.pic_type != PictureType::CoverBack)
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

impl AudioTagWrite for OggTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		let mut sig = [0; 4];
		file.read_exact(&mut sig)?;
		file.seek(SeekFrom::Start(0))?;

		#[cfg(feature = "format-flac")]
		if &sig == b"fLaC" {
			return flac::write_to(
				file,
				&self.inner.vendor,
				&self.inner.comments,
				&self.inner.pictures,
			);
		}

		#[cfg(any(feature = "format-opus", feature = "format-vorbis"))]
		{
			let p = ogg_pager::Page::read(file)?;
			file.seek(SeekFrom::Start(0))?;

			#[cfg(feature = "format-opus")]
			if p.content.starts_with(&OPUSHEAD) {
				return ogg::write::create_pages(
					file,
					&OPUSTAGS,
					&self.inner.vendor,
					&self.inner.comments,
					&self.inner.pictures,
				);
			}

			#[cfg(feature = "format-vorbis")]
			if p.content.starts_with(&VORBIS_IDENT_HEAD) {
				return ogg::write::create_pages(
					file,
					&VORBIS_COMMENT_HEAD,
					&self.inner.vendor,
					&self.inner.comments,
					&self.inner.pictures,
				);
			}
		}

		Ok(())
	}
}
