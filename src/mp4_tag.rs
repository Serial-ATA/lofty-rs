use super::*;
use mp4ameta;

pub struct Mp4Tag {
    inner: mp4ameta::Tag,
}

impl Mp4Tag {
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        Ok(Self {
            inner: mp4ameta::Tag::read_from_path(path)?,
        })
    }
}

impl<'a> From<&'a Mp4Tag> for AnyTag<'a> {
    fn from(inp: &'a Mp4Tag) -> Self {
        (&inp.inner).into()
    }
}

impl<'a> From<AnyTag<'a>> for Mp4Tag {
    fn from(inp: AnyTag<'a>) -> Self {
        Self { inner: inp.into() }
    }
}

impl<'a> From<&'a mp4ameta::Tag> for AnyTag<'a> {
    fn from(inp: &'a mp4ameta::Tag) -> Self {
        let mut t = Self::default();
        t.title = inp.title().map(Cow::borrowed);
        let artists = inp.artists().fold(Vec::new(), |mut v, a| {
            v.push(Cow::borrowed(a));
            v
        });
        t.artists = if artists.len() > 0 {
            Some(artists)
        } else {
            None
        };
        if let Some(Ok(y)) = inp.year().map(|y| y.parse()) {
            t.year = Some(y);
        }
        t.album_title = inp.album().map(Cow::borrowed);
        let album_artists = inp.album_artists().fold(Vec::new(), |mut v, a| {
            v.push(Cow::borrowed(a));
            v
        });
        t.album_artists = if album_artists.len() > 0 {
            Some(album_artists)
        } else {
            None
        };
        if let Some(Ok(img)) = inp.artwork().map(|a| a.try_into()) {
            t.album_cover = Some(img);
        }
        let (a, b) = inp.track();
        t.track_number = a;
        t.total_tracks = b;
        let (a, b) = inp.disc();
        t.disc_number = a;
        t.total_discs = b;
        t
    }
}

impl<'a> From<AnyTag<'a>> for mp4ameta::Tag {
    fn from(inp: AnyTag<'a>) -> Self {
        let mut t = mp4ameta::Tag::default();
        inp.title().map(|v| t.set_title(v));
        inp.artists()
            .map(|i| i.iter().for_each(|a| t.add_artist(a.as_ref())));
        inp.year.map(|v| t.set_year(v.to_string()));
        inp.album_title().map(|v| t.set_album(v));
        inp.album_artists()
            .map(|i| i.iter().for_each(|a| t.add_album_artist(a.as_ref())));
        inp.track_number().map(|v| t.set_track_number(v));
        inp.total_tracks().map(|v| t.set_total_tracks(v));
        inp.disc_number().map(|v| t.set_disc_number(v));
        inp.total_discs().map(|v| t.set_total_discs(v));
        t
    }
}

impl<'a> std::convert::TryFrom<&'a mp4ameta::Data> for Picture<'a> {
    type Error = crate::Error;
    fn try_from(inp: &'a mp4ameta::Data) -> crate::Result<Self> {
        Ok(match *inp {
            mp4ameta::Data::Png(ref data) => Self {
                data: Cow::borrowed(data),
                mime_type: MimeType::Png,
            },
            mp4ameta::Data::Jpeg(ref data) => Self {
                data: Cow::borrowed(data),
                mime_type: MimeType::Jpeg,
            },
            _ => return Err(crate::Error::NotAPicture),
        })
    }
}

impl AudioTagIo for Mp4Tag {
    fn into_anytag(&self) -> AnyTag<'_> {
        self.into()
    }

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
        self.inner.set_artist(artist)
    }
    fn artists(&self) -> Option<Vec<&str>> {
        let v = self.inner.artists().fold(Vec::new(), |mut v, a| {
            v.push(a);
            v
        });
        if v.len() > 0 {
            Some(v)
        } else {
            None
        }
    }
    fn add_artist(&mut self, v: &str) {
        self.inner.add_artist(v);
    }

    fn year(&self) -> Option<i32> {
        self.inner.year().and_then(|x| str::parse(x).ok())
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

    fn album_artists(&self) -> Option<Vec<&str>> {
        let v = self.inner.album_artists().fold(Vec::new(), |mut v, a| {
            v.push(a);
            v
        });
        if v.len() > 0 {
            Some(v)
        } else {
            None
        }
    }
    fn add_album_artist(&mut self, v: &str) {
        self.inner.add_album_artist(v);
    }

    fn album_cover(&self) -> Option<Picture> {
        use mp4ameta::Data::*;
        self.inner.artwork().and_then(|data| match data {
            Jpeg(d) => Some(Picture {
                data: Cow::borrowed(d),
                mime_type: MimeType::Jpeg,
            }),
            Png(d) => Some(Picture {
                data: Cow::borrowed(d),
                mime_type: MimeType::Png,
            }),
            _ => None,
        })
    }
    fn set_album_cover(&mut self, cover: Picture) {
        self.remove_album_cover();
        self.inner.add_artwork(match cover.mime_type {
            MimeType::Png => mp4ameta::Data::Png(cover.data.into_owned()),
            MimeType::Jpeg => mp4ameta::Data::Jpeg(cover.data.into_owned()),
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
        self.inner.remove_artists();
    }
    fn remove_year(&mut self) {
        self.inner.remove_year();
    }
    fn remove_album_title(&mut self) {
        self.inner.remove_album();
    }
    fn remove_album_artist(&mut self) {
        self.inner.remove_data(mp4ameta::atom::ALBUM_ARTIST);
        self.inner.remove_album_artists();
    }
    fn remove_album_cover(&mut self) {
        self.inner.remove_artwork();
    }
    fn remove_track(&mut self) {
        self.inner.remove_track(); // faster than removing separately
    }
    fn remove_track_number(&mut self) {
        self.inner.remove_track_number();
    }
    fn remove_total_tracks(&mut self) {
        self.inner.remove_total_tracks();
    }
    fn remove_disc(&mut self) {
        self.inner.remove_disc();
    }
    fn remove_disc_number(&mut self) {
        self.inner.remove_disc_number();
    }
    fn remove_total_discs(&mut self) {
        self.inner.remove_total_discs();
    }

    fn write_to(&mut self, file: &mut File) -> crate::Result<()> {
        self.inner.write_to(file)?;
        Ok(())
    }
    fn write_to_path(&mut self, path: &str) -> crate::Result<()> {
        self.inner.write_to_path(path)?;
        Ok(())
    }
}
