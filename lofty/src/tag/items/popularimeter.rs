//! Generic user star rating (a.k.a. popularimeter) support
//!
//! Popularimeters, in ID3v2 terms, are star ratings with an associated email and play counter.
//! Most tag formats support ratings in some capacity, usually with a subset of the information available
//! in ID3v2. For example, Vorbis Comments supports star ratings and optionally emails, but not play
//! counters.
//!
//! This module contains a generic [`Popularimeter`], which supports all of the information available
//! in ID3v2, and can (lossily) be converted to all other supported formats.
//!
//! ## Ratings
//!
//! Unfortunately, no tag format has a standard for mapping integer values to star ratings. This means
//! many application-specific mappings have appeared over the years (see [providers](#providers)).
//!
//! Additionally, most applications assume whole-number star ratings. As such, this generic format
//! can **not** be used for fractional ratings.
//!
//! ### Providers
//!
//! Lofty provides multiple rating providers for popular applications (e.g. [`MusicBeeProvider`]).
//! These providers are dispatched by email, as many apps will use the `email` field to store their
//! name (e.g. "Windows Media Player 9 Series"). These providers can **not** be disabled.
//!
//! To define a custom rating provider, see [`RatingProvider`] and [`set_custom_rating_provider()`].
//! However, consider checking out the existing providers to have wider application support.
//!
//! ## Usage
//!
//! ```
//! use lofty::tag::items::popularimeter::{Popularimeter, StarRating};
//! use lofty::tag::{ItemKey, Tag, TagType};
//!
//! // Create a MusicBee-style popularimeter
//! let play_counter = 10;
//! let rating = Popularimeter::musicbee(StarRating::Three, play_counter);
//!
//! // Popularimeters can be inserted like any other text value.
//! // All format-specific encoding is handled behind-the-scenes.
//! let mut tag = Tag::new(TagType::Id3v2);
//! tag.insert_text(ItemKey::Popularimeter, rating.to_string());
//!
//! // Then all ratings can be fetched
//! let ratings = tag.ratings().collect::<Vec<_>>();
//! assert_eq!(ratings.len(), 1);
//! ```

use crate::tag::TagType;

use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::sync::OnceLock;

/// A whole-number star rating
///
/// There is no generic way to handle fractional ratings, as *many* players/taggers will assume
/// integer ratings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarRating {
	/// One star
	One = 1,
	/// Two stars
	Two = 2,
	/// Three stars
	Three = 3,
	/// Four stars
	Four = 4,
	/// Five stars
	Five = 5,
}

/// A rating provider for [`Popularimeter`]s
///
/// As there is no standard for ratings, there are multiple providers for popular applications
/// available (e.g. [`MusicBeeProvider`]). See [`Popularimeter`] for more details.
///
/// To set a custom rating provider, see [`set_custom_rating_provider()`].
pub trait RatingProvider: Send + Sync {
	/// Whether this provider should be used for the given email
	fn supports_email(&self, email: &str) -> bool;

	/// Converts a [`StarRating`] to a format-specific integer
	fn rate(&self, tag_type: TagType, rating: StarRating) -> u8;

	/// Converts a format-specific rating to a [`StarRating`]
	fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating;
}

static CUSTOM_PROVIDER: OnceLock<&'static dyn RatingProvider> = OnceLock::new();

/// Apply a custom rating provider globally
///
/// By default, the fallback provider will be [`DefaultRatingProvider`].
///
/// # Panics
///
/// This will panic if called more than once.
///
/// # Examples
///
/// ```
/// use lofty::tag::TagType;
/// use lofty::tag::items::popularimeter::{
/// 	RatingProvider, StarRating, set_custom_rating_provider,
/// };
///
/// struct MyCustomProvider;
///
/// impl RatingProvider for MyCustomProvider {
/// 	fn supports_email(&self, email: &str) -> bool {
/// 		true // Support all emails
/// 	}
///
/// 	fn rate(&self, tag_type: TagType, rating: StarRating) -> u8 {
/// 		match tag_type {
/// 			_ => todo!("Mapping of `StarRating` to format-specific values"),
/// 		}
/// 	}
///
/// 	fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating {
/// 		match tag_type {
/// 			_ => todo!("Mapping of format-specific values to `StarRating`"),
/// 		}
/// 	}
/// }
///
/// set_custom_rating_provider(MyCustomProvider);
/// ```
pub fn set_custom_rating_provider<T>(provider: T)
where
	T: RatingProvider + 'static,
{
	assert!(
		CUSTOM_PROVIDER.set(Box::leak(Box::new(provider))).is_ok(),
		"Multiple calls to `set_custom_rating_provider()`"
	);
}

fn custom_provider() -> &'static dyn RatingProvider {
	CUSTOM_PROVIDER.get().map_or(DEFAULT_PROVIDER, |p| *p)
}

/// The default [`RatingProvider`] used as a fallback for unknown ratings
///
/// This is equivalent to the [`MusicBeeProvider`], except it supports *all* emails.
///
/// This can be overwritten with [`set_custom_rating_provider()`].
pub struct DefaultRatingProvider;

impl RatingProvider for DefaultRatingProvider {
	fn supports_email(&self, _: &str) -> bool {
		true
	}

	// MusicBee-style ratings seem to be the most widely used (?)
	fn rate(&self, tag_type: TagType, rating: StarRating) -> u8 {
		MUSICBEE_PROVIDER.rate(tag_type, rating)
	}

	fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating {
		MUSICBEE_PROVIDER.convert_raw(tag_type, rating)
	}
}

static DEFAULT_PROVIDER: &'static dyn RatingProvider = &DefaultRatingProvider;
static MUSICBEE_PROVIDER: &'static dyn RatingProvider = &MusicBeeProvider;
static WMP_PROVIDER: &'static dyn RatingProvider = &WindowsMediaPlayerProvider;
static PICARD_PROVIDER: &'static dyn RatingProvider = &PicardProvider;

/// A [MusicBee]-style rating
///
/// [MusicBee]: https://getmusicbee.com/
pub struct MusicBeeProvider;

impl RatingProvider for MusicBeeProvider {
	fn supports_email(&self, email: &str) -> bool {
		email == Popularimeter::MUSICBEE_EMAIL
	}

	fn rate(&self, tag_type: TagType, rating: StarRating) -> u8 {
		match tag_type {
			TagType::Id3v2 => match rating {
				StarRating::One => 1,
				StarRating::Two => 64,
				StarRating::Three => 128,
				StarRating::Four => 196,
				StarRating::Five => 255,
			},
			_ => {
				let stars = rating as u8;
				stars.saturating_mul(20)
			},
		}
	}

	#[allow(clippy::match_overlapping_arm)]
	fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating {
		match tag_type {
			TagType::Id3v2 => match rating {
				..=1 => StarRating::One,
				..=64 => StarRating::Two,
				..=128 => StarRating::Three,
				..=196 => StarRating::Four,
				..=255 => StarRating::Five,
			},
			_ => match rating {
				..=20 => StarRating::One,
				..=40 => StarRating::Two,
				..=60 => StarRating::Three,
				..=80 => StarRating::Four,
				_ => StarRating::Five,
			},
		}
	}
}

/// A [Windows Media Player]-style rating
///
/// [Windows Media Player]: https://en.wikipedia.org/wiki/Windows_Media_Player
pub struct WindowsMediaPlayerProvider;

impl RatingProvider for WindowsMediaPlayerProvider {
	fn supports_email(&self, email: &str) -> bool {
		email == Popularimeter::WMP_EMAIL
	}

	fn rate(&self, _: TagType, rating: StarRating) -> u8 {
		// WMP only supports ID3v2 ratings, and uses the same values as MusicBee
		MusicBeeProvider.rate(TagType::Id3v2, rating)
	}

	fn convert_raw(&self, _: TagType, rating: u8) -> StarRating {
		MusicBeeProvider.convert_raw(TagType::Id3v2, rating)
	}
}

/// A [MusicBrainz Picard]-style rating
///
/// [MusicBrainz Picard]: https://picard.musicbrainz.org/
pub struct PicardProvider;

impl RatingProvider for PicardProvider {
	fn supports_email(&self, email: &str) -> bool {
		email == Popularimeter::PICARD_EMAIL
	}

	fn rate(&self, tag_type: TagType, rating: StarRating) -> u8 {
		match tag_type {
			TagType::Id3v2 => {
				let stars = rating as u8;
				stars.saturating_mul(51)
			},
			_ => {
				let stars = rating as u8;
				stars.saturating_mul(5)
			},
		}
	}

	#[allow(clippy::match_overlapping_arm)]
	fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating {
		match tag_type {
			TagType::Id3v2 => match rating {
				..=51 => StarRating::One,
				..=102 => StarRating::Two,
				..=153 => StarRating::Three,
				..=204 => StarRating::Four,
				..=255 => StarRating::Five,
			},
			_ => match rating {
				..=5 => StarRating::One,
				..=10 => StarRating::Two,
				..=15 => StarRating::Three,
				..=20 => StarRating::Four,
				_ => StarRating::Five,
			},
		}
	}
}

/// A generic user rating and play counter
///
/// Unfortunately, there is no widely agreed-upon scale for ratings. There are constructors for multiple
/// popular taggers (e.g. [Popularimeter::musicbee()]), that will create a properly scaled [`StarRating`].
///
/// In most cases, apps will look for ratings with their specific [`email`], meaning it *should* be
/// safe to write multiple application-specific ratings to the same file.
///
/// See the module docs for more information.
///
/// See also: [`StarRating`]
///
/// [`email`]: Popularimeter::email
#[derive(Clone)]
pub struct Popularimeter<'a> {
	// Private since the email is what determines how we handle conversions
	pub(crate) email: Option<Cow<'a, str>>,
	pub(crate) rating_provider: &'static dyn RatingProvider,
	/// The star rating provided by the user
	pub rating: StarRating,
	/// The number of times the user has played the song
	///
	/// NOTE: This is only supported in ID3v2
	pub play_counter: u64,
}

impl Debug for Popularimeter<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Popularimeter")
			.field("email", &self.email)
			.field("rating", &self.rating)
			.field("play_counter", &self.play_counter)
			.finish()
	}
}

impl<'a> Popularimeter<'a> {
	const WMP_EMAIL: &'static str = "Windows Media Player 9 Series";
	const MUSICBEE_EMAIL: &'static str = "MusicBee";
	const PICARD_EMAIL: &'static str = "users@musicbrainz.org";

	/// Create a new [`Popularimeter`] using the custom [`RatingProvider`]
	///
	/// NOTES:
	///
	/// * This will use [`DefaultRatingProvider`] if [`set_custom_rating_provider()`] has not been called.
	/// * For wider support, check out the tagger-specific constructors (e.g. [`Popularimeter::musicbee()`])
	///   first, before creating a custom rating scale.
	///
	/// # Examples
	///
	/// ```
	/// use lofty::tag::items::popularimeter::{Popularimeter, StarRating};
	///
	/// let rating = Popularimeter::custom("foo@example.com", StarRating::Three, 5);
	/// assert_eq!(rating.email(), Some("foo@example.com"));
	/// assert_eq!(rating.rating(), StarRating::Three);
	/// assert_eq!(rating.play_counter, 5);
	/// ```
	pub fn custom(email: impl Into<Cow<'a, str>>, rating: StarRating, play_counter: u64) -> Self {
		Self {
			email: Some(email.into()),
			rating_provider: custom_provider(),
			rating,
			play_counter,
		}
	}

	pub(crate) fn mapped(
		email: impl Into<Cow<'a, str>>,
		tag_type: TagType,
		rate: u8,
		play_counter: u64,
	) -> Option<Self> {
		let email = email.into();

		let rating_provider;
		match &*email {
			Popularimeter::WMP_EMAIL => rating_provider = WMP_PROVIDER,
			Popularimeter::MUSICBEE_EMAIL => rating_provider = MUSICBEE_PROVIDER,
			Popularimeter::PICARD_EMAIL => rating_provider = PICARD_PROVIDER,
			_ => {
				rating_provider = custom_provider();
				if !rating_provider.supports_email(&email) {
					return None;
				}
			},
		}

		let star_rating = rating_provider.convert_raw(tag_type, rate);
		Some(Self {
			email: (!email.is_empty()).then_some(email),
			rating_provider,
			rating: star_rating,
			play_counter,
		})
	}

	/// The email associated with this rating
	///
	/// NOTE: In many cases, this will be an application name (e.g. "Windows Media Player 9 Series"), rather
	///       than an actual email.
	///
	/// # Examples
	///
	/// ```
	/// use lofty::tag::items::popularimeter::{Popularimeter, StarRating};
	///
	/// let rating = Popularimeter::custom("foo@example.com", StarRating::Three, 5);
	/// assert_eq!(rating.email(), Some("foo@example.com"))
	/// ```
	pub fn email(&self) -> Option<&str> {
		self.email.as_deref()
	}

	/// The user's rating
	///
	/// # Examples
	///
	/// ```
	/// use lofty::tag::items::popularimeter::{Popularimeter, StarRating};
	///
	/// let rating = Popularimeter::musicbee(StarRating::Three, 5);
	/// assert_eq!(rating.rating(), StarRating::Three);
	/// ```
	pub fn rating(&self) -> StarRating {
		self.rating
	}

	pub(crate) fn mapped_value(&self, tag_type: TagType) -> u8 {
		self.rating_provider.rate(tag_type, self.rating)
	}

	pub(crate) fn from_str(s: &str) -> Result<Self, ()> {
		let mut parts = s.splitn(3, '|');
		let email = parts.next().ok_or(())?;
		let star_rating = parts.next().ok_or(())?;
		let play_counter = parts
			.next()
			.ok_or(())
			.and_then(|p| p.parse::<u64>().map_err(|_| ()))?;

		let star_rating = match star_rating.parse().map_err(|_| ())? {
			1 => StarRating::One,
			2 => StarRating::Two,
			3 => StarRating::Three,
			4 => StarRating::Four,
			5 => StarRating::Five,
			_ => return Err(()),
		};

		let rating_provider;
		match email {
			Popularimeter::WMP_EMAIL => rating_provider = WMP_PROVIDER,
			Popularimeter::MUSICBEE_EMAIL => rating_provider = MUSICBEE_PROVIDER,
			Popularimeter::PICARD_EMAIL => rating_provider = PICARD_PROVIDER,
			_ => {
				rating_provider = custom_provider();
				if !rating_provider.supports_email(email) {
					return Err(());
				}
			},
		}

		Ok(Popularimeter {
			email: (!email.is_empty()).then(|| Cow::Owned(email.to_owned())),
			rating_provider,
			rating: star_rating,
			play_counter,
		})
	}
}

impl Popularimeter<'static> {
	/// Create a new [Windows Media Player]-style rating
	///
	/// [Windows Media Player]: https://en.wikipedia.org/wiki/Windows_Media_Player
	pub fn windows_media_player(rating: StarRating, play_counter: u64) -> Self {
		Self {
			email: Some(Cow::Borrowed(Self::WMP_EMAIL)),
			rating_provider: WMP_PROVIDER,
			rating,
			play_counter,
		}
	}

	/// Create a new [MusicBee]-style rating
	///
	/// [MusicBee]: https://getmusicbee.com/
	pub fn musicbee(rating: StarRating, play_counter: u64) -> Self {
		Self {
			email: Some(Cow::Borrowed(Self::MUSICBEE_EMAIL)),
			rating_provider: MUSICBEE_PROVIDER,
			rating,
			play_counter,
		}
	}

	/// Create a new [MusicBrainz Picard]-style rating
	///
	/// [MusicBrainz Picard]: https://picard.musicbrainz.org/
	pub fn picard(rating: StarRating, play_counter: u64) -> Self {
		Self {
			email: Some(Cow::Borrowed(Self::PICARD_EMAIL)),
			rating_provider: PICARD_PROVIDER,
			rating,
			play_counter,
		}
	}
}

impl Display for Popularimeter<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let email = self.email.as_deref().unwrap_or("");
		write!(f, "{email}|{}|{}", self.rating as u8, self.play_counter)
	}
}
