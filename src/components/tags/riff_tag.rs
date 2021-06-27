#![cfg(feature = "format-riff")]

use crate::components::logic::riff;
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture, Result, TagType, ToAny, ToAnyTag,
};
use lofty_attr::impl_tag;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};

struct RiffInnerTag {
	data: Option<HashMap<String, String>>,
}

impl Default for RiffInnerTag {
	fn default() -> Self {
		let data: Option<HashMap<String, String>> = Some(HashMap::new());

		Self { data }
	}
}

#[impl_tag(RiffInnerTag, TagType::RiffInfo)]
pub struct RiffTag;

impl RiffTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: RiffInnerTag {
				data: riff::read_from(reader)?,
			},
		})
	}
}

impl RiffTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		self.inner
			.data
			.as_ref()
			.unwrap()
			.get_key_value(key)
			.and_then(|pair| {
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
		let mut data = self.inner.data.clone().unwrap();
		let _ = data.insert(key.to_string(), val.into());
		self.inner.data = Some(data);
	}

	fn remove_key(&mut self, key: &str) {
		let mut data = self.inner.data.clone().unwrap();
		data.retain(|k, _| k != key);
		self.inner.data = Some(data);
	}
}

impl AudioTagEdit for RiffTag {
	fn title(&self) -> Option<&str> {
		self.get_value("Title")
	}

	fn set_title(&mut self, title: &str) {
		self.set_value("Title", title)
	}

	fn remove_title(&mut self) {
		self.remove_key("Title")
	}

	fn artist_str(&self) -> Option<&str> {
		self.get_value("Artist")
	}

	fn set_artist(&mut self, artist: &str) {
		self.set_value("Artist", artist)
	}

	fn remove_artist(&mut self) {
		self.remove_key("Artist")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self.get_value("Year").map(str::parse::<i32>) {
			Some(y)
		} else {
			None
		}
	}

	fn set_year(&mut self, year: i32) {
		self.set_value("Year", year.to_string())
	}

	fn remove_year(&mut self) {
		self.remove_key("Year")
	}

	fn album_title(&self) -> Option<&str> {
		self.get_value("Album")
	}

	fn set_album_title(&mut self, v: &str) {
		self.set_value("Album", v)
	}

	fn remove_album_title(&mut self) {
		self.remove_key("Album")
	}

	fn album_artist_str(&self) -> Option<&str> {
		self.get_value("AlbumArtist")
	}

	fn set_album_artist(&mut self, artist: &str) {
		self.set_value("AlbumArtist", artist)
	}

	fn remove_album_artists(&mut self) {
		self.remove_key("AlbumArtist")
	}

	/// This will always return `None`, as this is non-standard
	fn front_cover(&self) -> Option<Picture> {
		None
	}

	/// This will not do anything, as this is non-standard
	fn set_front_cover(&mut self, _cover: Picture) {}

	/// This will not do anything, as this is non-standard
	fn remove_front_cover(&mut self) {}

	/// This will always return `None`, as this is non-standard
	fn back_cover(&self) -> Option<Picture> {
		None
	}

	/// This will not do anything, as this is non-standard
	fn set_back_cover(&mut self, _cover: Picture) {}

	/// This will not do anything, as this is non-standard
	fn remove_back_cover(&mut self) {}

	/// This will always return `None`, as this is non-standard
	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		None
	}

	fn track_number(&self) -> Option<u32> {
		if let Some(Ok(y)) = self.get_value("TrackNumber").map(str::parse::<u32>) {
			Some(y)
		} else {
			None
		}
	}

	fn set_track_number(&mut self, track_number: u32) {
		self.set_value("TrackNumber", track_number.to_string())
	}

	fn remove_track_number(&mut self) {
		self.remove_key("TrackNumber")
	}

	fn total_tracks(&self) -> Option<u32> {
		if let Some(Ok(tt)) = self.get_value("TrackTotal").map(str::parse::<u32>) {
			Some(tt)
		} else {
			None
		}
	}

	fn set_total_tracks(&mut self, total_track: u32) {
		self.set_value("TrackTotal", total_track.to_string())
	}

	fn remove_total_tracks(&mut self) {
		self.remove_key("TrackTotal")
	}

	fn disc_number(&self) -> Option<u32> {
		if let Some(Ok(dn)) = self.get_value("DiscNumber").map(str::parse::<u32>) {
			Some(dn)
		} else {
			None
		}
	}

	fn set_disc_number(&mut self, disc_number: u32) {
		self.set_value("DiscNumber", disc_number.to_string())
	}

	fn remove_disc_number(&mut self) {
		self.remove_key("DiscNumber")
	}

	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(td)) = self.get_value("DiscTotal").map(str::parse::<u32>) {
			Some(td)
		} else {
			None
		}
	}

	fn set_total_discs(&mut self, total_discs: u32) {
		self.set_value("DiscTotal", total_discs.to_string())
	}

	fn remove_total_discs(&mut self) {
		self.remove_key("DiscTotal")
	}
}

impl AudioTagWrite for RiffTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		if let Some(data) = self.inner.data.clone() {
			riff::write_to(file, data)?;
		}

		Ok(())
	}
}
