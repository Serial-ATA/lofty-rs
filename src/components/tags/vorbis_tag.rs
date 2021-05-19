#![cfg(any(
	feature = "format-vorbis",
	feature = "format-opus",
	feature = "format-flac"
))]

use crate::components::logic::write::vorbis_generic;
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error, Picture, PictureType, Result,
	TagType, ToAny, ToAnyTag, VorbisFormat,
};
use lofty_attr::impl_tag;

use lewton::inside_ogg::OggStreamReader;
use opus_headers::OpusHeaders;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::path::Path;

pub const VORBIS: [u8; 7] = [3, 118, 111, 114, 98, 105, 115];
const OPUSTAGS: [u8; 8] = [79, 112, 117, 115, 84, 97, 103, 115];

struct VorbisInnerTag {
	format: Option<VorbisFormat>,
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

	fn from_path<P>(path: P, format: &VorbisFormat) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		match format {
			VorbisFormat::Ogg => {
				let tag = lewton::inside_ogg::OggStreamReader::new(File::open(path)?)?;
				let vorbis_tag: VorbisTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
			VorbisFormat::Opus => {
				let tag = opus_headers::parse_from_path(path)?;
				let vorbis_tag: VorbisTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
			VorbisFormat::Flac => {
				let tag = metaflac::Tag::read_from_path(path)?;
				let vorbis_tag: VorbisTag = tag.try_into()?;

				Ok(vorbis_tag.inner)
			},
		}
	}
}

#[impl_tag(VorbisInnerTag, TagType::Vorbis(VorbisFormat::Ogg))]
pub struct VorbisTag;

#[cfg(feature = "format-vorbis")]
impl TryFrom<lewton::inside_ogg::OggStreamReader<File>> for VorbisTag {
	type Error = crate::Error;

	fn try_from(inp: OggStreamReader<File>) -> Result<Self> {
		let mut tag = Self::default();

		let mut comments = inp.comment_hdr.comment_list;

		let mut pictures: Vec<Picture> = Vec::new();

		if let Some(p) = comments
			.iter()
			.position(|(k, _)| *k == "METADATA_BLOCK_PICTURE")
		{
			let kv = comments.remove(p);
			if let Ok(pic) = Picture::from_apic_bytes(&kv.1.as_bytes()) {
				pictures.push(pic)
			}
		}

		tag.inner = VorbisInnerTag {
			format: Some(VorbisFormat::Ogg),
			vendor: inp.comment_hdr.vendor,
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

#[cfg(feature = "format-opus")]
impl TryFrom<opus_headers::OpusHeaders> for VorbisTag {
	type Error = crate::Error;

	fn try_from(inp: OpusHeaders) -> Result<Self> {
		let mut tag = Self::default();

		let mut comments = inp.comments.user_comments;

		// TODO: opus_headers doesn't store all keys
		let mut pictures = None;

		if let Some(data) = comments.remove("METADATA_BLOCK_PICTURE") {
			if let Ok(pic) = Picture::from_apic_bytes(&data.as_bytes()) {
				pictures = Some(Cow::from(vec![pic]))
			}
		}

		tag.inner = VorbisInnerTag {
			format: Some(VorbisFormat::Opus),
			vendor: inp.comments.vendor,
			comments,
			pictures,
		};

		Ok(tag)
	}
}

#[cfg(feature = "format-flac")]
impl TryFrom<metaflac::Tag> for VorbisTag {
	type Error = crate::Error;

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
				format: Some(VorbisFormat::Flac),
				vendor: comments.vendor_string,
				comments: comment_collection,
				pictures: Some(Cow::from(pictures)),
			};

			return Ok(tag);
		}

		Err(Error::InvalidData)
	}
}

impl VorbisTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P, format: &VorbisFormat) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self {
			inner: VorbisInnerTag::from_path(path, &format)?,
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
		self.remove_front_cover();

		let pictures = create_cover(&cover, &self.inner.pictures);
		self.inner.pictures = pictures
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
		self.remove_front_cover();

		let pictures = create_cover(&cover, &self.inner.pictures);
		self.inner.pictures = pictures
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

fn create_cover(
	cover: &Picture,
	pictures: &Option<Cow<'static, [Picture]>>,
) -> Option<Cow<'static, [Picture]>> {
	if cover.pic_type == PictureType::CoverFront || cover.pic_type == PictureType::CoverBack {
		if let Ok(pic) = Picture::from_apic_bytes(&cover.as_apic_bytes()) {
			if let Some(pictures) = pictures {
				let mut pictures = pictures.to_vec();
				pictures.retain(|p| p.pic_type != PictureType::CoverBack);

				pictures.push(pic);
				return Some(Cow::from(pictures));
			}

			return Some(Cow::from(vec![pic]));
		}
	}

	None
}

impl AudioTagWrite for VorbisTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		if let Some(format) = self.inner.format.clone() {
			match format {
				VorbisFormat::Ogg => {
					vorbis_generic(file, &VORBIS, &self.inner.vendor, &self.inner.comments)?;
				},
				VorbisFormat::Opus => {
					vorbis_generic(file, &OPUSTAGS, &self.inner.vendor, &self.inner.comments)?;
				},
				VorbisFormat::Flac => {
					crate::components::logic::write::flac(
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
