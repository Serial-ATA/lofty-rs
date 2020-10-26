use super::*;
use mp4ameta;

pub(crate) struct Mp4Tag {
    inner: mp4ameta::Tag,
}

impl Mp4Tag {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
        Ok(Self {
            inner: mp4ameta::Tag::read_from_path(path)?,
        })
    }
}

impl AudioTagsIo for Mp4Tag {
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
        self.inner.remove_data(mp4ameta::atom::ARTIST);
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
