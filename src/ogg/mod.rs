//! Items for OGG container formats
//!
//! ## File notes
//!
//! The only supported tag format is [`VorbisComments`]
pub(crate) mod constants;
pub(crate) mod opus;
pub(crate) mod read;
pub(crate) mod speex;
pub(crate) mod tag;
pub(crate) mod vorbis;
pub(crate) mod write;

use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

use ogg_pager::Page;

// Exports

pub use opus::properties::OpusProperties;
pub use opus::OpusFile;
pub use speex::properties::SpeexProperties;
pub use speex::SpeexFile;
pub use tag::VorbisComments;
pub use vorbis::properties::VorbisProperties;
pub use vorbis::VorbisFile;

pub(self) fn verify_signature(content: &[u8], sig: &[u8]) -> Result<()> {
	let sig_len = sig.len();

	if content.len() < sig_len || &content[..sig_len] != sig {
		decode_err!(@BAIL Vorbis, "File missing magic signature");
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
