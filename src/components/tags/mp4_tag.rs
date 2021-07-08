use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, LoftyError, MimeType, Picture,
	PictureType, Result, TagType, ToAny, ToAnyTag,
};

use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Seek};

use lofty_attr::impl_tag;
pub use mp4ameta::{Fourcc, Tag as Mp4InnerTag};

#[impl_tag(Mp4InnerTag, TagType::Mp4)]
pub struct Mp4Tag {}

impl Mp4Tag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: Mp4InnerTag::read_from(reader)?,
		})
	}
}

impl std::convert::TryFrom<mp4ameta::Data> for Picture {
	type Error = LoftyError;

	fn try_from(inp: mp4ameta::Data) -> Result<Self> {
		Ok(match inp {
			mp4ameta::Data::Png(data) => Self {
				pic_type: PictureType::Other,
				mime_type: MimeType::Png,
				description: None,
				data: Cow::from(data),
			},
			mp4ameta::Data::Jpeg(data) => Self {
				pic_type: PictureType::Other,
				mime_type: MimeType::Jpeg,
				description: None,
				data: Cow::from(data),
			},
			mp4ameta::Data::Bmp(data) => Self {
				pic_type: PictureType::Other,
				mime_type: MimeType::Bmp,
				description: None,
				data: Cow::from(data),
			},
			_ => return Err(LoftyError::NotAPicture),
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

	fn copyright(&self) -> Option<&str> {
		self.inner.copyright()
	}
	fn set_copyright(&mut self, copyright: &str) {
		self.inner.set_copyright(copyright)
	}
	fn remove_copyright(&mut self) {
		self.inner.remove_copyright()
	}

	fn album_title(&self) -> Option<&str> {
		self.inner.album()
	}
	fn set_album_title(&mut self, title: &str) {
		self.inner.set_album(title)
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

	fn front_cover(&self) -> Option<Picture> {
		if let Some(picture) = &self.inner.artwork() {
			return match picture {
				mp4ameta::Data::Jpeg(d) => Some(Picture {
					pic_type: PictureType::Other,
					mime_type: MimeType::Jpeg,
					description: None,
					data: Cow::from(d.clone()),
				}),
				mp4ameta::Data::Png(d) => Some(Picture {
					pic_type: PictureType::Other,
					mime_type: MimeType::Png,
					description: None,
					data: Cow::from(d.clone()),
				}),
				mp4ameta::Data::Bmp(d) => Some(Picture {
					pic_type: PictureType::Other,
					mime_type: MimeType::Bmp,
					description: None,
					data: Cow::from(d.clone()),
				}),
				_ => None,
			};
		}

		None
	}

	fn set_front_cover(&mut self, cover: Picture) {
		self.inner.remove_artwork();

		self.inner.add_artwork(match cover.mime_type {
			MimeType::Png => mp4ameta::Data::Png(Vec::from(cover.data)),
			MimeType::Jpeg => mp4ameta::Data::Jpeg(Vec::from(cover.data)),
			MimeType::Bmp => mp4ameta::Data::Bmp(Vec::from(cover.data)),
			_ => panic!("Attempt to add an invalid image format to MP4"),
		});
	}

	fn remove_front_cover(&mut self) {
		self.inner.remove_artwork();
	}

	fn back_cover(&self) -> Option<Picture> {
		self.front_cover()
	}
	fn set_back_cover(&mut self, cover: Picture) {
		self.set_front_cover(cover)
	}
	fn remove_back_cover(&mut self) {
		self.inner.remove_artwork();
	}

	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		let mut pictures = Vec::new();

		for art in self.inner.artworks() {
			let info = match art {
				mp4ameta::Data::Png(d) => Some((MimeType::Png, d.clone())),
				mp4ameta::Data::Jpeg(d) => Some((MimeType::Jpeg, d.clone())),
				mp4ameta::Data::Bmp(d) => Some((MimeType::Bmp, d.clone())),
				_ => None,
			};

			if let Some((mime_type, data)) = info {
				pictures.push(Picture {
					pic_type: PictureType::Other,
					mime_type,
					description: None,
					data: Cow::from(data),
				})
			}
		}

		if pictures.is_empty() {
			None
		} else {
			Some(Cow::from(pictures))
		}
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
