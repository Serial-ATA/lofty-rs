use crate::{
	components::logic, impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture,
	Result, TagType, ToAny, ToAnyTag,
};

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::Path;
#[cfg(feature = "duration")]
use std::time::Duration;

struct RiffInnerTag {
	data: Option<HashMap<String, String>>,
}

impl Default for RiffInnerTag {
	fn default() -> Self {
		let data: Option<HashMap<String, String>> = Some(HashMap::new());

		Self { data }
	}
}

impl RiffTag {
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self {
			inner: RiffInnerTag {
				data: logic::read::wav(File::open(path)?)?,
			},
			#[cfg(feature = "duration")]
			duration: None,
		})
	}
}

impl_tag!(RiffTag, RiffInnerTag, TagType::Riff);

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

	fn album_cover(&self) -> Option<Picture> {
		todo!()
	}

	fn set_album_cover(&mut self, _cover: Picture) {
		todo!()
	}

	fn remove_album_cover(&mut self) {
		todo!()
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
			let mut chunk = Vec::new();

			chunk.extend(riff::LIST_ID.value.iter());

			let fourcc = "INFO"; // TODO: ID3
			chunk.extend(fourcc.as_bytes().iter());

			for (k, v) in data {
				if let Some(fcc) = logic::read::key_to_fourcc(&*k) {
					let mut val = v.as_bytes().to_vec();

					if val.len() % 2 != 0 {
						val.push(0)
					}

					let size = val.len() as u32;

					chunk.extend(fcc.iter());
					chunk.extend(size.to_le_bytes().iter());
					chunk.extend(val.iter());
				}
			}

			let mut file_bytes = Vec::new();
			std::io::copy(file.borrow_mut(), &mut file_bytes)?;

			let len = (chunk.len() - 4) as u32;
			let size = len.to_le_bytes();

			#[allow(clippy::needless_range_loop)]
			for i in 0..4 {
				chunk.insert(i + 4, size[i]);
			}

			let data = logic::write::wav(Cursor::new(file_bytes), chunk)?;

			file.seek(SeekFrom::Start(0))?;
			file.set_len(0)?;
			file.write_all(&*data)?;
		}

		Ok(())
	}

	fn write_to_path(&self, path: &str) -> Result<()> {
		self.write_to(&mut OpenOptions::new().read(true).write(true).open(path)?)?;

		Ok(())
	}
}
