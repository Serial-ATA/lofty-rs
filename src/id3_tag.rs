use super::*;
use id3;

pub(crate) struct Id3Tag {
    inner: id3::Tag,
}

impl Id3Tag {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
        Ok(Self {
            inner: id3::Tag::read_from_path(path)?,
        })
    }
}

impl AudioTagsIo for Id3Tag {
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
