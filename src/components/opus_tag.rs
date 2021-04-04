use crate::{
	impl_tag, traits::MissingImplementations, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite,
	Picture, Result, TagType, ToAny, ToAnyTag,
};
use std::{
	borrow::BorrowMut,
	collections::{hash_map::RandomState, HashMap},
	fs::File,
	path::Path,
};

use opus_headers::{CommentHeader, IdentificationHeader, OpusHeaders as OpusInnerTag};

impl MissingImplementations for OpusInnerTag {
	fn default() -> Self {
		Self {
			id: IdentificationHeader {
				version: 0,
				channel_count: 0,
				pre_skip: 0,
				input_sample_rate: 0,
				output_gain: 0,
				channel_mapping_family: 0,
				channel_mapping_table: None,
			},
			comments: CommentHeader {
				vendor: "".to_string(),
				user_comments: Default::default(),
			},
		}
	}

	fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let headers = opus_headers::parse_from_path(path)?;

		Ok(Self {
			id: headers.id,
			comments: headers.comments,
		})
	}
}

impl_tag!(OpusTag, OpusInnerTag, TagType::Opus);

impl<'a> From<AnyTag<'a>> for OpusTag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut t = OpusTag::default();
		inp.title().map(|v| t.set_title(v));
		inp.artists_as_string().map(|v| t.set_artist(&v));
		inp.year.map(|v| t.set_year(v as u16));
		inp.album().title.map(|v| t.set_album_title(v));
		inp.album()
			.artists
			.map(|v| t.set_album_artists(v.join(", ")));
		inp.track_number().map(|v| t.set_track_number(v));
		inp.total_tracks().map(|v| t.set_total_tracks(v));
		inp.disc_number().map(|v| t.set_disc_number(v));
		inp.total_discs().map(|v| t.set_total_discs(v));
		t
	}
}

impl<'a> From<&'a OpusTag> for AnyTag<'a> {
	fn from(inp: &'a OpusTag) -> Self {
		let mut t = Self::default();
		t.title = inp.title();
		t.artists = inp.artists();
		t.year = inp.year().map(|y| y as i32);
		t.album = Album::new(inp.album_title(), inp.album_artists(), inp.album_cover());
		t.track_number = inp.track_number();
		t.total_tracks = inp.total_tracks();
		t.disc_number = inp.disc_number();
		t.total_discs = inp.total_discs();
		t
	}
}

impl OpusTag {
	pub fn get_value(&self, key: &str) -> Option<&str> {
		let comments = &self.0.comments.user_comments;

		if let Some(pair) = comments.get_key_value(key) {
			if !pair.1.is_empty() {
				Some(pair.1.as_str())
			} else {
				None
			}
		} else {
			None
		}
	}
	pub fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		let mut comments = self.0.comments.user_comments.clone();
		let _ = comments.insert(key.to_string(), val.into());
		self.0.comments.user_comments = comments;
	}
	pub fn remove(&mut self, key: &str) {
		let mut comments = self.0.comments.user_comments.clone();
		comments.retain(|k, _| k != key);
		self.0.comments.user_comments = comments;
	}
}

impl AudioTagEdit for OpusTag {
	fn title(&self) -> Option<&str> {
		self.get_value("TITLE")
	}
	fn set_title(&mut self, title: &str) {
		self.set_value("TITLE", title);
	}

	fn remove_title(&mut self) {
		self.remove("TITLE");
	}
	fn artist(&self) -> Option<&str> {
		self.get_value("ARTIST")
	}

	fn set_artist(&mut self, artist: &str) {
		self.set_value("ARTIST", artist)
	}

	fn add_artist(&mut self, artist: &str) {
		todo!()
	}

	fn artists(&self) -> Option<Vec<&str>> {
		todo!()
	}

	fn remove_artist(&mut self) {
		self.remove("ARTIST");
	}

	fn year(&self) -> Option<u16> {
		if let Some(Ok(y)) = self
			.get_value("DATE")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y as u16)
		} else if let Some(Ok(y)) = self.get_value("YEAR").map(|s| s.parse::<i32>()) {
			Some(y as u16)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: u16) {
		self.set_value("DATE", &year.to_string());
		self.set_value("YEAR", &year.to_string());
	}
	fn remove_year(&mut self) {
		self.remove("YEAR");
		self.remove("DATE");
	}

	fn album_title(&self) -> Option<&str> {
		self.get_value("ALBUM")
	}
	fn set_album_title(&mut self, title: &str) {
		self.set_value("ALBUM", title)
	}
	fn remove_album_title(&mut self) {
		self.remove("ALBUM");
	}

	fn album_artists(&self) -> Option<Vec<&str>> {
		self.get_value("ALBUMARTIST").map(|a| vec![a])
	}
	fn set_album_artists(&mut self, artists: String) {
		self.set_value("ALBUMARTIST", artists)
	}

	fn add_album_artist(&mut self, artist: &str) {
		todo!()
	}

	fn remove_album_artists(&mut self) {
		self.remove("ALBUMARTIST");
	}

	fn album_cover(&self) -> Option<Picture> {
		// TODO
		// self.0
		//     .pictures()
		//     .filter(|&pic| matches!(pic.picture_type, metaflac::block::PictureType::CoverFront))
		//     .next()
		//     .and_then(|pic| {
		//         Some(Picture {
		//             data: &pic.data,
		//             mime_type: (pic.mime_type.as_str()).try_into().ok()?,
		//         })
		//     })
		None
	}
	fn set_album_cover(&mut self, cover: Picture) {
		// TODO
		// self.remove_album_cover();
		// let mime = String::from(cover.mime_type);
		// let picture_type = metaflac::block::PictureType::CoverFront;
		// self.0
		//     .add_picture(mime, picture_type, (cover.data).to_owned());
	}
	fn remove_album_cover(&mut self) {
		// TODO
		// self.0
		//     .remove_picture_type(metaflac::block::PictureType::CoverFront)
	}

	fn track_number(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("TRACKNUMBER").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_track_number(&mut self, v: u16) {
		self.set_value("TRACKNUMBER", &v.to_string())
	}
	fn remove_track_number(&mut self) {
		self.remove("TRACKNUMBER");
	}

	// ! not standard
	fn total_tracks(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("TOTALTRACKS").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_tracks(&mut self, v: u16) {
		self.set_value("TOTALTRACKS", &v.to_string())
	}
	fn remove_total_tracks(&mut self) {
		self.remove("TOTALTRACKS");
	}

	fn disc_number(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("DISCNUMBER").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_disc_number(&mut self, v: u16) {
		self.set_value("DISCNUMBER", &v.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.remove("DISCNUMBER");
	}

	// ! not standard
	fn total_discs(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("TOTALDISCS").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_discs(&mut self, v: u16) {
		self.set_value("TOTALDISCS", &v.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.remove("TOTALDISCS");
	}
}

impl AudioTagWrite for OpusTag {
	fn write_to(&mut self, file: &mut File) -> Result<()> {
		// self.0.write_to(file)?; TODO
		Ok(())
	}
	fn write_to_path(&mut self, path: &str) -> Result<()> {
		// self.0.write_to_path(path)?; TODO
		Ok(())
	}
}
