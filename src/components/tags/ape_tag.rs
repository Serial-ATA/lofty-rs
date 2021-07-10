use crate::types::picture::{PicType, APE_PICTYPES};
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture, Result, TagType, ToAny, ToAnyTag,
};

use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Seek};

use ape::Item;
pub use ape::Tag as ApeInnerTag;
use lofty_attr::{get_set_methods, impl_tag};
use unicase::UniCase;

#[impl_tag(ApeInnerTag, TagType::Ape)]
pub struct ApeTag;

impl ApeTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: ape::read_from(reader)?,
		})
	}
}

impl ApeTag {
	fn get_value(&self, key: UniCase<&str>) -> Option<&str> {
		if let Some(item) = self.inner.item(&key) {
			if let ape::ItemValue::Text(val) = &item.value {
				return Some(&*val);
			}
		}

		None
	}

	#[allow(clippy::unused_self)]
	fn get_picture(&self, item: &Item) -> Option<Picture> {
		if let ape::ItemValue::Binary(bin) = &item.value {
			if let Ok(pic) = Picture::from_ape_bytes(&item.key, bin) {
				return Some(pic);
			}
		}

		None
	}

	fn set_value<V>(&mut self, key: UniCase<&str>, val: V)
	where
		V: Into<String>,
	{
		let item = ape::Item {
			key: key.to_string(),
			value: ape::ItemValue::Text(val.into()),
		};

		self.inner.set_item(item)
	}

	fn remove_key(&mut self, key: UniCase<&str>) {
		let _ = self.inner.remove_item(&key);
	}
}

impl AudioTagEdit for ApeTag {
	get_set_methods!(title, UniCase::new("Title"));
	get_set_methods!(artist, UniCase::new("Artist"));
	get_set_methods!(copyright, UniCase::new("Copyright"));
	get_set_methods!(genre, UniCase::new("Genre"));
	get_set_methods!(lyrics, UniCase::new("Lyrics"));
	get_set_methods!(lyricist, UniCase::new("Lyricist"));
	get_set_methods!(composer, UniCase::new("Composer"));
	get_set_methods!(album_title, UniCase::new("Album"));
	get_set_methods!(encoder, UniCase::new("Encoder"));

	// Album artists aren't standard?
	get_set_methods!(album_artist, UniCase::new("AlbumArtist"));

	fn date(&self) -> Option<String> {
		self.get_value(UniCase::from("Date"))
			.map(std::string::ToString::to_string)
	}
	fn set_date(&mut self, date: &str) {
		self.set_value(UniCase::from("Date"), date)
	}
	fn remove_date(&mut self) {
		self.remove_key(UniCase::from("Date"))
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self.get_value(UniCase::from("Year")).map(str::parse::<i32>) {
			return Some(y);
		}

		None
	}
	fn set_year(&mut self, year: i32) {
		self.set_value(UniCase::from("Year"), year.to_string())
	}
	fn remove_year(&mut self) {
		self.remove_key(UniCase::from("Year"))
	}

	fn bpm(&self) -> Option<u16> {
		if let Some(bpm) = self.get_value(UniCase::from("BPM")) {
			return bpm.parse::<u16>().ok();
		}

		None
	}
	fn set_bpm(&mut self, bpm: u16) {
		self.set_value(UniCase::from("BPM"), bpm.to_string())
	}
	fn remove_bpm(&mut self) {
		self.remove_key(UniCase::from("BPM"))
	}

	fn front_cover(&self) -> Option<Picture> {
		if let Some(val) = self.inner.item("Cover Art (Front)") {
			return self.get_picture(val);
		}

		None
	}
	fn set_front_cover(&mut self, cover: Picture) {
		self.remove_front_cover();

		if let Ok(item) = ape::Item::from_binary("Cover Art (Front)", cover.as_ape_bytes()) {
			self.inner.set_item(item)
		}
	}
	fn remove_front_cover(&mut self) {
		self.remove_key(UniCase::from("Cover Art (Front)"))
	}

	fn back_cover(&self) -> Option<Picture> {
		if let Some(val) = self.inner.item("Cover Art (Back)") {
			return self.get_picture(val);
		}

		None
	}
	fn set_back_cover(&mut self, cover: Picture) {
		self.remove_back_cover();

		if let Ok(item) = ape::Item::from_binary("Cover Art (Back)", cover.as_ape_bytes()) {
			self.inner.set_item(item)
		}
	}
	fn remove_back_cover(&mut self) {
		self.remove_key(UniCase::from("Cover Art (Back)"))
	}

	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		let mut pics = Vec::new();

		for pic_type in &APE_PICTYPES {
			if let Some(item) = self.inner.item(pic_type) {
				if let Some(pic) = self.get_picture(item) {
					pics.push(pic)
				}
			}
		}

		if pics.is_empty() {
			None
		} else {
			Some(Cow::from(pics))
		}
	}
	fn set_pictures(&mut self, pictures: Vec<Picture>) {
		self.remove_pictures();

		for p in pictures {
			let key = p.pic_type.as_ape_key();

			if let Ok(item) = ape::Item::from_binary(key, p.as_ape_bytes()) {
				self.inner.set_item(item)
			}
		}
	}
	fn remove_pictures(&mut self) {
		for key in &APE_PICTYPES {
			self.inner.remove_item(key);
		}
	}

	// Track number and total tracks are stored together as num/total?
	fn track_number(&self) -> Option<u32> {
		let numbers = self.get_value(UniCase::from("Track"));

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();

			if let Some(track_number) = split.first() {
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
			self.set_value(UniCase::from("Track"), track_str)
		} else {
			self.set_value(UniCase::from("Track"), track.to_string())
		}
	}
	fn remove_track_number(&mut self) {
		self.remove_key(UniCase::from("Track"))
	}

	fn total_tracks(&self) -> Option<u32> {
		let numbers = self.get_value(UniCase::from("Track"));

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();

			if let Some(total_tracks) = split.last() {
				if let Ok(num) = total_tracks.parse::<u32>() {
					return Some(num);
				}
			}
		}

		None
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		if let (Some(track_number), _) = self.track() {
			let track_str = format!("{}/{}", track_number, total_track);
			self.set_value(UniCase::from("Track"), track_str)
		} else {
			self.set_value(UniCase::from("Track"), format!("0/{}", total_track))
		}
	}
	fn remove_total_tracks(&mut self) {
		if let (Some(track_number), _) = self.track() {
			self.set_value(UniCase::from("Track"), track_number.to_string())
		} else {
			self.remove_track_number()
		}
	}

	fn disc_number(&self) -> Option<u32> {
		if let Some(disc_num) = self.get_value(UniCase::from("Disc")) {
			if let Ok(num) = disc_num.parse::<u32>() {
				return Some(num);
			}
		}

		None
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.set_value(UniCase::from("Disc"), disc_number.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.remove_key(UniCase::from("Disc"));
	}

	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(num)) = self.get_value(UniCase::from("Disc")).map(str::parse::<u32>) {
			return Some(num);
		}

		None
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.set_value(UniCase::from("Disc"), total_discs.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.remove_key(UniCase::from("Disc"))
	}
}

impl AudioTagWrite for ApeTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		ape::write_to(&self.inner, file)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		ape::write_to_path(&self.inner, path)?;
		Ok(())
	}
}
