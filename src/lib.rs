//! This crate makes it easier to parse tags/metadata in audio files of different file types.
//!
//! This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3 and m4a files with a single function: `audiotags::read_from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** crates in order to parse metadata in different file foramts.
//!
//! ## Example
//!
//! ```ignore
//! use audiotags;
//!
//! fn main() {
//!     const MP3: &'static str = "a.mp3";
//!     let mut tags = audiotags::read_from_path(MP3).unwrap();
//!     // without this crate you would call id3::Tag::read_from_path()
//!     println!("Title: {:?}", tags.title());
//!     println!("Artist: {:?}", tags.artist());
//!     tags.set_album_artist("CINDERELLA PROJECT");
//!     let album = tags.album().unwrap();
//!     println!("Album title and artist: {:?}", (album.title, album.artist));
//!     println!("Track: {:?}", tags.track());
//!     tags.write_to_path(MP3).unwrap();
//! // Title: Some("お願い！シンデレラ")
//! // Artist: Some("高垣楓、城ヶ崎美嘉、小日向美穂、十時愛梨、川島瑞樹、日野茜、輿水幸子、佐久間まゆ、白坂小梅")
//! // Album title and artist: ("THE IDOLM@STER CINDERELLA GIRLS ANIMATION PROJECT 01 Star!!", Some("CINDERELLA PROJECT"))
//! // Track: (Some(2), Some(4))
//!
//!     const M4A: &'static str = "b.m4a";
//!     let mut tags = audiotags::read_from_path(M4A).unwrap();
//!     // without this crate you would call mp4ameta::Tag::read_from_path()
//!     println!("Title: {:?}", tags.title());
//!     println!("Artist: {:?}", tags.artist());
//!     let album = tags.album().unwrap();
//!     println!("Album title and artist: {:?}", (album.title, album.artist));
//!     tags.set_total_tracks(4);
//!     println!("Track: {:?}", tags.track());
//!     tags.write_to_path(M4A).unwrap();
//! // Title: Some("ふわふわ時間")
//! // Artist: Some("桜高軽音部 [平沢唯・秋山澪・田井中律・琴吹紬(CV:豊崎愛生、日笠陽子、佐藤聡美、寿美菜子)]")
//! // Album title and artist: ("ふわふわ時間", Some("桜高軽音部 [平沢唯・秋山澪・田井中律・琴吹紬(CV:豊崎愛生、日笠陽子、佐藤聡美、寿美菜子)]"))
//! // Track: (Some(1), Some(4))
//! }
//! ```

use id3;
use metaflac;
use mp4ameta;
use std::collections::HashMap;
use std::convert::From;
use std::fs::File;
use std::path::Path;
use strum::Display;

type BoxedError = Box<dyn std::error::Error>;

#[derive(Debug, Display)]
pub enum Error {
    UnsupportedFormat(String),
}

impl std::error::Error for Error {}

/// Guesses the audio metadata handler from the file extension, and returns the `Box`ed IO handler.
pub fn read_from_path(path: impl AsRef<Path>) -> Result<Box<dyn AudioTagsIo>, BoxedError> {
    match path
        .as_ref()
        .extension()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .to_lowercase()
        .as_str()
    {
        "mp3" => Ok(Box::new(Id3Tags::read_from_path(path)?)),
        "m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => {
            Ok(Box::new(M4aTags::read_from_path(path)?))
        }
        "flac" => Ok(Box::new(FlacTags::read_from_path(path)?)),
        p @ _ => Err(Box::new(Error::UnsupportedFormat(p.to_owned()))),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MimeType {
    Png,
    Jpeg,
    Tiff,
    Bmp,
    Gif,
}

impl From<MimeType> for String {
    fn from(mt: MimeType) -> Self {
        match mt {
            MimeType::Jpeg => "image/jpeg".to_owned(),
            MimeType::Png => "image/png".to_owned(),
            MimeType::Tiff => "image/tiff".to_owned(),
            MimeType::Bmp => "image/bmp".to_owned(),
            MimeType::Gif => "image/gif".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub data: Vec<u8>,
    pub mime_type: MimeType,
}

impl Picture {
    pub fn try_with_mime(data: Vec<u8>, mime: &str) -> Result<Self, ()> {
        let mime_type = match mime {
            "image/jpeg" => MimeType::Jpeg,
            "image/png" => MimeType::Png,
            "image/tiff" => MimeType::Tiff,
            "image/bmp" => MimeType::Bmp,
            "image/gif" => MimeType::Gif,
            _ => return Err(()),
        };
        Ok(Self { data, mime_type })
    }
}

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub artist: Option<String>,
    pub cover: Option<Picture>,
}

/// Implementors of this trait are able to read and write audio metadata.
///
/// Constructor methods e.g. `from_file` should be implemented separately.
pub trait AudioTagsIo {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, title: &str);
    fn remove_title(&mut self);

    fn artist(&self) -> Option<&str>;
    fn set_artist(&mut self, artist: &str);
    fn remove_artist(&mut self);

    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn remove_year(&mut self);

    fn album(&self) -> Option<Album> {
        self.album_title().map(|title| Album {
            title: title.to_owned(),
            artist: self.album_artist().map(|x| x.to_owned()),
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

    fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError>;
    // cannot use impl AsRef<Path>
    fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError>;
}

pub struct Id3Tags {
    inner: id3::Tag,
}

impl Id3Tags {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
        Ok(Self {
            inner: id3::Tag::read_from_path(path)?,
        })
    }
}

impl AudioTagsIo for Id3Tags {
    fn title(&self) -> Option<&str> {
        self.inner.title()
    }
    fn set_title(&mut self, title: &str) {
        self.inner.set_title(title)
    }
    fn remove_title(&mut self) {
        self.inner.remove_title();
    }

    fn artist(&self) -> Option<&str> {
        self.inner.artist()
    }
    fn set_artist(&mut self, artist: &str) {
        self.inner.set_artist(artist)
    }
    fn remove_artist(&mut self) {
        self.inner.remove_artist();
    }

    fn year(&self) -> Option<i32> {
        self.inner.year()
    }
    fn set_year(&mut self, year: i32) {
        self.inner.set_year(year)
    }
    fn remove_year(&mut self) {
        self.inner.remove("TYER")
        // self.inner.remove_year(); // TODO
    }

    fn album_title(&self) -> Option<&str> {
        self.inner.album()
    }
    fn set_album_title(&mut self, v: &str) {
        self.inner.set_album(v)
    }
    fn remove_album_title(&mut self) {
        self.inner.remove_album();
    }

    fn album_artist(&self) -> Option<&str> {
        self.inner.album_artist()
    }
    fn set_album_artist(&mut self, v: &str) {
        self.inner.set_album_artist(v)
    }
    fn remove_album_artist(&mut self) {
        self.inner.remove_album_artist();
    }

    fn album_cover(&self) -> Option<Picture> {
        if let Some(Ok(pic)) = self
            .inner
            .pictures()
            .filter(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
            .next()
            .map(|pic| Picture::try_with_mime(pic.data.clone(), &pic.mime_type))
        {
            Some(pic)
        } else {
            None
        }
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.remove_album_cover();
        self.inner.add_picture(id3::frame::Picture {
            mime_type: String::from(cover.mime_type),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "".to_owned(),
            data: cover.data,
        });
    }
    fn remove_album_cover(&mut self) {
        self.inner
            .remove_picture_by_type(id3::frame::PictureType::CoverFront);
    }

    fn track_number(&self) -> Option<u16> {
        self.inner.track().map(|x| x as u16)
    }
    fn set_track_number(&mut self, track: u16) {
        self.inner.set_track(track as u32);
    }
    fn remove_track_number(&mut self) {
        self.inner.remove_track();
    }

    fn total_tracks(&self) -> Option<u16> {
        self.inner.total_tracks().map(|x| x as u16)
    }
    fn set_total_tracks(&mut self, total_track: u16) {
        self.inner.set_total_tracks(total_track as u32);
    }
    fn remove_total_tracks(&mut self) {
        self.inner.remove_total_tracks();
    }

    fn disc_number(&self) -> Option<u16> {
        self.inner.disc().map(|x| x as u16)
    }
    fn set_disc_number(&mut self, disc_number: u16) {
        self.inner.set_disc(disc_number as u32)
    }
    fn remove_disc_number(&mut self) {
        self.inner.remove_disc();
    }

    fn total_discs(&self) -> Option<u16> {
        self.inner.total_discs().map(|x| x as u16)
    }
    fn set_total_discs(&mut self, total_discs: u16) {
        self.inner.set_total_discs(total_discs as u32)
    }
    fn remove_total_discs(&mut self) {
        self.inner.remove_total_discs();
    }

    fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError> {
        self.inner.write_to(file, id3::Version::Id3v24)?;
        Ok(())
    }
    fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError> {
        self.inner.write_to_path(path, id3::Version::Id3v24)?;
        Ok(())
    }
}

pub struct M4aTags {
    inner: mp4ameta::Tag,
}

impl M4aTags {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
        Ok(Self {
            inner: mp4ameta::Tag::read_from_path(path)?,
        })
    }
}

impl AudioTagsIo for M4aTags {
    fn title(&self) -> Option<&str> {
        self.inner.title()
    }
    fn set_title(&mut self, title: &str) {
        self.inner.set_title(title)
    }

    fn artist(&self) -> Option<&str> {
        self.inner.artist()
    }
    fn set_artist(&mut self, artist: &str) {
        self.inner.set_title(artist)
    }

    fn year(&self) -> Option<i32> {
        match self.inner.year() {
            Some(year) => str::parse(year).ok(),
            None => None,
        }
    }
    fn set_year(&mut self, year: i32) {
        self.inner.set_year(year.to_string())
    }

    fn album_title(&self) -> Option<&str> {
        self.inner.album()
    }
    fn set_album_title(&mut self, v: &str) {
        self.inner.set_album(v)
    }

    fn album_artist(&self) -> Option<&str> {
        self.inner.album_artist()
    }
    fn set_album_artist(&mut self, v: &str) {
        self.inner.set_album_artist(v)
    }

    fn album_cover(&self) -> Option<Picture> {
        use mp4ameta::Data::*;
        if let Some(Some(pic)) = self.inner.artwork().map(|data| match data {
            Jpeg(d) => Some(Picture {
                data: d.clone(),
                mime_type: MimeType::Jpeg,
            }),
            Png(d) => Some(Picture {
                data: d.clone(),
                mime_type: MimeType::Png,
            }),
            _ => None,
        }) {
            Some(pic)
        } else {
            None
        }
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.remove_album_cover();
        self.inner.add_artwork(match cover.mime_type {
            MimeType::Png => mp4ameta::Data::Png(cover.data),
            MimeType::Jpeg => mp4ameta::Data::Jpeg(cover.data),
            _ => panic!("Only png and jpeg are supported in m4a"),
        });
    }

    fn track_number(&self) -> Option<u16> {
        self.inner.track_number()
    }
    fn total_tracks(&self) -> Option<u16> {
        self.inner.total_tracks()
    }
    fn set_track_number(&mut self, track: u16) {
        self.inner.set_track_number(track);
    }
    fn set_total_tracks(&mut self, total_track: u16) {
        self.inner.set_total_tracks(total_track);
    }

    fn disc_number(&self) -> Option<u16> {
        self.inner.disc_number()
    }
    fn total_discs(&self) -> Option<u16> {
        self.inner.total_discs()
    }
    fn set_disc_number(&mut self, disc_number: u16) {
        self.inner.set_disc_number(disc_number)
    }
    fn set_total_discs(&mut self, total_discs: u16) {
        self.inner.set_total_discs(total_discs)
    }

    fn remove_title(&mut self) {
        self.inner.remove_title();
    }
    fn remove_artist(&mut self) {
        //self.inner.
        // self.inner.remove_artist(); // TODO:
    }
    fn remove_year(&mut self) {
        self.inner.remove_year();
    }
    fn remove_album_title(&mut self) {
        self.inner.remove_album();
    }
    fn remove_album_artist(&mut self) {
        // self.inner.remove_album_artist(); // TODO:
    }
    fn remove_album_cover(&mut self) {
        self.inner.remove_artwork();
    }
    fn remove_track(&mut self) {
        self.inner.remove_track();
    }
    fn remove_track_number(&mut self) {
        // self.inner.remove_data(mp4ameta::atom::TRACK_NUMBER); not correct
        // TODO: self.inner.remove_track_number();
    }
    fn remove_total_tracks(&mut self) {
        // TODO: self.inner.remove_total_tracks();
    }
    fn remove_disc(&mut self) {
        self.inner.remove_disc();
    }
    fn remove_disc_number(&mut self) {
        // self.inner.remove_data(mp4ameta::atom::DISC_NUMBER); not correct
        // TODO: self.inner.remove_disc_number();
    }
    fn remove_total_discs(&mut self) {
        // TODO: self.inner.remove_total_discs();
    }

    fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError> {
        self.inner.write_to(file)?;
        Ok(())
    }
    fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError> {
        self.inner.write_to_path(path)?;
        Ok(())
    }
}

struct FlacTags {
    inner: metaflac::Tag,
}

impl FlacTags {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
        Ok(Self {
            inner: metaflac::Tag::read_from_path(path)?,
        })
    }
    pub fn get_first(&self, key: &str) -> Option<&str> {
        if let Some(Some(v)) = self.inner.vorbis_comments().map(|c| c.get(key)) {
            if !v.is_empty() {
                Some(v[0].as_str())
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn set_first(&mut self, key: &str, val: &str) {
        self.inner.vorbis_comments_mut().set(key, vec![val]);
    }
    pub fn remove(&mut self, k: &str) {
        self.inner.vorbis_comments_mut().comments.remove(k);
    }
}

impl AudioTagsIo for FlacTags {
    fn title(&self) -> Option<&str> {
        self.get_first("TITLE")
    }
    fn set_title(&mut self, title: &str) {
        self.set_first("TITLE", title);
    }

    fn artist(&self) -> Option<&str> {
        self.get_first("ARTIST")
    }
    fn set_artist(&mut self, artist: &str) {
        self.set_first("ARTIST", artist)
    }

    fn year(&self) -> Option<i32> {
        if let Some(Ok(y)) = self
            .get_first("DATE")
            .map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
        {
            Some(y)
        } else if let Some(Ok(y)) = self.get_first("YEAR").map(|s| s.parse::<i32>()) {
            Some(y)
        } else {
            None
        }
    }
    fn set_year(&mut self, year: i32) {
        self.set_first("DATE", &year.to_string());
        self.set_first("YEAR", &year.to_string());
    }

    fn album_title(&self) -> Option<&str> {
        self.get_first("ALBUM")
    }
    fn set_album_title(&mut self, title: &str) {
        self.set_first("ALBUM", title)
    }

    fn album_artist(&self) -> Option<&str> {
        self.get_first("ALBUMARTIST")
    }
    fn set_album_artist(&mut self, v: &str) {
        self.set_first("ALBUMARTIST", v)
    }

    fn album_cover(&self) -> Option<Picture> {
        if let Some(Ok(pic)) = self
            .inner
            .pictures()
            .filter(|&pic| matches!(pic.picture_type, metaflac::block::PictureType::CoverFront))
            .next()
            .map(|pic| Picture::try_with_mime(pic.data.clone(), &pic.mime_type))
        {
            Some(pic)
        } else {
            None
        }
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.remove_album_cover();
        let mime = String::from(cover.mime_type);
        let picture_type = metaflac::block::PictureType::CoverFront;
        self.inner.add_picture(mime, picture_type, cover.data);
    }

    fn track_number(&self) -> Option<u16> {
        if let Some(Ok(n)) = self.get_first("TRACKNUMBER").map(|x| x.parse::<u16>()) {
            Some(n)
        } else {
            None
        }
    }
    fn set_track_number(&mut self, v: u16) {
        self.set_first("TRACKNUMBER", &v.to_string())
    }

    // ! not standard
    fn total_tracks(&self) -> Option<u16> {
        if let Some(Ok(n)) = self.get_first("TOTALTRACKS").map(|x| x.parse::<u16>()) {
            Some(n)
        } else {
            None
        }
    }
    fn set_total_tracks(&mut self, v: u16) {
        self.set_first("TOTALTRACKS", &v.to_string())
    }

    fn disc_number(&self) -> Option<u16> {
        if let Some(Ok(n)) = self.get_first("DISCNUMBER").map(|x| x.parse::<u16>()) {
            Some(n)
        } else {
            None
        }
    }
    fn set_disc_number(&mut self, v: u16) {
        self.set_first("DISCNUMBER", &v.to_string())
    }

    // ! not standard
    fn total_discs(&self) -> Option<u16> {
        if let Some(Ok(n)) = self.get_first("TOTALDISCS").map(|x| x.parse::<u16>()) {
            Some(n)
        } else {
            None
        }
    }
    fn set_total_discs(&mut self, v: u16) {
        self.set_first("TOTALDISCS", &v.to_string())
    }

    fn remove_title(&mut self) {
        self.remove("TITLE");
    }
    fn remove_artist(&mut self) {
        self.remove("ARTIST");
    }
    fn remove_year(&mut self) {
        self.remove("YEAR");
        self.remove("DATE");
    }
    fn remove_album_title(&mut self) {
        self.remove("ALBUM");
    }
    fn remove_album_artist(&mut self) {
        self.remove("ALBUMARTIST");
    }
    fn remove_album_cover(&mut self) {
        self.inner
            .remove_picture_type(metaflac::block::PictureType::CoverFront)
    }
    fn remove_track_number(&mut self) {
        self.remove("TRACKNUMBER");
    }
    fn remove_total_tracks(&mut self) {
        self.remove("TOTALTRACKS");
    }
    fn remove_disc_number(&mut self) {
        self.remove("DISCNUMBER");
    }
    fn remove_total_discs(&mut self) {
        self.remove("TOTALDISCS");
    }
    fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError> {
        self.inner.write_to(file)?;
        Ok(())
    }
    fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError> {
        self.inner.write_to_path(path)?;
        Ok(())
    }
}
