#![cfg(feature = "monkey")]

use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, MimeType, Picture, PictureType,
	Result, TagType, ToAny, ToAnyTag,
};

pub use ape::Tag as ApeInnerTag;

use ape::Item;
use byteorder::{LittleEndian, ReadBytesExt};
use filepath::FilePath;
use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::Path;

#[cfg(feature = "duration")]
use std::time::Duration;

impl_tag!(ApeTag, ApeInnerTag, TagType::Ape);

impl ApeTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self {
			inner: ape::read(&path)?,
			#[cfg(feature = "duration")]
			duration: None, // TODO
		})
	}
}

impl ApeTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		if let Some(item) = self.inner.item(key) {
			if let ape::ItemValue::Text(val) = &item.value {
				return Some(&*val);
			}
		}

		None
	}

	fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		let item = ape::Item {
			key: key.to_string(),
			value: ape::ItemValue::Text(val.into()),
		};

		self.inner.set_item(item)
	}

	fn remove_key(&mut self, key: &str) {
		let _ = self.inner.remove_item(key);
	}
}

impl AudioTagEdit for ApeTag {
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

	fn artists_vec(&self) -> Option<Vec<&str>> {
		self.artist_str().map(|a| a.split('/').collect())
	}

	fn remove_artist(&mut self) {
		self.remove_key("Artist")
	}

	fn date(&self) -> Option<String> {
		self.get_value("Date").map(std::string::ToString::to_string)
	}

	fn set_date(&mut self, date: &str) {
		self.set_value("Date", date)
	}

	fn remove_date(&mut self) {
		self.remove_key("Date")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self
			.get_value("Date")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y)
		} else if let Some(Ok(y)) = self.get_value("Year").map(str::parse::<i32>) {
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

	// Album artists aren't standard?
	fn album_artist_str(&self) -> Option<&str> {
		self.get_value("Album artist")
	}
	fn album_artists_vec(&self) -> Option<Vec<&str>> {
		self.album_artist_str().map(|a| a.split('/').collect())
	}

	fn set_album_artist(&mut self, artists: &str) {
		self.set_value("Album artist", artists)
	}

	fn remove_album_artists(&mut self) {
		self.remove_key("Album artist")
	}

	fn front_cover(&self) -> Option<Picture> {
		if let Some(val) = self.inner.item("Cover Art (Front)") {
			return get_picture(val);
		}

		None
	}

	fn set_front_cover(&mut self, cover: Picture) {
		self.remove_front_cover();

		if let Ok(item) = ape::Item::from_binary("Cover Art (Front)", cover.data) {
			self.inner.set_item(item)
		}
	}

	fn remove_front_cover(&mut self) {
		self.remove_key("Cover Art (Front)")
	}

	fn back_cover(&self) -> Option<Picture> {
		if let Some(val) = self.inner.item("Cover Art (Back)") {
			return get_picture(val);
		}

		None
	}

	fn set_back_cover(&mut self, cover: Picture) {
		self.remove_back_cover();

		if let Ok(item) = ape::Item::from_binary("Cover Art (Back)", cover.data) {
			self.inner.set_item(item)
		}
	}

	fn remove_back_cover(&mut self) {
		self.remove_key("Cover Art (Back)")
	}

	fn pictures(&self) -> Option<Vec<Picture>> {
		// TODO
		None
	}

	// Track number and total tracks are stored together as num/total?
	fn track_number(&self) -> Option<u32> {
		let numbers = self.get_value("Track");

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();
			let track_number = split[0];

			if !track_number.is_empty() {
				if let Ok(num) = track_number.parse::<u32>() {
					return Some(num);
				}
			}
		}

		None
	}
	fn set_track_number(&mut self, track: u32) {
		if let (_, Some(total)) = self.track() {
			let track_str = format!("{}/{}", track, total);
			self.set_value("Track", track_str)
		} else {
			self.set_value("Track", track.to_string())
		}
	}
	fn remove_track_number(&mut self) {
		self.remove_key("Track")
	}

	fn total_tracks(&self) -> Option<u32> {
		let numbers = self.get_value("Track");

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();
			let track_number = split[1];

			if !track_number.is_empty() {
				if let Ok(num) = track_number.parse::<u32>() {
					return Some(num);
				}
			}
		}

		None
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		if let (Some(track_number), _) = self.track() {
			let track_str = format!("{}/{}", track_number, total_track);
			self.set_value("Track", track_str)
		} else {
			self.set_value("Track", format!("0/{}", total_track))
		}
	}
	fn remove_total_tracks(&mut self) {
		if let (Some(track_number), _) = self.track() {
			self.set_value("Track", track_number.to_string())
		} else {
			self.remove_track_number()
		}
	}

	fn disc_number(&self) -> Option<u32> {
		if let Some(disc_num) = self.get_value("Disc") {
			if let Ok(num) = disc_num.parse::<u32>() {
				return Some(num);
			}
		}

		None
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.set_value("Disc", disc_number.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.remove_key("Disc");
	}

	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(num)) = self.get_value("Disc").map(str::parse::<u32>) {
			return Some(num);
		}

		None
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.set_value("Disc", total_discs.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.remove_key("Disc")
	}
}

fn get_picture(item: &Item) -> Option<Picture> {
	if let ape::ItemValue::Binary(bin) = &item.value {
		if !bin.is_empty() {
			let pic_type = match &*item.key {
				"Cover Art (Front)" => PictureType::CoverFront,
				"Cover Art (Back)" => PictureType::CoverBack,
				_ => PictureType::Other,
			};

			let data_pos: Option<usize> =
				if bin.starts_with(&[b'\xff']) || bin.starts_with(&[b'\x89']) {
					Some(0)
				} else {
					bin.iter().find(|x| x == &&b'\0').map(|pos| *pos as usize)
				};

			if let Some(pos) = data_pos {
				let mut cursor = Cursor::new(bin.clone());

				if cursor.seek(SeekFrom::Start((pos + 1) as u64)).is_ok() {
					if let Ok(mime) = cursor.read_u32::<LittleEndian>() {
						if let Some(mime_type) = match &mime.to_le_bytes() {
							b"PNG\0" => Some(MimeType::Png),
							b"JPEG" => Some(MimeType::Jpeg),
							_ => None,
						} {
							cursor.set_position(0_u64);

							return Some(Picture {
								pic_type,
								mime_type,
								data: cursor.into_inner(),
							});
						}
					}
				}
			}
		}
	}

	None
}

impl AudioTagWrite for ApeTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		// Write only uses paths, this is annoying
		ape::write(&self.inner, file.path()?)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		ape::write(&self.inner, path)?;
		Ok(())
	}
}
