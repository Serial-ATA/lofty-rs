#![cfg(feature = "mp3")]

use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error,
	MimeType, Picture, Result, TagType, ToAny, ToAnyTag,
};

pub use id3::Tag as Id3v2InnerTag;
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;

impl_tag!(Id3v2Tag, Id3v2InnerTag, TagType::Id3v2);

impl Id3v2Tag {
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self(Id3v2InnerTag::read_from_path(path)?))
	}
}

impl<'a> From<&'a Id3v2Tag> for AnyTag<'a> {
	fn from(inp: &'a Id3v2Tag) -> Self {
		Self {
			title: inp.title(),
			artists: inp.artists_vec(),
			year: inp.year().map(|y| y as i32),
			album: Album::new(
				inp.album_title(),
				inp.album_artists_vec(),
				inp.album_cover(),
			),
			track_number: inp.track_number(),
			total_tracks: inp.total_tracks(),
			disc_number: inp.disc_number(),
			total_discs: inp.total_discs(),
			comments: None,
			date: None, // TODO
			duration_ms: None,
		}
	}
}

impl<'a> From<AnyTag<'a>> for Id3v2Tag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut tag = Id3v2Tag::new();

		if let Some(v) = inp.title() {
			tag.set_title(v)
		}
		if let Some(v) = inp.artists_as_string() {
			tag.set_artist(v.as_str())
		}
		if let Some(v) = inp.year {
			tag.set_year(v)
		}
		if let Some(v) = inp.album().title {
			tag.set_album_title(v)
		}
		if let Some(v) = inp.album().artists {
			tag.set_album_artist(&v.join("/"))
		}
		if let Some(v) = inp.track_number() {
			tag.set_track(v)
		}
		if let Some(v) = inp.total_tracks() {
			tag.set_total_tracks(v)
		}
		if let Some(v) = inp.disc_number() {
			tag.set_disc(v)
		}
		if let Some(v) = inp.total_discs() {
			tag.set_total_discs(v)
		}

		tag
	}
}

impl<'a> std::convert::TryFrom<&'a id3::frame::Picture> for Picture<'a> {
	type Error = Error;
	fn try_from(inp: &'a id3::frame::Picture) -> Result<Self> {
		let &id3::frame::Picture {
			ref mime_type,
			ref data,
			..
		} = inp;
		let mime_type: MimeType = mime_type.as_str().try_into()?;
		Ok(Self {
			data: &data,
			mime_type,
		})
	}
}

impl AudioTagEdit for Id3v2Tag {
	fn title(&self) -> Option<&str> {
		self.0.title()
	}
	fn set_title(&mut self, title: &str) {
		self.0.set_title(title)
	}
	fn remove_title(&mut self) {
		self.0.remove_title();
	}

	fn artist_str(&self) -> Option<&str> {
		self.0.artist()
	}

	fn set_artist(&mut self, artist: &str) {
		self.0.set_artist(artist)
	}

	fn artists_vec(&self) -> Option<Vec<&str>> {
		self.artist_str().map(|a| a.split('/').collect())
	}

	fn remove_artist(&mut self) {
		self.0.remove_artist()
	}

	fn year(&self) -> Option<i32> {
		self.0.year()
	}
	fn set_year(&mut self, year: i32) {
		self.0.set_year(year as i32)
	}
	fn remove_year(&mut self) {
		self.0.remove_year()
	}

	fn album_title(&self) -> Option<&str> {
		self.0.album()
	}
	fn set_album_title(&mut self, v: &str) {
		self.0.set_album(v)
	}
	fn remove_album_title(&mut self) {
		self.0.remove_album();
	}

	fn album_artist_str(&self) -> Option<&str> {
		self.0.album_artist()
	}

	fn album_artists_vec(&self) -> Option<Vec<&str>> {
		self.0.album_artist().map(|a| a.split('/').collect())
	}

	fn set_album_artist(&mut self, artists: &str) {
		self.0.set_album_artist(artists)
	}

	fn remove_album_artists(&mut self) {
		self.0.remove_album_artist()
	}

	fn album_cover(&self) -> Option<Picture> {
		self.0
			.pictures()
			.find(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
			.and_then(|pic| {
				Some(Picture {
					data: &pic.data,
					mime_type: (pic.mime_type.as_str()).try_into().ok()?,
				})
			})
	}
	fn set_album_cover(&mut self, cover: Picture) {
		self.remove_album_cover();
		self.0.add_picture(id3::frame::Picture {
			mime_type: String::from(cover.mime_type),
			picture_type: id3::frame::PictureType::CoverFront,
			description: "".to_owned(),
			data: cover.data.to_owned(),
		});
	}
	fn remove_album_cover(&mut self) {
		self.0
			.remove_picture_by_type(id3::frame::PictureType::CoverFront);
	}

	fn track_number(&self) -> Option<u32> {
		self.0.track()
	}
	fn set_track_number(&mut self, track: u32) {
		self.0.set_track(track);
	}
	fn remove_track_number(&mut self) {
		self.0.remove_track();
	}

	fn total_tracks(&self) -> Option<u32> {
		self.0.total_tracks()
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		self.0.set_total_tracks(total_track as u32);
	}
	fn remove_total_tracks(&mut self) {
		self.0.remove_total_tracks();
	}

	fn disc_number(&self) -> Option<u32> {
		self.0.disc()
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.0.set_disc(disc_number as u32)
	}
	fn remove_disc_number(&mut self) {
		self.0.remove_disc();
	}

	fn total_discs(&self) -> Option<u32> {
		self.0.total_discs()
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.0.set_total_discs(total_discs)
	}
	fn remove_total_discs(&mut self) {
		self.0.remove_total_discs();
	}
}

impl AudioTagWrite for Id3v2Tag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		self.0.write_to(file, id3::Version::Id3v24)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		self.0.write_to_path(path, id3::Version::Id3v24)?;
		Ok(())
	}
}
