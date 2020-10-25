//! This crate makes it easier to parse tags/metadata in audio files of different file types.
//!
//! This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3 and m4a files with a single function: `audiotags::from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** crates in order to parse metadata in different file foramts.
//!
//! ## Example
//!
//! ```ignore
//! use audiotags;
//!
//! fn main() {
//!     const MP3: &'static str = "a.mp3";
//!     let mut tags = audiotags::from_path(MP3).unwrap();
//!     // without this crate you would call id3::Tag::from_path()
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
//!     let mut tags = audiotags::from_path(M4A).unwrap();
//!     // without this crate you would call mp4ameta::Tag::from_path()
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
use mp4ameta;
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
pub fn from_path(path: impl AsRef<Path>) -> Result<Box<dyn AudioTagsIo>, BoxedError> {
    match path
        .as_ref()
        .extension()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .to_lowercase()
        .as_str()
    {
        "mp3" => Ok(Box::new(Id3Tags::from_path(path)?)),
        "m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Box::new(M4aTags::from_path(path)?)),
        p @ _ => Err(Box::new(Error::UnsupportedFormat(p.to_owned()))),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PictureType {
    Png,
    Jpeg,
    Tiff,
    Bmp,
    Gif,
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub data: Vec<u8>,
    pub picture_type: PictureType,
}

impl Picture {
    pub fn try_with_mime(data: Vec<u8>, mime: &str) -> Result<Self, ()> {
        let picture_type = match mime {
            "image/jpeg" => PictureType::Jpeg,
            "image/png" => PictureType::Png,
            "image/tiff" => PictureType::Tiff,
            "image/bmp" => PictureType::Bmp,
            "image/gif" => PictureType::Gif,
            _ => return Err(()),
        };
        Ok(Self { data, picture_type })
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
    fn artist(&self) -> Option<&str>;
    fn set_artist(&mut self, artist: &str);
    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn album(&self) -> Option<Album>;
    fn album_title(&self) -> Option<&str>;
    fn album_artist(&self) -> Option<&str>;
    fn album_cover(&self) -> Option<Picture>;
    fn set_album(&mut self, album: Album);
    fn set_album_title(&mut self, v: &str);
    fn set_album_artist(&mut self, v: &str);
    fn set_album_cover(&mut self, cover: Picture);
    fn track(&self) -> (Option<u16>, Option<u16>);
    fn set_track_number(&mut self, track_number: u16);
    fn set_total_tracks(&mut self, total_track: u16);
    fn disc(&self) -> (Option<u16>, Option<u16>);
    fn set_disc_number(&mut self, disc_number: u16);
    fn set_total_discs(&mut self, total_discs: u16);
    fn write_to(&self, file: &File) -> Result<(), BoxedError>;
    // cannot use impl AsRef<Path>
    fn write_to_path(&self, path: &str) -> Result<(), BoxedError>;
}

pub struct Id3Tags {
    inner: id3::Tag,
}

impl Id3Tags {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
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
    fn artist(&self) -> Option<&str> {
        self.inner.artist()
    }
    fn set_artist(&mut self, artist: &str) {
        self.inner.set_title(artist)
    }
    fn year(&self) -> Option<i32> {
        self.inner.year()
    }
    fn set_year(&mut self, year: i32) {
        self.inner.set_year(year)
    }
    fn album(&self) -> Option<Album> {
        self.inner.album().map(|title| Album {
            title: title.to_owned(),
            artist: self.inner.album_artist().map(|x| x.to_owned()),
            cover: self.album_cover(),
        })
    }
    fn album_title(&self) -> Option<&str> {
        self.inner.album()
    }
    fn album_artist(&self) -> Option<&str> {
        self.inner.album_artist()
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
    fn set_album(&mut self, album: Album) {
        self.inner.set_album(album.title);
        if let Some(artist) = album.artist {
            self.inner.set_album_artist(artist)
        } else {
            self.inner.remove_album_artist()
        }
        if let Some(pic) = album.cover {
            self.set_album_cover(pic)
        } else {
            self.inner
                .remove_picture_by_type(id3::frame::PictureType::CoverFront);
        }
    }
    fn set_album_title(&mut self, v: &str) {
        self.inner.set_album(v)
    }
    fn set_album_artist(&mut self, v: &str) {
        self.inner.set_album_artist(v)
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.inner
            .remove_picture_by_type(id3::frame::PictureType::CoverFront);
        self.inner.add_picture(match cover.picture_type {
            PictureType::Jpeg => id3::frame::Picture {
                mime_type: "jpeg".to_owned(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_owned(),
                data: cover.data,
            },
            PictureType::Png => id3::frame::Picture {
                mime_type: "png".to_owned(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_owned(),
                data: cover.data,
            },
            PictureType::Tiff => id3::frame::Picture {
                mime_type: "tiff".to_owned(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_owned(),
                data: cover.data,
            },
            PictureType::Bmp => id3::frame::Picture {
                mime_type: "bmp".to_owned(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_owned(),
                data: cover.data,
            },
            PictureType::Gif => id3::frame::Picture {
                mime_type: "gif".to_owned(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "".to_owned(),
                data: cover.data,
            },
        });
    }
    fn track(&self) -> (Option<u16>, Option<u16>) {
        (
            self.inner.track().map(|x| x as u16),
            self.inner.total_tracks().map(|x| x as u16),
        )
    }
    fn set_track_number(&mut self, track: u16) {
        self.inner.set_track(track as u32);
    }
    fn set_total_tracks(&mut self, total_track: u16) {
        self.inner.set_total_tracks(total_track as u32);
    }
    fn disc(&self) -> (Option<u16>, Option<u16>) {
        (
            self.inner.disc().map(|x| x as u16),
            self.inner.total_discs().map(|x| x as u16),
        )
    }
    fn set_disc_number(&mut self, disc_number: u16) {
        self.inner.set_disc(disc_number as u32)
    }
    fn set_total_discs(&mut self, total_discs: u16) {
        self.inner.set_total_discs(total_discs as u32)
    }
    fn write_to(&self, file: &File) -> Result<(), BoxedError> {
        self.inner.write_to(file, id3::Version::Id3v24)?;
        Ok(())
    }
    fn write_to_path(&self, path: &str) -> Result<(), BoxedError> {
        self.inner.write_to_path(path, id3::Version::Id3v24)?;
        Ok(())
    }
}

pub struct M4aTags {
    inner: mp4ameta::Tag,
}

impl M4aTags {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
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
    fn album(&self) -> Option<Album> {
        self.inner.album().map(|title| Album {
            title: title.to_owned(),
            artist: self.inner.album_artist().map(|x| x.to_owned()),
            cover: self.album_cover(),
        })
    }
    fn album_cover(&self) -> Option<Picture> {
        use mp4ameta::Data::*;
        if let Some(Some(pic)) = self.inner.artwork().map(|data| match data {
            Jpeg(d) => Some(Picture {
                data: d.clone(),
                picture_type: PictureType::Jpeg,
            }),
            Png(d) => Some(Picture {
                data: d.clone(),
                picture_type: PictureType::Png,
            }),
            _ => None,
        }) {
            Some(pic)
        } else {
            None
        }
    }
    fn album_title(&self) -> Option<&str> {
        self.inner.album()
    }
    fn album_artist(&self) -> Option<&str> {
        self.inner.album_artist()
    }
    fn set_album(&mut self, album: Album) {
        self.inner.set_album(album.title);
        if let Some(artist) = album.artist {
            self.inner.set_album_artist(artist)
        } else {
            // self.inner.remove_album_artist(artist)
        }
        if let Some(pic) = album.cover {
            self.set_album_cover(pic)
        } else {
            self.inner.remove_artwork();
        }
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.inner.remove_artwork();
        self.inner.add_artwork(match cover.picture_type {
            PictureType::Png => mp4ameta::Data::Png(cover.data),
            PictureType::Jpeg => mp4ameta::Data::Jpeg(cover.data),
            _ => panic!("Only png and jpeg are supported in m4a"),
        });
    }
    fn set_album_title(&mut self, v: &str) {
        self.inner.set_album(v)
    }
    fn set_album_artist(&mut self, v: &str) {
        self.inner.set_album_artist(v)
    }
    fn track(&self) -> (Option<u16>, Option<u16>) {
        self.inner.track()
    }
    fn set_track_number(&mut self, track: u16) {
        self.inner.set_track_number(track);
    }
    fn set_total_tracks(&mut self, total_track: u16) {
        self.inner.set_total_tracks(total_track);
    }
    fn disc(&self) -> (Option<u16>, Option<u16>) {
        self.inner.disc()
    }
    fn set_disc_number(&mut self, disc_number: u16) {
        self.inner.set_disc_number(disc_number)
    }
    fn set_total_discs(&mut self, total_discs: u16) {
        self.inner.set_total_discs(total_discs)
    }
    fn write_to(&self, file: &File) -> Result<(), BoxedError> {
        self.inner.write_to(file)?;
        Ok(())
    }
    fn write_to_path(&self, path: &str) -> Result<(), BoxedError> {
        self.inner.write_to_path(path)?;
        Ok(())
    }
}
