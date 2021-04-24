#![cfg(feature = "mp4")]

use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error, MimeType, Picture,
	Result, TagType, ToAny, ToAnyTag,
};

pub use mp4ameta::{FourCC, Tag as Mp4InnerTag};

use std::fs::File;
use std::path::Path;
#[cfg(feature = "duration")]
use std::time::Duration;

impl_tag!(Mp4Tag, Mp4InnerTag, TagType::Mp4);

impl Mp4Tag {
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self {
			inner: Mp4InnerTag::read_from_path(path)?,
			#[cfg(feature = "duration")]
			duration: None,
		})
	}
}

impl std::convert::TryFrom<mp4ameta::Data> for Picture {
	type Error = Error;
	fn try_from(inp: mp4ameta::Data) -> Result<Self> {
		Ok(match inp {
			mp4ameta::Data::Png(data) => Self {
				data,
				mime_type: MimeType::Png,
			},
			mp4ameta::Data::Jpeg(data) => Self {
				data,
				mime_type: MimeType::Jpeg,
			},
			_ => return Err(Error::NotAPicture),
		})
	}
}

impl AudioTagEdit for Mp4Tag {
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
		self.inner.remove_artists();
	}

	fn year(&self) -> Option<i32> {
		self.inner.year().and_then(|x| str::parse(x).ok())
	}
	fn set_year(&mut self, year: i32) {
		self.inner.set_year(year.to_string())
	}

	fn remove_year(&mut self) {
		self.inner.remove_year();
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
		self.inner.remove_album_artists();
	}
	fn album_cover(&self) -> Option<Picture> {
		use mp4ameta::Data::{Jpeg, Png};

		self.inner.artwork().and_then(|data| match data {
			Jpeg(d) => Some(Picture {
				data: d.clone(),
				mime_type: MimeType::Jpeg,
			}),
			Png(d) => Some(Picture {
				data: d.clone(),
				mime_type: MimeType::Png,
			}),
			_ => None,
		})
	}

	fn set_album_cover(&mut self, cover: Picture) {
		self.remove_album_cover();
		self.inner.add_artwork(match cover.mime_type {
			MimeType::Png => mp4ameta::Data::Png(cover.data),
			MimeType::Jpeg => mp4ameta::Data::Jpeg(cover.data),
			_ => panic!("Only png and jpeg are supported in m4a"),
		});
	}
	fn remove_album_cover(&mut self) {
		self.inner.remove_artwork();
	}
	fn remove_track(&mut self) {
		self.inner.remove_track(); // faster than removing separately
	}
	fn track_number(&self) -> Option<u32> {
		self.inner.track_number().map(u32::from)
	}

	fn set_track_number(&mut self, track: u32) {
		self.inner.set_track_number(track as u16);
	}
	fn remove_track_number(&mut self) {
		self.inner.remove_track_number();
	}
	fn total_tracks(&self) -> Option<u32> {
		self.inner.total_tracks().map(u32::from)
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		self.inner.set_total_tracks(total_track as u16);
	}
	fn remove_total_tracks(&mut self) {
		self.inner.remove_total_tracks();
	}
	fn remove_disc(&mut self) {
		self.inner.remove_disc();
	}
	fn disc_number(&self) -> Option<u32> {
		self.inner.disc_number().map(u32::from)
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.inner.set_disc_number(disc_number as u16)
	}
	fn remove_disc_number(&mut self) {
		self.inner.remove_disc_number();
	}
	fn total_discs(&self) -> Option<u32> {
		self.inner.total_discs().map(u32::from)
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.inner.set_total_discs(total_discs as u16)
	}
	fn remove_total_discs(&mut self) {
		self.inner.remove_total_discs();
	}
}

impl AudioTagWrite for Mp4Tag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		self.inner.write_to(&file)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		self.inner.write_to_path(path)?;
		Ok(())
	}
}
