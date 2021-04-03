use super::picture::Picture;

/// A struct for representing an album for convenience.
#[derive(Debug)]
pub struct Album<'a> {
	pub title: &'a str,
	pub artist: Option<&'a str>,
	pub cover: Option<Picture<'a>>,
}

impl<'a> Album<'a> {
	pub fn title(title: &'a str) -> Self {
		Self {
			title,
			artist: None,
			cover: None,
		}
	}
	pub fn artist(mut self, artist: &'a str) -> Self {
		self.artist = Some(artist);
		self
	}
	pub fn cover(mut self, cover: Picture<'a>) -> Self {
		self.cover = Some(cover);
		self
	}
	pub fn full(title: &'a str, artist: &'a str, cover: Picture<'a>) -> Self {
		Self {
			title,
			artist: Some(artist),
			cover: Some(cover),
		}
	}
}
