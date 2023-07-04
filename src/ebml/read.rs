use super::EbmlFile;
use crate::error::Result;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(_reader: &mut R, _parse_options: ParseOptions) -> Result<EbmlFile>
where
	R: Read + Seek,
{
	todo!()
}
