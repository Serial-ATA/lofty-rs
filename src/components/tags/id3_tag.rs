#![cfg(feature = "format-id3")]

use crate::tag::Id3Format;
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, LoftyError, MimeType, Picture,
	PictureType, Result, TagType, ToAny, ToAnyTag,
};
use lofty_attr::impl_tag;

pub use id3::Tag as Id3v2InnerTag;

use filepath::FilePath;
use std::borrow::Cow;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[impl_tag(Id3v2InnerTag, TagType::Id3v2(Id3Format::Default))]
pub struct Id3v2Tag;

impl Id3v2Tag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R, format: &Id3Format) -> Result<Self>
	where
		R: Read + Seek,
	{
		match format {
			Id3Format::Default => Ok(Self {
				inner: Id3v2InnerTag::read_from(reader)?,
			}),
			Id3Format::Riff => Ok(Self {
				inner: Id3v2InnerTag::read_from_wav_reader(reader)?,
			}),
			Id3Format::Form => Ok(Self {
				inner: Id3v2InnerTag::read_from_aiff_reader(reader)?,
			}),
		}
	}
}

impl std::convert::TryFrom<id3::frame::Picture> for Picture {
	type Error = LoftyError;

	fn try_from(inp: id3::frame::Picture) -> Result<Self> {
		let id3::frame::Picture {
			ref mime_type,
			data,
			ref picture_type,
			description,
			..
		} = inp;
		let mime_type: MimeType = mime_type.as_str().try_into()?;
		let pic_type = *picture_type;
		let description = if description == String::new() {
			None
		} else {
			Some(Cow::from(description))
		};

		Ok(Self {
			pic_type,
			mime_type,
			description,
			data: Cow::from(data),
		})
	}
}

impl TryFrom<Picture> for id3::frame::Picture {
	type Error = LoftyError;

	fn try_from(inp: Picture) -> Result<Self> {
		Ok(Self {
			mime_type: String::from(inp.mime_type),
			picture_type: inp.pic_type,
			description: inp
				.description
				.map_or_else(|| "".to_string(), |d| d.to_string()),
			data: Vec::from(inp.data),
		})
	}
}

impl AudioTagEdit for Id3v2Tag {
	fn title(&self) -> Option<&str> {
		self.inner.title()
	}
	fn set_title(&mut self, title: &str) {
		self.inner.set_title(title)
	}
	fn remove_title(&mut self) {
		self.inner.remove_title();
	}

	fn artist_str(&self) -> Option<&str> {
		self.inner.artist()
	}

	fn set_artist(&mut self, artist: &str) {
		self.inner.set_artist(artist)
	}

	fn remove_artist(&mut self) {
		self.inner.remove_artist()
	}

	fn date(&self) -> Option<String> {
		if let Some(released) = self.inner.get("TDRL") {
			if let id3::frame::Content::Text(date) = &released.content() {
				return Some(date.clone());
			}
		}

		if let Some(recorded) = self.inner.get("TRDC") {
			if let id3::frame::Content::Text(date) = &recorded.content() {
				return Some(date.clone());
			}
		}

		None
	}

	fn set_date(&mut self, date: &str) {
		if let Ok(t) = date.parse::<id3::Timestamp>() {
			self.inner.set_date_released(t)
		}
	}

	fn remove_date(&mut self) {
		self.inner.remove_date_released();
		self.inner.remove_date_recorded();
	}

	fn year(&self) -> Option<i32> {
		self.inner.year()
	}
	fn set_year(&mut self, year: i32) {
		self.inner.set_year(year)
	}
	fn remove_year(&mut self) {
		self.inner.remove_year()
	}

	fn album_title(&self) -> Option<&str> {
		self.inner.album()
	}
	fn set_album_title(&mut self, v: &str) {
		self.inner.set_album(v)
	}
	fn remove_album_title(&mut self) {
		self.inner.remove_album();
	}

	fn album_artist_str(&self) -> Option<&str> {
		self.inner.album_artist()
	}

	fn set_album_artist(&mut self, artists: &str) {
		self.inner.set_album_artist(artists)
	}

	fn remove_album_artists(&mut self) {
		self.inner.remove_album_artist()
	}

	fn front_cover(&self) -> Option<Picture> {
		self.inner
			.pictures()
			.find(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
			.and_then(|pic| {
				Some(Picture {
					pic_type: PictureType::CoverFront,
					data: Cow::from(pic.data.clone()),
					mime_type: (pic.mime_type.as_str()).try_into().ok()?,
					description: if pic.description == String::new() {
						None
					} else {
						Some(Cow::from(pic.description.clone()))
					},
				})
			})
	}

	fn set_front_cover(&mut self, cover: Picture) {
		self.remove_front_cover();

		if let Ok(pic) = cover.try_into() {
			self.inner.add_picture(pic)
		}
	}

	fn remove_front_cover(&mut self) {
		self.inner
			.remove_picture_by_type(id3::frame::PictureType::CoverFront);
	}

	fn back_cover(&self) -> Option<Picture> {
		self.inner
			.pictures()
			.find(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverBack))
			.and_then(|pic| {
				Some(Picture {
					pic_type: PictureType::CoverBack,
					data: Cow::from(pic.data.clone()),
					mime_type: (pic.mime_type.as_str()).try_into().ok()?,
					description: if pic.description == String::new() {
						None
					} else {
						Some(Cow::from(pic.description.clone()))
					},
				})
			})
	}

	fn set_back_cover(&mut self, cover: Picture) {
		self.remove_back_cover();

		if let Ok(pic) = cover.try_into() {
			self.inner.add_picture(pic)
		}
	}

	fn remove_back_cover(&mut self) {
		self.inner
			.remove_picture_by_type(id3::frame::PictureType::CoverBack);
	}

	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		let mut pictures = self.inner.pictures().peekable();

		if pictures.peek().is_some() {
			let mut collection = Vec::new();

			for pic in pictures {
				match TryInto::<Picture>::try_into(pic.clone()) {
					Ok(p) => collection.push(p),
					Err(_) => return None,
				}
			}

			return Some(Cow::from(collection));
		}

		None
	}

	fn track_number(&self) -> Option<u32> {
		self.inner.track()
	}
	fn set_track_number(&mut self, track: u32) {
		self.inner.set_track(track);
	}
	fn remove_track_number(&mut self) {
		self.inner.remove_track();
	}

	fn total_tracks(&self) -> Option<u32> {
		self.inner.total_tracks()
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		self.inner.set_total_tracks(total_track as u32);
	}
	fn remove_total_tracks(&mut self) {
		self.inner.remove_total_tracks();
	}

	fn disc_number(&self) -> Option<u32> {
		self.inner.disc()
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.inner.set_disc(disc_number as u32)
	}
	fn remove_disc_number(&mut self) {
		self.inner.remove_disc();
	}

	fn total_discs(&self) -> Option<u32> {
		self.inner.total_discs()
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.inner.set_total_discs(total_discs)
	}
	fn remove_total_discs(&mut self) {
		self.inner.remove_total_discs();
	}
}

impl AudioTagWrite for Id3v2Tag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		let mut id = [0; 4];
		file.read_exact(&mut id)?;
		file.seek(SeekFrom::Start(0))?;

		match &id {
			b"RIFF" => self
				.inner
				.write_to_wav(file.path()?, id3::Version::Id3v24)?,
			b"FORM" => self
				.inner
				.write_to_aiff(file.path()?, id3::Version::Id3v24)?,
			_ => self.inner.write_to(file, id3::Version::Id3v24)?,
		}

		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		let id = &std::fs::read(&path)?[0..4];

		match id {
			b"RIFF" => self.inner.write_to_wav(path, id3::Version::Id3v24)?,
			b"FORM" => self.inner.write_to_aiff(path, id3::Version::Id3v24)?,
			_ => self.inner.write_to_path(path, id3::Version::Id3v24)?,
		}

		Ok(())
	}
}
