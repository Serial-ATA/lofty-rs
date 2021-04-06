use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error, MimeType, Picture,
	Result, TagType, ToAny, ToAnyTag,
};
use std::{convert::TryInto, fs::File, path::Path};

pub use id3::Tag as Id3v2InnerTag;

impl_tag!(Id3v2Tag, Id3v2InnerTag, TagType::Id3v2);

impl<'a> From<&'a Id3v2Tag> for AnyTag<'a> {
	fn from(inp: &'a Id3v2Tag) -> Self {
		Self {
			title: inp.title(),
			artists: inp.artists(),
			year: inp.year().map(|y| y as i32),
			album: Album::new(inp.album_title(), inp.album_artists(), inp.album_cover()),
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

		inp.title().map(|v| tag.set_title(v));
		inp.artists_as_string().map(|v| tag.set_artist(v.as_str()));
		inp.year.map(|v| tag.set_year(v as u16));
		inp.album().title.map(|v| tag.set_album_title(v));
		inp.album()
			.artists
			.map(|v| tag.set_album_artists(v.join(", ")));
		inp.track_number().map(|v| tag.set_track(v as u16));
		inp.total_tracks().map(|v| tag.set_total_tracks(v as u16));
		inp.disc_number().map(|v| tag.set_disc(v as u16));
		inp.total_discs().map(|v| tag.set_total_discs(v as u16));
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

	fn artist(&self) -> Option<&str> {
		self.0.artist()
	}

	fn set_artist(&mut self, artist: &str) {
		self.0.set_artist(artist)
	}

	fn add_artist(&mut self, artist: &str) {
		if let Some(artists) = self.artist() {
			let mut artists: Vec<&str> = artists.split(", ").collect();
			artists.push(artist);
			self.set_artist(&artists.join(", "))
		} else {
			self.set_artist(artist)
		}
	}

	fn artists(&self) -> Option<Vec<&str>> {
		self.artist().map(|a| a.split(", ").collect())
	}

	fn remove_artist(&mut self) {
		self.0.remove_artist()
	}

	fn year(&self) -> Option<u16> {
		self.0.year().map(|y| y as u16)
	}
	fn set_year(&mut self, year: u16) {
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

	fn album_artists(&self) -> Option<Vec<&str>> {
		self.0.album_artist().map(|a| a.split(", ").collect())
	}

	fn set_album_artists(&mut self, artists: String) {
		self.0.set_album_artist(artists)
	}

	fn add_album_artist(&mut self, artist: &str) {
		todo!()
	}

	fn remove_album_artists(&mut self) {
		self.0.remove_album_artist()
	}

	fn album_cover(&self) -> Option<Picture> {
		self.0
			.pictures()
			.filter(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
			.next()
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

	fn track_number(&self) -> Option<u16> {
		self.0.track().map(|x| x as u16)
	}
	fn set_track_number(&mut self, track: u16) {
		self.0.set_track(track as u32);
	}
	fn remove_track_number(&mut self) {
		self.0.remove_track();
	}

	fn total_tracks(&self) -> Option<u16> {
		self.0.total_tracks().map(|x| x as u16)
	}
	fn set_total_tracks(&mut self, total_track: u16) {
		self.0.set_total_tracks(total_track as u32);
	}
	fn remove_total_tracks(&mut self) {
		self.0.remove_total_tracks();
	}

	fn disc_number(&self) -> Option<u16> {
		self.0.disc().map(|x| x as u16)
	}
	fn set_disc_number(&mut self, disc_number: u16) {
		self.0.set_disc(disc_number as u32)
	}
	fn remove_disc_number(&mut self) {
		self.0.remove_disc();
	}

	fn total_discs(&self) -> Option<u16> {
		self.0.total_discs().map(|x| x as u16)
	}
	fn set_total_discs(&mut self, total_discs: u16) {
		self.0.set_total_discs(total_discs as u32)
	}
	fn remove_total_discs(&mut self) {
		self.0.remove_total_discs();
	}
}

impl AudioTagWrite for Id3v2Tag {
	fn write_to(&mut self, file: &mut File) -> Result<()> {
		self.0.write_to(file, id3::Version::Id3v24)?;
		Ok(())
	}
	fn write_to_path(&mut self, path: &str) -> Result<()> {
		self.0.write_to_path(path, id3::Version::Id3v24)?;
		Ok(())
	}
}
