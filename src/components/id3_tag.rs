use crate::*;
use id3;

pub use id3::Tag as Id3v2InnerTag;

impl_tag!(Id3v2Tag, Id3v2InnerTag, TagType::Id3v2);

impl<'a> From<&'a Id3v2Tag> for AnyTag<'a> {
    fn from(inp: &'a Id3v2Tag) -> Self {
        Self {
            config: inp.config.clone(),

            title: inp.title(),
            artists: inp.artists(),
            year: inp.year(),
            album_title: inp.album_title(),
            album_artists: inp.album_artists(),
            album_cover: inp.album_cover(),
            track_number: inp.track_number(),
            total_tracks: inp.total_tracks(),
            disc_number: inp.disc_number(),
            total_discs: inp.total_discs(),
        }
    }
}

impl<'a> From<AnyTag<'a>> for Id3v2Tag {
    fn from(inp: AnyTag<'a>) -> Self {
        Self {
            config: inp.config.clone(),
            inner: {
                let mut t = id3::Tag::new();
                inp.title().map(|v| t.set_title(v));
                inp.artists_as_string().map(|v| t.set_artist(&v));
                inp.year.map(|v| t.set_year(v));
                inp.album_title().map(|v| t.set_album(v));
                inp.album_artists_as_string().map(|v| t.set_artist(&v));
                inp.track_number().map(|v| t.set_track(v as u32));
                inp.total_tracks().map(|v| t.set_total_tracks(v as u32));
                inp.disc_number().map(|v| t.set_disc(v as u32));
                inp.total_discs().map(|v| t.set_total_discs(v as u32));
                t
            },
        }
    }
}

impl<'a> std::convert::TryFrom<&'a id3::frame::Picture> for Picture<'a> {
    type Error = crate::Error;
    fn try_from(inp: &'a id3::frame::Picture) -> crate::Result<Self> {
        let &id3::frame::Picture {
            ref mime_type,
            ref data,
            ..
        } = inp;
        let mime_type: MimeType = mime_type.as_str().try_into()?;
        Ok(Self {
            data: &data,
            mime_type,
        })
    }
}

impl AudioTagEdit for Id3v2Tag {
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
        self.inner
            .pictures()
            .filter(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
            .next()
            .and_then(|pic| {
                Some(Picture {
                    data: &pic.data,
                    mime_type: (pic.mime_type.as_str()).try_into().ok()?,
                })
            })
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.remove_album_cover();
        self.inner.add_picture(id3::frame::Picture {
            mime_type: String::from(cover.mime_type),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "".to_owned(),
            data: cover.data.to_owned(),
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
}

impl AudioTagWrite for Id3v2Tag {
    fn write_to(&mut self, file: &mut File) -> crate::Result<()> {
        self.inner.write_to(file, id3::Version::Id3v24)?;
        Ok(())
    }
    fn write_to_path(&mut self, path: &str) -> crate::Result<()> {
        self.inner.write_to_path(path, id3::Version::Id3v24)?;
        Ok(())
    }
}

// impl<'a> From<AnyTag<'a>> for Id3Tag {
//     fn from(anytag: AnyTag) -> Self {
//         Self {
//             inner: anytag.into(),
//         }
//     }
// }

// impl From<AnyTag> for id3::Tag {
//     fn from(anytag: AnyTag) -> Self {
//         let mut id3tag = Self::default();
//         anytag
//             .artists_as_string(SEP_ARTIST)
//             .map(|v| id3tag.set_artist(v));
//         anytag.year().map(|v| id3tag.set_year(v));
//         anytag.album_title().map(|v| id3tag.set_album(v));
//         anytag
//             .album_artists_as_string(SEP_ARTIST)
//             .map(|v| id3tag.set_album_artist(v));
//         anytag.track_number().map(|v| id3tag.set_track(v as u32));
//         anytag
//             .total_tracks()
//             .map(|v| id3tag.set_total_tracks(v as u32));
//         anytag.disc_number().map(|v| id3tag.set_disc(v as u32));
//         anytag
//             .total_discs()
//             .map(|v| id3tag.set_total_discs(v as u32));
//         id3tag
//     }
// }
