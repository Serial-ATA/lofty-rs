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
use crate::io::{RevSearchEnd, RevSearchStart};
use crate::macros::decode_err;
use crate::util::io::ReadFindExt;

use std::io::{Read, Seek, SeekFrom};

use ogg_pager::{MAX_CONTENT_SIZE, Page, PageError, PageHeader};

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

/// Find the last page in the OGG stream.
///
/// This will leave the reader at the end of the [`Page`].
fn find_last_page<R>(data: &mut R) -> Result<Page>
where
	R: Read + Seek,
{
	const PATTERN: &[u8] = b"OggS";
	const BUFFER_SIZE: u64 = (MAX_CONTENT_SIZE / 4) as u64;

	let header_end = data.stream_position()?;

	// Prior to this point, all implementations only read the header packets. There should *always*
	// be more pages for the audio data, otherwise the file is invalid.
	if !data.rfind(PATTERN).buffer_size(BUFFER_SIZE).search()? {
		return Err(PageError::MissingMagic.into());
	}

	// In the absolute *worst* case we'll do 3 more retries. Realistically though, the last page in
	// the stream should be well below the maximum size.
	let last_page_header;
	loop {
		match PageHeader::read(data) {
			Ok(h) => {
				last_page_header = h;
				break;
			},
			// False positive, keep searching
			Err(
				PageError::MissingMagic | PageError::InvalidVersion | PageError::BadSegmentCount,
			) => {
				if !data
					.rfind(PATTERN)
					.buffer_size(BUFFER_SIZE)
					.start_pos(RevSearchStart::FromCurrent)
					.end_pos(RevSearchEnd::Pos(header_end))
					.search()?
				{
					return Err(PageError::MissingMagic.into());
				}
				continue;
			},
			Err(err) => return Err(err.into()),
		}
	}

	data.seek(SeekFrom::Start(last_page_header.start))?;
	Ok(Page::read(data)?)
}
