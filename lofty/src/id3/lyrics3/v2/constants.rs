pub(crate) const LYRICS3V2_HEADER: &str = "LYRICSBEGIN";
pub(crate) const LYRICS3V2_FOOTER: &str = "LYRICS200";

/// The absolute max tax size. This is the size of everything in the tag except the size and [`LYRICS3V2_FOOTER`].
pub(crate) const TAG_MAX_SIZE: u32 = 999_999;
/// The absolute max field size. Some fields have smaller limits.
pub(crate) const FIELD_MAX_SIZE: u32 = 99999;
/// Max size of most text fields (e.g., artist name, image descriptions, etc.)
pub(crate) const BASIC_TEXT_FIELD_MAX_SIZE: u32 = 250;
/// Max size of image paths
pub(crate) const PATH_MAX: u32 = 4096;

pub(crate) const FIELD_INDICATORS: [u8; 3] = *b"IND";
pub(crate) const FIELD_LYRICS: [u8; 3] = *b"LYR";
pub(crate) const FIELD_INFO: [u8; 3] = *b"INF";
pub(crate) const FIELD_AUTHOR: [u8; 3] = *b"AUT";
pub(crate) const FIELD_EXTENDED_ALBUM_NAME: [u8; 3] = *b"EAL";
pub(crate) const FIELD_EXTENDED_ARTIST_NAME: [u8; 3] = *b"EAR";
pub(crate) const FIELD_EXTENDED_TRACK_TITLE: [u8; 3] = *b"ETT";
pub(crate) const FIELD_IMAGE: [u8; 3] = *b"IMG";

use crate::tag::ItemKey;
pub(crate) const VALID_ITEMKEYS: [ItemKey; 6] = [
	ItemKey::Lyrics,
	ItemKey::Comment,
	ItemKey::Lyricist,
	ItemKey::AlbumTitle,
	ItemKey::TrackArtist,
	ItemKey::TrackTitle,
];
