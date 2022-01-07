//! OPUS/FLAC/Vorbis specific items
//!
//! ## File notes
//!
//! The only supported tag format is [`VorbisComments`]
pub(crate) mod constants;
pub(crate) mod flac;
pub(crate) mod opus;
pub(crate) mod read;
#[cfg(feature = "vorbis_comments")]
pub(crate) mod tag;
pub(crate) mod vorbis;
#[cfg(feature = "vorbis_comments")]
pub(crate) mod write;

pub use crate::ogg::flac::FlacFile;
pub use crate::ogg::opus::properties::OpusProperties;
pub use crate::ogg::opus::OpusFile;
#[cfg(feature = "vorbis_comments")]
pub use crate::ogg::tag::VorbisComments;
pub use crate::ogg::vorbis::properties::VorbisProperties;
pub use crate::ogg::vorbis::VorbisFile;

use crate::{LoftyError, Result};

use std::io::{Read, Seek};

use ogg_pager::Page;

pub(self) fn verify_signature(page: &Page, sig: &[u8]) -> Result<()> {
	let sig_len = sig.len();

	if page.content().len() < sig_len || &page.content()[..sig_len] != sig {
		return Err(LoftyError::Ogg("File missing magic signature"));
	}

	Ok(())
}

pub(self) fn find_last_page<R>(data: &mut R) -> Result<Page>
where
	R: Read + Seek,
{
	let mut last_page = Page::read(data, true)?;

	while let Ok(page) = Page::read(data, true) {
		last_page = page
	}

	Ok(last_page)
}
