//! Items for OGG container formats
//!
//! ## File notes
//!
//! The only supported tag format is [`VorbisComments`]
pub(crate) mod constants;
pub(crate) mod flac;
pub(crate) mod opus;
pub(crate) mod read;
pub(crate) mod speex;
pub(crate) mod vorbis;

use crate::error::{FileDecodingError, Result};
use crate::types::file::FileType;

use std::io::{Read, Seek};

use ogg_pager::Page;

// Exports

crate::macros::feature_locked! {
	#![cfg(feature = "vorbis_comments")]
	pub(crate) mod write;

	pub(crate) mod tag;
	pub use tag::VorbisComments;
}

pub use flac::FlacFile;
pub use opus::properties::OpusProperties;
pub use opus::OpusFile;
pub use speex::properties::SpeexProperties;
pub use speex::SpeexFile;
pub use vorbis::properties::VorbisProperties;
pub use vorbis::VorbisFile;

pub(self) fn verify_signature(page: &Page, sig: &[u8]) -> Result<()> {
	let sig_len = sig.len();

	if page.content().len() < sig_len || &page.content()[..sig_len] != sig {
		return Err(
			FileDecodingError::new(FileType::Vorbis, "File missing magic signature").into(),
		);
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
