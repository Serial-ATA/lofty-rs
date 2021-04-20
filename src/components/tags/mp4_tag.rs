#![cfg(feature = "mp4")]

use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error,
	MimeType, Picture, Result, TagType, ToAny, ToAnyTag,
};

pub use mp4ameta::{FourCC, Tag as Mp4InnerTag};
use std::fs::File;
use std::path::Path;

impl_tag!(Mp4Tag, Mp4InnerTag, TagType::Mp4);

impl Mp4Tag {
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self(Mp4InnerTag::read_from_path(path)?))
	}
}

impl<'a> From<&'a Mp4Tag> for AnyTag<'a> {
	fn from(inp: &'a Mp4Tag) -> Self {
		let title = inp.title();
		let artists = inp.artists_vec().map(|i| i.into_iter().collect::<Vec<_>>());
		let year = inp.year().map(|y| y as i32);
		let album = Album::new(
			inp.album_title(),
			inp.album_artists_vec(),
			inp.album_cover(),
		);
		let (track_number, total_tracks) = inp.track();
		let (disc_number, total_discs) = inp.disc();

		Self {
			title,
			artists,
			year,
			album,
			track_number,
			total_tracks,
			disc_number,
			total_discs,
			comments: None,
			date: None,
			duration_ms: None, // TODO?
		}
	}
}

impl<'a> From<AnyTag<'a>> for Mp4Tag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut tag = Mp4Tag::new();

		if let Some(v) = inp.title() {
			tag.set_title(v)
		}
		if let Some(i) = inp.artists_as_string() {
			tag.set_artist(&*i)
		}
		if let Some(v) = inp.year {
			tag.set_year(v)
		}
		if let Some(v) = inp.album().title {
			tag.set_album_title(v)
		}
		if let Some(i) = inp.album().artists_as_string() {
			tag.set_album_artist(&*i)
		}
		if let Some(v) = inp.track_number() {
			tag.set_track_number(v)
		}
		if let Some(v) = inp.total_tracks() {
			tag.set_total_tracks(v)
		}
		if let Some(v) = inp.disc_number() {
			tag.set_disc_number(v)
		}
		if let Some(v) = inp.total_discs() {
			tag.set_total_discs(v)
		}
		tag
	}
}

impl<'a> std::convert::TryFrom<&'a mp4ameta::Data> for Picture<'a> {
	type Error = Error;
	fn try_from(inp: &'a mp4ameta::Data) -> Result<Self> {
		Ok(match *inp {
			mp4ameta::Data::Png(ref data) => Self {
				data,
				mime_type: MimeType::Png,
			},
			mp4ameta::Data::Jpeg(ref data) => Self {
				data,
				mime_type: MimeType::Jpeg,
			},
			_ => return Err(Error::NotAPicture),
		})
	}
}

impl AudioTagEdit for Mp4Tag {
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

	fn remove_artist(&mut self) {
		self.0.remove_artists();
	}

	fn year(&self) -> Option<i32> {
		self.0.year().and_then(|x| str::parse(x).ok())
	}
	fn set_year(&mut self, year: i32) {
		self.0.set_year(year.to_string())
	}

	fn remove_year(&mut self) {
		self.0.remove_year();
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

	fn set_album_artist(&mut self, artists: &str) {
		self.0.set_album_artist(artists)
	}

	fn remove_album_artists(&mut self) {
		self.0.remove_album_artists();
	}
	fn album_cover(&self) -> Option<Picture> {
		use mp4ameta::Data::{Jpeg, Png};

		self.0.artwork().and_then(|data| match data {
			Jpeg(d) => Some(Picture {
				data: d,
				mime_type: MimeType::Jpeg,
			}),
			Png(d) => Some(Picture {
				data: d,
				mime_type: MimeType::Png,
			}),
			_ => None,
		})
	}

	fn set_album_cover(&mut self, cover: Picture) {
		self.remove_album_cover();
		self.0.add_artwork(match cover.mime_type {
			MimeType::Png => mp4ameta::Data::Png(cover.data.to_owned()),
			MimeType::Jpeg => mp4ameta::Data::Jpeg(cover.data.to_owned()),
			_ => panic!("Only png and jpeg are supported in m4a"),
		});
	}
	fn remove_album_cover(&mut self) {
		self.0.remove_artwork();
	}
	fn remove_track(&mut self) {
		self.0.remove_track(); // faster than removing separately
	}
	fn track_number(&self) -> Option<u32> {
		self.0.track_number().map(u32::from)
	}

	fn set_track_number(&mut self, track: u32) {
		self.0.set_track_number(track as u16);
	}
	fn remove_track_number(&mut self) {
		self.0.remove_track_number();
	}
	fn total_tracks(&self) -> Option<u32> {
		self.0.total_tracks().map(u32::from)
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		self.0.set_total_tracks(total_track as u16);
	}
	fn remove_total_tracks(&mut self) {
		self.0.remove_total_tracks();
	}
	fn remove_disc(&mut self) {
		self.0.remove_disc();
	}
	fn disc_number(&self) -> Option<u32> {
		self.0.disc_number().map(u32::from)
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.0.set_disc_number(disc_number as u16)
	}
	fn remove_disc_number(&mut self) {
		self.0.remove_disc_number();
	}
	fn total_discs(&self) -> Option<u32> {
		self.0.total_discs().map(u32::from)
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.0.set_total_discs(total_discs as u16)
	}
	fn remove_total_discs(&mut self) {
		self.0.remove_total_discs();
	}
}

impl AudioTagWrite for Mp4Tag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		self.0.write_to(&file)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		self.0.write_to_path(path)?;
		Ok(())
	}
}
