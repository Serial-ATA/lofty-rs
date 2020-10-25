use std::convert::TryFrom;

use crate::ErrorKind;

// iTunes media types
/// A media type code stored in the `stik` atom.
pub const MOVIE: u8 = 0;
/// A media type code stored in the `stik` atom.
pub const NORMAL: u8 = 1;
/// A media type code stored in the `stik` atom.
pub const AUDIOBOOK: u8 = 2;
/// A media type code stored in the `stik` atom.
pub const WHACKED_BOOKMARK: u8 = 5;
/// A media type code stored in the `stik` atom.
pub const MUSIC_VIDEO: u8 = 6;
/// A media type code stored in the `stik` atom.
pub const SHORT_FILM: u8 = 9;
/// A media type code stored in the `stik` atom.
pub const TV_SHOW: u8 = 10;
/// A media type code stored in the `stik` atom.
pub const BOOKLET: u8 = 11;

// iTunes advisory ratings
/// An advisory rating code stored in the `rtng` atom.
pub const CLEAN: u8 = 2;
/// An advisory rating code stored in the `rtng` atom.
pub const INOFFENSIVE: u8 = 0;

/// An enum describing the media type of a file stored in the `stik` atom.
#[derive(Debug, Clone, PartialEq)]
pub enum MediaType {
    /// A media type stored as 0 in the `stik` atom.
    Movie,
    /// A media type stored as 1 in the `stik` atom.
    Normal,
    /// A media type stored as 2 in the `stik` atom.
    AudioBook,
    /// A media type stored as 5 in the `stik` atom.
    WhackedBookmark,
    /// A media type stored as 6 in the `stik` atom.
    MusicVideo,
    /// A media type stored as 9 in the `stik` atom.
    ShortFilm,
    /// A media type stored as 10 in the `stik` atom.
    TvShow,
    /// A media type stored as 11 in the `stik` atom.
    Booklet,
}

impl MediaType {
    /// Returns the integer value corresponding to the media type.
    pub fn value(&self) -> u8 {
        match self {
            Self::Movie => MOVIE,
            Self::Normal => NORMAL,
            Self::AudioBook => AUDIOBOOK,
            Self::WhackedBookmark => WHACKED_BOOKMARK,
            Self::MusicVideo => MUSIC_VIDEO,
            Self::ShortFilm => SHORT_FILM,
            Self::TvShow => TV_SHOW,
            Self::Booklet => BOOKLET,
        }
    }
}

impl TryFrom<u8> for MediaType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            MOVIE => Ok(Self::Movie),
            NORMAL => Ok(Self::Normal),
            AUDIOBOOK => Ok(Self::AudioBook),
            WHACKED_BOOKMARK => Ok(Self::WhackedBookmark),
            MUSIC_VIDEO => Ok(Self::MusicVideo),
            SHORT_FILM => Ok(Self::ShortFilm),
            TV_SHOW => Ok(Self::TvShow),
            BOOKLET => Ok(Self::Booklet),
            _ => Err(crate::Error::new(
                ErrorKind::UnknownMediaType(value),
                "Unknown media type".into(),
            )),
        }
    }
}

/// An enum describing the rating of a file stored in the `rtng` atom.
#[derive(Debug, Clone, PartialEq)]
pub enum AdvisoryRating {
    /// An advisory rating stored as 2 in the `rtng` atom.
    Clean,
    /// An advisory rating stored as 0 in the `rtng` atom.
    Inoffensive,
    /// An advisory rating indicated by any other value than 0 or 2 in the `rtng` atom, containing
    /// the value.
    Explicit(u8),
}

impl AdvisoryRating {
    /// Returns the integer value corresponding to the rating.
    pub fn value(&self) -> u8 {
        match self {
            Self::Clean => CLEAN,
            Self::Inoffensive => INOFFENSIVE,
            Self::Explicit(r) => *r,
        }
    }
}

impl From<u8> for AdvisoryRating {
    fn from(rating: u8) -> Self {
        match rating {
            CLEAN => Self::Clean,
            INOFFENSIVE => Self::Inoffensive,
            _ => Self::Explicit(rating),
        }
    }
}
