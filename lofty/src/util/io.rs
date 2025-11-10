use std::io::Seek;

// TODO: https://github.com/rust-lang/rust/issues/59359
pub(crate) trait SeekStreamLen: Seek {
	fn stream_len_hack(&mut self) -> crate::error::Result<u64> {
		use std::io::SeekFrom;

		let current_pos = self.stream_position()?;
		let len = self.seek(SeekFrom::End(0))?;

		self.seek(SeekFrom::Start(current_pos))?;

		Ok(len)
	}
}

impl<T> SeekStreamLen for T where T: Seek {}
