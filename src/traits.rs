#[allow(clippy::wildcard_imports)]
use crate::components::tags::*;
use crate::{Album, AnyTag, Picture, Result, TagType};

use std::borrow::Cow;
use std::fs::{File, OpenOptions};

/// Combination of [`AudioTagEdit`], [`AudioTagWrite`], and [`ToAnyTag`]
pub trait AudioTag: AudioTagEdit + AudioTagWrite + ToAnyTag {}

/// Implementors of this trait are able to read and write audio metadata.
///
/// Constructor methods e.g. `from_file` should be implemented separately.
pub trait AudioTagEdit {
	/// Returns the track title
	fn title(&self) -> Option<&str>;
	/// Sets the track title
	fn set_title(&mut self, title: &str);
	/// Removes the track title
	fn remove_title(&mut self);

	/// Returns the artist(s) as a string
	fn artist_str(&self) -> Option<&str>;
	/// Sets the artist string
	fn set_artist(&mut self, artist: &str);

	/// Splits the artist string into a `Vec`
	fn artists(&self, delimiter: &str) -> Option<Vec<&str>> {
		self.artist_str().map(|a| a.split(delimiter).collect())
	}
	/// Removes the artist string
	fn remove_artist(&mut self);

	/// Returns the track date
	fn date(&self) -> Option<String> {
		self.year().map(|y| y.to_string())
	}
	/// Sets the track date
	fn set_date(&mut self, date: &str) {
		if let Ok(d) = date.parse::<i32>() {
			self.set_year(d)
		}
	}
	/// Removes the track date
	fn remove_date(&mut self) {
		self.remove_year()
	}

	/// Returns the track year
	fn year(&self) -> Option<i32> {
		None
	}
	/// Sets the track year
	fn set_year(&mut self, _year: i32) {}
	/// Removes the track year
	fn remove_year(&mut self) {}

	/// Returns the track's [`Album`]
	fn album(&self) -> Album<'_> {
		Album {
			title: self.album_title(),
			artist: self.album_artist_str(),
			covers: self.album_covers(),
		}
	}

	/// Returns the album title
	fn album_title(&self) -> Option<&str> {
		None
	}
	/// Sets the album title
	fn set_album_title(&mut self, _title: &str) {}
	/// Removes the album title
	fn remove_album_title(&mut self) {}

	/// Returns the album artist string
	fn album_artist_str(&self) -> Option<&str> {
		None
	}
	/// Splits the artist string into a `Vec`
	fn album_artists(&self, delimiter: &str) -> Option<Vec<&str>> {
		self.album_artist_str()
			.map(|a| a.split(delimiter).collect())
	}
	/// Sets the album artist string
	fn set_album_artist(&mut self, _artist: &str) {}
	/// Removes the album artist string
	fn remove_album_artists(&mut self) {}

	/// Returns the front and back album covers
	fn album_covers(&self) -> (Option<Picture>, Option<Picture>) {
		(self.front_cover(), self.back_cover())
	}
	/// Removes both album covers
	fn remove_album_covers(&mut self) {
		self.remove_front_cover();
		self.remove_back_cover();
	}

	/// Returns the front cover
	fn front_cover(&self) -> Option<Picture> {
		None
	}
	/// Sets the front cover
	fn set_front_cover(&mut self, _cover: Picture) {}
	/// Removes the front cover
	fn remove_front_cover(&mut self) {}

	/// Returns the front cover
	fn back_cover(&self) -> Option<Picture> {
		None
	}
	/// Sets the front cover
	fn set_back_cover(&mut self, _cover: Picture) {}
	/// Removes the front cover
	fn remove_back_cover(&mut self) {}

	/// Returns an `Iterator` over all pictures stored in the track
	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		None
	}

	/// Returns the track number and total tracks
	fn track(&self) -> (Option<u32>, Option<u32>) {
		(self.track_number(), self.total_tracks())
	}
	/// Sets the track number and total tracks
	fn set_track(&mut self, track_number: u32, total_tracks: u32) {
		self.set_track_number(track_number);
		self.set_total_tracks(total_tracks);
	}
	/// Removes the track number and total tracks
	fn remove_track(&mut self) {
		self.remove_track_number();
		self.remove_total_tracks();
	}

	/// Returns the track number
	fn track_number(&self) -> Option<u32> {
		None
	}
	/// Sets the track number
	fn set_track_number(&mut self, _track_number: u32) {}
	/// Removes the track number
	fn remove_track_number(&mut self) {}

	/// Returns the total tracks
	fn total_tracks(&self) -> Option<u32> {
		None
	}
	/// Sets the total tracks
	fn set_total_tracks(&mut self, _total_track: u32) {}
	/// Removes the total tracks
	fn remove_total_tracks(&mut self) {}

	/// Returns the disc number and total discs
	fn disc(&self) -> (Option<u32>, Option<u32>) {
		(self.disc_number(), self.total_discs())
	}
	/// Removes the disc number and total discs
	fn remove_disc(&mut self) {
		self.remove_disc_number();
		self.remove_total_discs();
	}

	/// Returns the disc number
	fn disc_number(&self) -> Option<u32> {
		None
	}
	/// Sets the disc number
	fn set_disc_number(&mut self, _disc_number: u32) {}
	/// Removes the disc number
	fn remove_disc_number(&mut self) {}

	/// Returns the total discs
	fn total_discs(&self) -> Option<u32> {
		None
	}
	/// Sets the total discs
	fn set_total_discs(&mut self, _total_discs: u32) {}
	/// Removes the total discs
	fn remove_total_discs(&mut self) {}
}

/// Functions for writing to a file
pub trait AudioTagWrite {
	/// Write tag to a [`File`][std::fs::File]
	///
	/// # Errors
	///
	/// Will return `Err` if unable to write to the `File`
	fn write_to(&self, file: &mut File) -> Result<()>;
	/// Write tag to a path
	///
	/// # Errors
	///
	/// Will return `Err` if `path` doesn't exist
	fn write_to_path(&self, path: &str) -> Result<()> {
		self.write_to(&mut OpenOptions::new().read(true).write(true).open(path)?)?;

		Ok(())
	}
}

/// Conversions between tag types
pub trait ToAnyTag: ToAny {
	/// Converts the tag to [`AnyTag`]
	fn to_anytag(&self) -> AnyTag<'_>;

	/// Convert the tag type, which can be lossy.
	fn to_dyn_tag(&self, tag_type: TagType) -> Box<dyn AudioTag> {
		// TODO: write a macro or something that implement this method for every tag type so that if the
		// TODO: target type is the same, just return self
		match tag_type {
			#[cfg(feature = "format-ape")]
			TagType::Ape => Box::new(ApeTag::from(self.to_anytag())),
			#[cfg(feature = "format-id3")]
			TagType::Id3v2(_) => Box::new(Id3v2Tag::from(self.to_anytag())),
			#[cfg(feature = "format-mp4")]
			TagType::Mp4 => Box::new(Mp4Tag::from(self.to_anytag())),
			#[cfg(any(
				feature = "format-vorbis",
				feature = "format-flac",
				feature = "format-opus"
			))]
			TagType::Ogg(_) => Box::new(OggTag::from(self.to_anytag())),
			#[cfg(feature = "format-riff")]
			TagType::RiffInfo => Box::new(RiffTag::from(self.to_anytag())),
			#[cfg(feature = "format-aiff")]
			TagType::AiffText => Box::new(AiffTag::from(self.to_anytag())),
		}
	}
}

/// Tag conversion to `Any`
pub trait ToAny {
	/// Convert tag to `Any`
	fn to_any(&self) -> &dyn std::any::Any;
	/// Mutably convert tag to `Any`
	#[allow(clippy::wrong_self_convention)]
	fn to_any_mut(&mut self) -> &mut dyn std::any::Any;
}
