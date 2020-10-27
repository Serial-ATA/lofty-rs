//! # audiotags
//!
//! [![Crate](https://img.shields.io/crates/v/audiotags.svg)](https://crates.io/crates/audiotags)
//! [![Crate](https://img.shields.io/crates/d/audiotags.svg)](https://crates.io/crates/audiotags)
//! [![Crate](https://img.shields.io/crates/l/audiotags.svg)](https://crates.io/crates/audiotags)
//! [![Documentation](https://docs.rs/audiotags/badge.svg)](https://docs.rs/audiotags/)
//!
//! This crate makes it easier to parse, convert and write metadata (a.k.a tag) in audio files of different file types.
//!
//! This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3, flac, and m4a files with a single function: `Tag::default().read_from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** etc. in order to parse metadata in different file formats.
//!
//! ## Performace
//!
//! Using **audiotags** incurs a little overhead due to vtables if you want to guess the metadata format (from file extension). Apart from this there is the performance if no different from directly calling function provided by the 'specialized' crates.
//!
//! No copies will be made if you only need to read and write metadata of one format. If you want to convert between tags, copying is unavoidable no matter if you use **audiotags** or use getters and setters provided by specialized libraries. **audiotags** is not making additional unnecessary copies.
//!
//! Theoretically it is possible to achieve zero-copy conversions if all parsers can parse into a unified struct. However, this is going to be a lot of work. I might be able to implement them, but it will be no sooner than the Christmas vacation.
//!
//! See [README](https://github.com/TianyiShi2001/audiotags) for some examples.

pub(crate) use audiotags_dev_macro::*;

mod id3_tag;
pub use id3_tag::Id3v2Tag;
mod flac_tag;
mod mp4_tag;
pub use flac_tag::FlacTag;
pub use mp4_tag::Mp4Tag;

pub mod error;
pub use error::{Error, Result};

pub mod config;
pub use config::Config;

use std::convert::From;
use std::fs::File;
use std::path::Path;

use std::convert::{TryFrom, TryInto};

#[derive(Clone, Copy, Debug)]
pub enum TagType {
    // /// Guess the tag type based on the file extension
    // Guess,
    /// ## Common file extensions
    ///
    /// `.mp3`
    ///
    /// ## References
    ///
    /// - https://www.wikiwand.com/en/ID3
    Id3v2,
    Flac,
    /// ## Common file extensions
    ///
    /// `.mp4, .m4a, .m4p, .m4b, .m4r and .m4v`
    ///
    /// ## References
    ///
    /// - https://www.wikiwand.com/en/MPEG-4_Part_14
    Mp4,
}

#[rustfmt::skip]
impl TagType {
    fn try_from_ext(ext: &str) -> crate::Result<Self> {
        match ext {
                                                     "mp3" => Ok(Self::Id3v2),
            "m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
                                                    "flac" => Ok(Self::Flac),
            p @ _ => Err(crate::Error::UnsupportedFormat(p.to_owned())),
        }
    }
}

#[derive(Default)]
pub struct Tag {
    tag_type: Option<TagType>,
    config: Config,
}

impl Tag {
    pub fn with_tag_type(tag_type: TagType) -> Self {
        Self {
            tag_type: Some(tag_type),
            config: Config::default(),
        }
    }
    pub fn with_config(config: Config) -> Self {
        Self {
            tag_type: None,
            config: config.clone(),
        }
    }
    pub fn with_tag_type_and_config(tag_type: TagType, config: Config) -> Self {
        Self {
            tag_type: Some(tag_type),
            config: config.clone(),
        }
    }

    pub fn read_from_path(&self, path: impl AsRef<Path>) -> crate::Result<Box<dyn AudioTag>> {
        match self.tag_type.unwrap_or(TagType::try_from_ext(
            path.as_ref()
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .to_lowercase()
                .as_str(),
        )?) {
            TagType::Id3v2 => Ok(Box::new({
                let mut t = Id3v2Tag::read_from_path(path)?;
                t.set_config(self.config.clone());
                t
            })),
            TagType::Mp4 => Ok(Box::new({
                let mut t = Mp4Tag::read_from_path(path)?;
                t.set_config(self.config.clone());
                t
            })),
            TagType::Flac => Ok(Box::new({
                let mut t = FlacTag::read_from_path(path)?;
                t.set_config(self.config.clone());
                t
            })),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MimeType {
    Png,
    Jpeg,
    Tiff,
    Bmp,
    Gif,
}

impl TryFrom<&str> for MimeType {
    type Error = crate::Error;
    fn try_from(inp: &str) -> crate::Result<Self> {
        Ok(match inp {
            "image/jpeg" => MimeType::Jpeg,
            "image/png" => MimeType::Png,
            "image/tiff" => MimeType::Tiff,
            "image/bmp" => MimeType::Bmp,
            "image/gif" => MimeType::Gif,
            _ => return Err(crate::Error::UnsupportedMimeType(inp.to_owned())),
        })
    }
}

impl From<MimeType> for &'static str {
    fn from(mt: MimeType) -> Self {
        match mt {
            MimeType::Jpeg => "image/jpeg",
            MimeType::Png => "image/png",
            MimeType::Tiff => "image/tiff",
            MimeType::Bmp => "image/bmp",
            MimeType::Gif => "image/gif",
        }
    }
}

impl From<MimeType> for String {
    fn from(mt: MimeType) -> Self {
        <MimeType as Into<&'static str>>::into(mt).to_owned()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Picture<'a> {
    pub data: &'a [u8],
    pub mime_type: MimeType,
}

impl<'a> Picture<'a> {
    pub fn new(data: &'a [u8], mime_type: MimeType) -> Self {
        Self { data, mime_type }
    }
}

/// A struct for representing an album for convinience.
#[derive(Debug)]
pub struct Album<'a> {
    pub title: &'a str,
    pub artist: Option<&'a str>,
    pub cover: Option<Picture<'a>>,
}

impl<'a> Album<'a> {
    pub fn with_title(title: &'a str) -> Self {
        Self {
            title: title,
            artist: None,
            cover: None,
        }
    }
    pub fn and_artist(mut self, artist: &'a str) -> Self {
        self.artist = Some(artist);
        self
    }
    pub fn and_cover(mut self, cover: Picture<'a>) -> Self {
        self.cover = Some(cover);
        self
    }
    pub fn with_all(title: &'a str, artist: &'a str, cover: Picture<'a>) -> Self {
        Self {
            title,
            artist: Some(artist),
            cover: Some(cover),
        }
    }
}

#[derive(Default)]
pub struct AnyTag<'a> {
    pub config: Config,
    pub title: Option<&'a str>,
    pub artists: Option<Vec<&'a str>>, // ? iterator
    pub year: Option<i32>,
    pub album_title: Option<&'a str>,
    pub album_artists: Option<Vec<&'a str>>, // ? iterator
    pub album_cover: Option<Picture<'a>>,
    pub track_number: Option<u16>,
    pub total_tracks: Option<u16>,
    pub disc_number: Option<u16>,
    pub total_discs: Option<u16>,
}

impl AnyTag<'_> {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    pub fn artists(&self) -> Option<&[&str]> {
        self.artists.as_deref()
    }
    pub fn year(&self) -> Option<i32> {
        self.year
    }
    pub fn album_title(&self) -> Option<&str> {
        self.album_title.as_deref()
    }
    pub fn album_artists(&self) -> Option<&[&str]> {
        self.album_artists.as_deref()
    }
    pub fn track_number(&self) -> Option<u16> {
        self.track_number
    }
    pub fn total_tracks(&self) -> Option<u16> {
        self.total_tracks
    }
    pub fn disc_number(&self) -> Option<u16> {
        self.track_number
    }
    pub fn total_discs(&self) -> Option<u16> {
        self.total_tracks
    }
}

impl AnyTag<'_> {
    pub fn artists_as_string(&self) -> Option<String> {
        self.artists()
            .map(|artists| artists.join(self.config.sep_artist))
    }
    pub fn album_artists_as_string(&self) -> Option<String> {
        self.album_artists()
            .map(|artists| artists.join(self.config.sep_artist))
    }
}

pub trait TagIo {
    fn read_from_path(path: &str) -> crate::Result<AnyTag>;
    fn write_to_path(path: &str) -> crate::Result<()>;
}

/// Implementors of this trait are able to read and write audio metadata.
///
/// Constructor methods e.g. `from_file` should be implemented separately.
pub trait AudioTag: AudioTagCommon {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, title: &str);
    fn remove_title(&mut self);

    fn artist(&self) -> Option<&str>;
    fn set_artist(&mut self, artist: &str);
    fn remove_artist(&mut self);

    fn artists(&self) -> Option<Vec<&str>> {
        if self.config().parse_multiple_artists {
            self.artist()
                .map(|a| a.split(self.config().sep_artist).collect::<Vec<&str>>())
        } else {
            self.artist().map(|v| vec![v])
        }
    }
    fn add_artist(&mut self, artist: &str) {
        self.set_artist(artist);
    }

    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn remove_year(&mut self);

    fn album(&self) -> Option<Album<'_>> {
        self.album_title().map(|title| Album {
            title,
            artist: self.album_artist(),
            cover: self.album_cover(),
        })
    }
    fn set_album(&mut self, album: Album) {
        self.set_album_title(&album.title);
        if let Some(artist) = album.artist {
            self.set_album_artist(&artist)
        } else {
            self.remove_album_artist()
        }
        if let Some(pic) = album.cover {
            self.set_album_cover(pic)
        } else {
            self.remove_album_cover()
        }
    }
    fn remove_album(&mut self) {
        self.remove_album_title();
        self.remove_album_artist();
        self.remove_album_cover();
    }

    fn album_title(&self) -> Option<&str>;
    fn set_album_title(&mut self, v: &str);
    fn remove_album_title(&mut self);

    fn album_artist(&self) -> Option<&str>;
    fn set_album_artist(&mut self, v: &str);
    fn remove_album_artist(&mut self);

    fn album_artists(&self) -> Option<Vec<&str>> {
        if self.config().parse_multiple_artists {
            self.album_artist()
                .map(|a| a.split(self.config().sep_artist).collect::<Vec<&str>>())
        } else {
            self.album_artist().map(|v| vec![v])
        }
    }
    fn add_album_artist(&mut self, artist: &str) {
        self.set_album_artist(artist);
    }

    fn album_cover(&self) -> Option<Picture>;
    fn set_album_cover(&mut self, cover: Picture);
    fn remove_album_cover(&mut self);

    fn track(&self) -> (Option<u16>, Option<u16>) {
        (self.track_number(), self.total_tracks())
    }
    fn set_track(&mut self, track: (u16, u16)) {
        self.set_track_number(track.0);
        self.set_total_tracks(track.1);
    }
    fn remove_track(&mut self) {
        self.remove_track_number();
        self.remove_total_tracks();
    }

    fn track_number(&self) -> Option<u16>;
    fn set_track_number(&mut self, track_number: u16);
    fn remove_track_number(&mut self);

    fn total_tracks(&self) -> Option<u16>;
    fn set_total_tracks(&mut self, total_track: u16);
    fn remove_total_tracks(&mut self);

    fn disc(&self) -> (Option<u16>, Option<u16>) {
        (self.disc_number(), self.total_discs())
    }
    fn set_disc(&mut self, disc: (u16, u16)) {
        self.set_disc_number(disc.0);
        self.set_total_discs(disc.1);
    }
    fn remove_disc(&mut self) {
        self.remove_disc_number();
        self.remove_total_discs();
    }

    fn disc_number(&self) -> Option<u16>;
    fn set_disc_number(&mut self, disc_number: u16);
    fn remove_disc_number(&mut self);

    fn total_discs(&self) -> Option<u16>;
    fn set_total_discs(&mut self, total_discs: u16);
    fn remove_total_discs(&mut self);

    fn write_to(&mut self, file: &mut File) -> crate::Result<()>;
    // cannot use impl AsRef<Path>
    fn write_to_path(&mut self, path: &str) -> crate::Result<()>;
}

pub trait AudioTagCommon {
    fn config(&self) -> &Config;
    fn set_config(&mut self, config: Config);
    fn into_anytag(&self) -> AnyTag<'_>;

    /// Convert the tag type, which can be lossy.
    fn into_tag(&self, tag_type: TagType) -> Box<dyn AudioTag> {
        match tag_type {
            TagType::Id3v2 => Box::new(Id3v2Tag::from(self.into_anytag())),
            TagType::Mp4 => Box::new(Mp4Tag::from(self.into_anytag())),
            TagType::Flac => Box::new(FlacTag::from(self.into_anytag())),
        }
    }
}

// pub trait IntoAnyTag {
//     fn into_anytag<'a>(&'a self) -> AnyTag<'a>;
//     fn into_tag<'a, T: From<AnyTag<'a>>>(&'a self) -> T {
//         self.into_anytag().into()
//     }
// }

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub enum PictureType {
//     Other,
//     Icon,
//     OtherIcon,
//     CoverFront,
//     CoverBack,
//     Leaflet,
//     Media,
//     LeadArtist,
//     Artist,
//     Conductor,
//     Band,
//     Composer,
//     Lyricist,
//     RecordingLocation,
//     DuringRecording,
//     DuringPerformance,
//     ScreenCapture,
//     BrightFish,
//     Illustration,
//     BandLogo,
//     PublisherLogo,
//     Undefined(u8),
// }
