use super::picture::Picture;

/// A struct for representing an album for convenience.
#[derive(Debug, Clone)]
pub struct Album<'a> {
	pub title: Option<&'a str>,
	pub artists: Option<Vec<&'a str>>,
	pub cover: Option<Picture<'a>>,
}

impl<'a> Default for Album<'a> {
	fn default() -> Self {
		Self {
			title: None,
			artists: None,
			cover: None,
		}
	}
}

impl<'a> Album<'a> {
	/// Create a new `Album`
	pub fn new(
		title: Option<&'a str>,
		artists: Option<Vec<&'a str>>,
		cover: Option<Picture<'a>>,
	) -> Self {
		Self {
			title,
			artists,
			cover,
		}
	}
	/// Create a new album with the specified title
	pub fn with_title(title: &'a str) -> Self {
		Self {
			title: Some(title),
			artists: None,
			cover: None,
		}
	}
	/// Set the album artists
	pub fn set_artists(mut self, artists: Vec<&'a str>) -> Self {
		self.artists = Some(artists);
		self
	}
	/// Appends an artist to the `artists` vec
	pub fn append_artist(mut self, artist: &'a str) {
		if let Some(mut artists) = self.artists {
			artists.push(artist)
		} else {
			self.artists = Some(vec![artist])
		}
	}
	/// Set the album cover
	pub fn set_cover(mut self, cover: Picture<'a>) -> Self {
		self.cover = Some(cover);
		self
	}
	/// Clears the `artists` field
	pub fn remove_artists(mut self) {
		self.artists = None
	}
	/// Clears the `cover` field
	pub fn remove_cover(mut self) {
		self.cover = None
	}
	/// Turns `artists` vec into a comma separated String
	pub fn artists_as_string(&self) -> Option<String> {
		self.artists.as_ref().map(|artists| artists.join(","))
	}
}
