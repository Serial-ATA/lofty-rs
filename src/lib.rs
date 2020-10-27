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
//! ## Example
//!
//! (Due to copyright restrictions I cannot upload the actual audio files here)
//!
//! ```ignore
//! use audiotags::Tag;
//! fn main() {
//!     const MP3: &'static str = "a.mp3";
//!     let mut tags = Tag::default().read_from_path(MP3).unwrap();
//!     // without this crate you would call id3::Tag::read_from_path()
//!     println!("Title: {:?}", tags.title());
//!     println!("Artist: {:?}", tags.artist());
//!     tags.set_album_artist("CINDERELLA PROJECT");
//!     let album = tags.album().unwrap();
//!     println!("Album title and artist: {:?}", (album.title, album.artist));
//!     println!("Track: {:?}", tags.track());
//!     tags.write_to_path(MP3).unwrap();
//!     // Title: Some("お願い！シンデレラ")
//!     // Artist: Some("高垣楓、城ヶ崎美嘉、小日向美穂、十時愛梨、川島瑞樹、日野茜、輿水幸子、佐久間まゆ、白坂小梅")
//!     // Album title and artist: ("THE IDOLM@STER CINDERELLA GIRLS ANIMATION PROJECT 01 Star!!", Some("CINDERELLA PROJECT"))
//!     // Track: (Some(2), Some(4))
//!     const M4A: &'static str = "b.m4a";
//!     let mut tags = Tag::default().read_from_path(M4A).unwrap();
//!     // without this crate you would call mp4ameta::Tag::read_from_path()
//!     println!("Title: {:?}", tags.title());
//!     println!("Artist: {:?}", tags.artist());
//!     let album = tags.album().unwrap();
//!     println!("Album title and artist: {:?}", (album.title, album.artist));
//!     tags.set_total_tracks(4);
//!     println!("Track: {:?}", tags.track());
//!     tags.write_to_path(M4A).unwrap();
//!     // Title: Some("ふわふわ時間")
//!     // Artist: Some("桜高軽音部 [平沢唯・秋山澪・田井中律・琴吹紬(CV:豊崎愛生、日笠陽子、佐藤聡美、寿美菜子)]")
//!     // Album title and artist: ("ふわふわ時間", Some("桜高軽音部 [平沢唯・秋山澪・田井中律・琴吹紬(CV:豊崎愛生、日笠陽子、佐藤聡美、寿美菜子)]"))
//!     // Track: (Some(1), Some(4))
//!     const FLAC: &'static str = "c.flac";
//!     let mut tags = Tag::default().read_from_path(FLAC).unwrap();
//!     // without this crate you would call metaflac::Tag::read_from_path()
//!     println!("Title: {:?}", tags.title());
//!     println!("Artist: {:?}", tags.artist());
//!     let album = tags.album().unwrap();
//!     println!("Album title and artist: {:?}", (album.title, album.artist));
//!     tags.set_year(2017);
//!     println!("Year: {:?}", tags.year());
//!     tags.write_to_path(FLAC).unwrap();
//!     // Title: Some("意味/無/ジュニーク・ニコール")
//!     // Artist: Some("岡部啓一")
//!     // Album title and artist: ("NieR:Automata Original Soundtrack", Some("SQUARE ENIX"))
//!     // Year: Some(2017)
//! }
//! ```
//!
//! You can convert between different tag types:
//!
//! ```ignore
//! use audiotags::{Tag, TagType};
//!
//! fn main() {
//!     // we have an mp3 and an m4a file
//!     const MP3_FILE: &'static str = "assets/a.mp3";
//!     const M4A_FILE: &'static str = "assets/a.m4a";
//!     // read tag from the mp3 file. Using `default()` so that the type of tag is guessed from the file extension
//!     let mut mp3tag = Tag::default().read_from_path(MP3_FILE).unwrap();
//!     // set the title
//!     mp3tag.set_title("title from mp3 file");
//!     // we can convert it to an mp4 tag and save it to an m4a file.
//!     let mut mp4tag = mp3tag.into_tag(TagType::Mp4);
//!     mp4tag.write_to_path(M4A_FILE).unwrap();
//!
//!     // reload the tag from the m4a file; this time specifying the tag type (you can also use `default()`)
//!     let mp4tag_reload = Tag::with_tag_type(TagType::Mp4)
//!         .read_from_path(M4A_FILE)
//!         .unwrap();
//!     // the tag originated from an mp3 file is successfully written to an m4a file!
//!     assert_eq!(mp4tag_reload.title(), Some("title from mp3 file"));
//! }
//!
//! ```
//!
//! ## Supported Formats
//!
//! | File Fomat    | Metadata Format       | backend                                                     |
//! | ------------- | --------------------- | ----------------------------------------------------------- |
//! | `mp3`         | id3v2.4               | [**id3**](https://github.com/polyfloyd/rust-id3)            |
//! | `m4a/mp4/...` | MPEG-4 audio metadata | [**mp4ameta**](https://github.com/Saecki/rust-mp4ameta)     |
//! | `flac`        | Vorbis comment        | [**metaflac**](https://github.com/jameshurst/rust-metaflac) |
//!
//! ## Getters and Setters
//!
//! ```ignore
//! pub trait AudioTagIo {
//!     fn title(&self) -> Option<&str>;
//!     fn set_title(&mut self, title: &str);
//!     fn remove_title(&mut self);
//!     fn artist(&self) -> Option<&str>;
//!     fn remove_artist(&mut self);
//!     fn set_artist(&mut self, artist: &str);
//!     fn year(&self) -> Option<i32>;
//!     fn set_year(&mut self, year: i32);
//!     fn remove_year(&mut self);
//!     fn album(&self) -> Option<Album>;
//!     fn remove_album(&mut self);
//!     fn album_title(&self) -> Option<&str>;
//!     fn remove_album_title(&mut self);
//!     fn album_artist(&self) -> Option<&str>;
//!     fn remove_album_artist(&mut self);
//!     fn album_cover(&self) -> Option<Picture>;
//!     fn remove_album_cover(&mut self);
//!     fn set_album(&mut self, album: Album);
//!     fn set_album_title(&mut self, v: &str);
//!     fn set_album_artist(&mut self, v: &str);
//!     fn set_album_cover(&mut self, cover: Picture);
//!     fn track(&self) -> (Option<u16>, Option<u16>);
//!     fn set_track(&mut self, track: (u16, u16));
//!     fn remove_track(&mut self);
//!     fn track_number(&self) -> Option<u16>;
//!     fn set_track_number(&mut self, track_number: u16);
//!     fn remove_track_number(&mut self);
//!     fn total_tracks(&self) -> Option<u16>;
//!     fn set_total_tracks(&mut self, total_track: u16);
//!     fn remove_total_tracks(&mut self);
//!     fn disc(&self) -> (Option<u16>, Option<u16>);
//!     fn set_disc(&mut self, disc: (u16, u16));
//!     fn remove_disc(&mut self);
//!     fn disc_number(&self) -> Option<u16>;
//!     fn set_disc_number(&mut self, disc_number: u16);
//!     fn remove_disc_number(&mut self);
//!     fn total_discs(&self) -> Option<u16>;
//!     fn set_total_discs(&mut self, total_discs: u16);
//!     fn remove_total_discs(&mut self);
//!     fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError>;
//!     // cannot use impl AsRef<Path>
//!     fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError>;
//! }
//! ```

mod id3_tag;
pub use id3_tag::Id3v2Tag;
mod flac_tag;
mod mp4_tag;
pub use flac_tag::FlacTag;
pub use mp4_tag::Mp4Tag;

pub mod error;
pub use error::{Error, Result};

use std::convert::From;
use std::fs::File;
use std::path::Path;

use beef::lean::Cow;

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
}

impl Tag {
    pub fn with_tag_type(tag_type: TagType) -> Self {
        Self {
            tag_type: Some(tag_type),
        }
    }

    pub fn read_from_path(&self, path: impl AsRef<Path>) -> crate::Result<Box<dyn AudioTagIo>> {
        match self.tag_type.unwrap_or(TagType::try_from_ext(
            path.as_ref()
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .to_lowercase()
                .as_str(),
        )?) {
            TagType::Id3v2 => Ok(Box::new(Id3v2Tag::read_from_path(path)?)),
            TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from_path(path)?)),
            TagType::Flac => Ok(Box::new(FlacTag::read_from_path(path)?)),
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
    pub data: Cow<'a, [u8]>,
    pub mime_type: MimeType,
}

impl<'a> Picture<'a> {
    pub fn new(data: &'a [u8], mime_type: MimeType) -> Self {
        Self {
            data: Cow::borrowed(data),
            mime_type,
        }
    }
}

/// A struct for representing an album for convinience.
#[derive(Debug)]
pub struct Album<'a> {
    pub title: Cow<'a, str>,
    pub artist: Option<Cow<'a, str>>,
    pub cover: Option<Picture<'a>>,
}

impl<'a> Album<'a> {
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Cow::owned(title.into()),
            artist: None,
            cover: None,
        }
    }
    pub fn and_artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = Some(Cow::owned(artist.into()));
        self
    }
    pub fn and_cover(mut self, cover: Picture<'a>) -> Self {
        self.cover = Some(cover);
        self
    }
    pub fn with_all(
        title: impl Into<String>,
        artist: impl Into<String>,
        cover: Picture<'a>,
    ) -> Self {
        Self {
            title: Cow::owned(title.into()),
            artist: Some(Cow::owned(artist.into())),
            cover: Some(cover),
        }
    }
}

const SEP_ARTIST: &'static str = ";";

#[derive(Default)]
pub struct AnyTag<'a> {
    pub title: Option<Cow<'a, str>>,
    pub artists: Option<Vec<Cow<'a, str>>>, // ? iterator
    pub year: Option<i32>,
    pub album_title: Option<Cow<'a, str>>,
    pub album_artists: Option<Vec<Cow<'a, str>>>, // ? iterator
    pub album_cover: Option<Picture<'a>>,
    pub track_number: Option<u16>,
    pub total_tracks: Option<u16>,
    pub disc_number: Option<u16>,
    pub total_discs: Option<u16>,
}

impl<'a> AnyTag<'a> {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    pub fn artists(&self) -> Option<&[Cow<str>]> {
        self.artists.as_deref()
    }
    pub fn year(&self) -> Option<i32> {
        self.year
    }
    pub fn album_title(&self) -> Option<&str> {
        self.album_title.as_deref()
    }
    pub fn album_artists(&self) -> Option<&[Cow<str>]> {
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

pub trait TagIo {
    fn read_from_path(path: &str) -> crate::Result<AnyTag>;
    fn write_to_path(path: &str) -> crate::Result<()>;
}

// impl<'a> AnyTag<'a> {
//     fn read_from_path(path: &str, tag_type: TagType) -> StdResult<Self, BoxedError> {
//         match tag_type {
//             TagType::Id3v2 => Ok(Id3v2Tag::read_from_path(path)?.into()),
//             _ => Err(Box::new(Error::UnsupportedFormat(".".to_owned()))),
//         }
//     }
// }

/// Implementors of this trait are able to read and write audio metadata.
///
/// Constructor methods e.g. `from_file` should be implemented separately.
pub trait AudioTagIo {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, title: &str);
    fn remove_title(&mut self);

    fn artist(&self) -> Option<&str>;
    fn set_artist(&mut self, artist: &str);
    fn remove_artist(&mut self);

    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn remove_year(&mut self);

    fn album(&self) -> Option<Album<'_>> {
        self.album_title().map(|title| Album {
            title: Cow::borrowed(title),
            artist: self.album_artist().map(Cow::borrowed),
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

    fn into_anytag(&self) -> AnyTag<'_>;

    /// Convert the tag type, which can be lossy.
    fn into_tag(&self, tag_type: TagType) -> Box<dyn AudioTagIo> {
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

// pub trait IntoTag: AudioTagIo {

//     fn into_tag<'a, T>(&'a self) -> T
//     where T: From<AnyTag<'a> {
//         self.into_anytag().into()
//     }
// }

// impl AnyTag {
//     pub fn artists_as_string(&self, sep: &str) -> Option<String> {
//         self.artists().map(|artists| artists.join(sep))
//     }
//     pub fn album_artists_as_string(&self, sep: &str) -> Option<String> {
//         self.album_artists().map(|artists| artists.join(sep))
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
