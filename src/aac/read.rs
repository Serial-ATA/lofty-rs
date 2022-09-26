use super::AACFile;
use crate::error::Result;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

#[allow(clippy::unnecessary_wraps)]
pub(super) fn read_from<R>(_reader: &mut R, _parse_options: ParseOptions) -> Result<AACFile>
where
	R: Read + Seek,
{
	// TODO
	Ok(AACFile::default())
}
