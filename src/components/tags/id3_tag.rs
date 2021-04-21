#![cfg(feature = "id3")]

use crate::tag::RiffFormat;
use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error, MimeType, Picture,
	Result, TagType, ToAny, ToAnyTag,
};

pub use id3::Tag as Id3v2InnerTag;

use filepath::FilePath;
use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
#[cfg(feature = "duration")]
use std::time::Duration;

impl_tag!(Id3v2Tag, Id3v2InnerTag, TagType::Id3v2);

impl Id3v2Tag {
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P, format: TagType) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		return match format {
			TagType::Id3v2 => Ok(Self {
				inner: Id3v2InnerTag::read_from_path(&path)?,
				#[cfg(feature = "duration")]
				duration: Some(mp3_duration::from_path(&path)?),
			}),
			TagType::Riff(RiffFormat::ID3) => Ok(Self {
				inner: Id3v2InnerTag::read_from_wav(&path)?,
				#[cfg(feature = "duration")]
				duration: None, // TODO
			}),
			_ => unreachable!(),
		};
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

	fn artists_vec(&self) -> Option<Vec<&str>> {
		self.artist_str().map(|a| a.split('/').collect())
	}

	fn remove_artist(&mut self) {
		self.inner.remove_artist()
	}

	fn year(&self) -> Option<i32> {
		self.inner.year()
	}
	fn set_year(&mut self, year: i32) {
		self.inner.set_year(year as i32)
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

	fn album_artists_vec(&self) -> Option<Vec<&str>> {
		self.inner.album_artist().map(|a| a.split('/').collect())
	}

	fn set_album_artist(&mut self, artists: &str) {
		self.inner.set_album_artist(artists)
	}

	fn remove_album_artists(&mut self) {
		self.inner.remove_album_artist()
	}

	fn album_cover(&self) -> Option<Picture> {
		self.inner
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
		self.inner.add_picture(id3::frame::Picture {
			mime_type: String::from(cover.mime_type),
			picture_type: id3::frame::PictureType::CoverFront,
			description: "".to_owned(),
			data: cover.data.to_owned(),
		});
	}
	fn remove_album_cover(&mut self) {
		self.inner
			.remove_picture_by_type(id3::frame::PictureType::CoverFront);
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
		file.read(&mut id)?;
		file.seek(SeekFrom::Start(0))?;

		if &id == b"RIFF" {
			self.inner
				.write_to_wav(file.path()?, id3::Version::Id3v24)?;
		} else {
			self.inner.write_to(file, id3::Version::Id3v24)?;
		}

		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		let id = &std::fs::read(&path)?[0..4];

		if &id == b"RIFF" {
			self.inner.write_to_wav(path, id3::Version::Id3v24)?;
		} else {
			self.inner.write_to_path(path, id3::Version::Id3v24)?;
		}

		Ok(())
	}
}
