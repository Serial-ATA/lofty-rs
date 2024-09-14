/// The parental advisory rating
///
/// See also:
/// * <https://docs.mp3tag.de/mapping/#itunesadvisory>
/// * <https://exiftool.org/TagNames/QuickTime.html>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvisoryRating {
	/// *Inoffensive*/*None* (0)
	Inoffensive,
	/// *Explicit* (1 or 4)
	///
	/// In the past Apple used the value `4` for explicit content
	/// that has later been replaced by `1`. Both values are considered
	/// as valid when reading but only the newer value `1` is written.
	Explicit,
	/// *Clean*/*Edited* (2)
	Clean,
}

impl AdvisoryRating {
	/// Returns the rating as it appears in the `rtng` atom
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::AdvisoryRating;
	///
	/// assert_eq!(AdvisoryRating::Inoffensive.as_u8(), 0);
	/// assert_eq!(AdvisoryRating::Explicit.as_u8(), 1);
	/// assert_eq!(AdvisoryRating::Clean.as_u8(), 2);
	/// ```
	pub fn as_u8(&self) -> u8 {
		match self {
			AdvisoryRating::Inoffensive => 0,
			AdvisoryRating::Explicit => 1,
			AdvisoryRating::Clean => 2,
		}
	}
}

impl TryFrom<u8> for AdvisoryRating {
	type Error = u8;

	fn try_from(input: u8) -> Result<Self, Self::Error> {
		match input {
			0 => Ok(Self::Inoffensive),
			1 | 4 => Ok(Self::Explicit),
			2 => Ok(Self::Clean),
			value => Err(value),
		}
	}
}
