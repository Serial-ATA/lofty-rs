#[allow(clippy::wildcard_imports)]
use crate::components::tags::*;
use crate::tag::RiffFormat;
use crate::{Album, AnyTag, Picture, Result, TagType};

use std::fs::File;

pub trait AudioTag: AudioTagEdit + AudioTagWrite + ToAnyTag {}

/// Implementors of this trait are able to read and write audio metadata.
///
/// Constructor methods e.g. `from_file` should be implemented separately.
pub trait AudioTagEdit {
	fn title(&self) -> Option<&str>;
	fn set_title(&mut self, title: &str);
	fn remove_title(&mut self);

	fn artist_str(&self) -> Option<&str>;
	fn set_artist(&mut self, artist: &str);

	fn artists_vec(&self) -> Option<Vec<&str>> {
		self.artist_str().map(|a| a.split('/').collect())
	}
	fn remove_artist(&mut self);

	fn year(&self) -> Option<i32>;
	fn set_year(&mut self, year: i32);
	fn remove_year(&mut self);

	fn album(&self) -> Album<'_> {
		Album {
			title: self.album_title(),
			artists: self.album_artists_vec(),
			cover: self.album_cover(),
		}
	}

	fn album_title(&self) -> Option<&str>;
	fn set_album_title(&mut self, v: &str);
	fn remove_album_title(&mut self);

	fn album_artist_str(&self) -> Option<&str>;
	fn album_artists_vec(&self) -> Option<Vec<&str>> {
		self.album_artist_str().map(|a| a.split('/').collect())
	}
	fn set_album_artist(&mut self, artist: &str);
	fn remove_album_artists(&mut self);

	fn album_cover(&self) -> Option<Picture>;
	fn set_album_cover(&mut self, cover: Picture);
	fn remove_album_cover(&mut self);

	fn track(&self) -> (Option<u32>, Option<u32>) {
		(self.track_number(), self.total_tracks())
	}
	fn set_track(&mut self, track: u32) {
		self.set_track_number(track);
	}
	fn remove_track(&mut self) {
		self.remove_track_number();
		self.remove_total_tracks();
	}

	fn track_number(&self) -> Option<u32>;
	fn set_track_number(&mut self, track_number: u32);
	fn remove_track_number(&mut self);

	fn total_tracks(&self) -> Option<u32>;
	fn set_total_tracks(&mut self, total_track: u32);
	fn remove_total_tracks(&mut self);

	fn disc(&self) -> (Option<u32>, Option<u32>) {
		(self.disc_number(), self.total_discs())
	}
	fn set_disc(&mut self, disc: u32) {
		self.set_disc_number(disc);
	}
	fn remove_disc(&mut self) {
		self.remove_disc_number();
		self.remove_total_discs();
	}

	fn disc_number(&self) -> Option<u32>;
	fn set_disc_number(&mut self, disc_number: u32);
	fn remove_disc_number(&mut self);

	fn total_discs(&self) -> Option<u32>;
	fn set_total_discs(&mut self, total_discs: u32);
	fn remove_total_discs(&mut self);
}

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
	fn write_to_path(&self, path: &str) -> Result<()>;
}

pub trait ToAnyTag: ToAny {
	fn to_anytag(&self) -> AnyTag<'_>;

	/// Convert the tag type, which can be lossy.
	fn to_dyn_tag(&self, tag_type: TagType) -> Box<dyn AudioTag> {
		// TODO: write a macro or something that implement this method for every tag type so that if the
		// TODO: target type is the same, just return self
		match tag_type {
			#[cfg(feature = "ape")]
			TagType::Ape => Box::new(ApeTag::from(self.to_anytag())),
			#[cfg(feature = "mp3")]
			TagType::Id3v2 | TagType::Riff(RiffFormat::ID3) => Box::new(Id3v2Tag::from(self.to_anytag())),
			#[cfg(feature = "mp4")]
			TagType::Mp4 => Box::new(Mp4Tag::from(self.to_anytag())),
			#[cfg(feature = "vorbis")]
			TagType::Vorbis(_) => Box::new(VorbisTag::from(self.to_anytag())),
			#[cfg(feature = "wav")]
			TagType::Riff(RiffFormat::Info) => Box::new(RiffTag::from(self.to_anytag())),
		}
	}
}

pub trait ToAny {
	fn to_any(&self) -> &dyn std::any::Any;
	fn to_any_mut(&mut self) -> &mut dyn std::any::Any;
}
