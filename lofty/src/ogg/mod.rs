//! Items for OGG container formats
//!
//! ## File notes
//!
//! The only supported tag format is [`VorbisComments`]
pub(crate) mod constants;
pub(crate) mod opus;
mod picture_storage;
pub(crate) mod read;
pub(crate) mod speex;
pub(crate) mod tag;
pub(crate) mod vorbis;
pub(crate) mod write;

use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek, SeekFrom};

use ogg_pager::{Page, PageHeader};

// Exports

pub use opus::OpusFile;
pub use opus::properties::OpusProperties;
pub use picture_storage::OggPictureStorage;
pub use speex::SpeexFile;
pub use speex::properties::SpeexProperties;
pub use tag::VorbisComments;
pub use vorbis::VorbisFile;
pub use vorbis::properties::VorbisProperties;

fn verify_signature(content: &[u8], sig: &[u8]) -> Result<()> {
	let sig_len = sig.len();

	if content.len() < sig_len || &content[..sig_len] != sig {
		decode_err!(@BAIL Vorbis, "File missing magic signature");
	}

	Ok(())
}

fn find_last_page<R>(data: &mut R) -> Result<Page>
where
	R: Read + Seek,
{
	let mut last_page_header = PageHeader::read(data)?;
	data.seek(SeekFrom::Current(last_page_header.content_size() as i64))?;

	while let Ok(header) = PageHeader::read(data) {
		last_page_header = header;
		data.seek(SeekFrom::Current(last_page_header.content_size() as i64))?;
	}

	data.seek(SeekFrom::Start(last_page_header.start))?;
	Ok(Page::read(data)?)
}
