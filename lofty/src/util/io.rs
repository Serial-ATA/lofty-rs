//! Various traits for reading and writing to file-like objects

use crate::error::{LoftyError, Result};
use crate::util::math::F80;

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};

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

/// Provides a method to truncate an object to the specified length
///
/// This is one component of the [`FileLike`] trait, which is used to provide implementors access to any
/// file saving methods such as [`AudioFile::save_to`](crate::file::AudioFile::save_to).
///
/// Take great care in implementing this for downstream types, as Lofty will assume that the
/// container has the new length specified. If this assumption were to be broken, files **will** become corrupted.
///
/// # Examples
///
/// ```rust
/// use lofty::io::Truncate;
///
/// let mut data = vec![1, 2, 3, 4, 5];
/// data.truncate(3);
///
/// assert_eq!(data, vec![1, 2, 3]);
/// ```
pub trait Truncate {
	/// The error type of the truncation operation
	type Error: Into<LoftyError>;

	/// Truncate a storage object to the specified length
	///
	/// # Errors
	///
	/// Errors depend on the object being truncated, which may not always be fallible.
	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error>;
}

impl Truncate for File {
	type Error = std::io::Error;

	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error> {
		self.set_len(new_len)
	}
}

impl Truncate for Vec<u8> {
	type Error = std::convert::Infallible;

	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error> {
		self.truncate(new_len as usize);
		Ok(())
	}
}

impl Truncate for VecDeque<u8> {
	type Error = std::convert::Infallible;

	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error> {
		self.truncate(new_len as usize);
		Ok(())
	}
}

impl<T> Truncate for Cursor<T>
where
	T: Truncate,
{
	type Error = <T as Truncate>::Error;

	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error> {
		self.get_mut().truncate(new_len)
	}
}

impl<T> Truncate for Box<T>
where
	T: Truncate,
{
	type Error = <T as Truncate>::Error;

	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error> {
		self.as_mut().truncate(new_len)
	}
}

impl<T> Truncate for &mut T
where
	T: Truncate,
{
	type Error = <T as Truncate>::Error;

	fn truncate(&mut self, new_len: u64) -> std::result::Result<(), Self::Error> {
		(**self).truncate(new_len)
	}
}

/// Provides a method to get the length of a storage object
///
/// This is one component of the [`FileLike`] trait, which is used to provide implementors access to any
/// file saving methods such as [`AudioFile::save_to`](crate::file::AudioFile::save_to).
///
/// Take great care in implementing this for downstream types, as Lofty will assume that the
/// container has the exact length specified. If this assumption were to be broken, files **may** become corrupted.
///
/// # Examples
///
/// ```rust
/// use lofty::io::Length;
///
/// let data = vec![1, 2, 3, 4, 5];
/// assert_eq!(data.len(), 5);
/// ```
pub trait Length {
	/// The error type of the length operation
	type Error: Into<LoftyError>;

	/// Get the length of a storage object
	///
	/// # Errors
	///
	/// Errors depend on the object being read, which may not always be fallible.
	fn len(&self) -> std::result::Result<u64, Self::Error>;
}

impl Length for File {
	type Error = std::io::Error;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		self.metadata().map(|m| m.len())
	}
}

impl Length for Vec<u8> {
	type Error = std::convert::Infallible;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		Ok(self.len() as u64)
	}
}

impl Length for VecDeque<u8> {
	type Error = std::convert::Infallible;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		Ok(self.len() as u64)
	}
}

impl<T> Length for Cursor<T>
where
	T: Length,
{
	type Error = <T as Length>::Error;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		Length::len(self.get_ref())
	}
}

impl<T> Length for Box<T>
where
	T: Length,
{
	type Error = <T as Length>::Error;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		Length::len(self.as_ref())
	}
}

impl<T> Length for &T
where
	T: Length,
{
	type Error = <T as Length>::Error;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		Length::len(*self)
	}
}

impl<T> Length for &mut T
where
	T: Length,
{
	type Error = <T as Length>::Error;

	fn len(&self) -> std::result::Result<u64, Self::Error> {
		Length::len(*self)
	}
}

/// Provides a set of methods to read and write to a file-like object
///
/// This is a combination of the [`Read`], [`Write`], [`Seek`], [`Truncate`], and [`Length`] traits.
/// It is used to provide implementors access to any file saving methods such as [`AudioFile::save_to`](crate::file::AudioFile::save_to).
///
/// Take great care in implementing this for downstream types, as Lofty will assume that the
/// trait implementations are correct. If this assumption were to be broken, files **may** become corrupted.
pub trait FileLike: Read + Write + Seek + Truncate + Length
where
	<Self as Truncate>::Error: Into<LoftyError>,
	<Self as Length>::Error: Into<LoftyError>,
{
}

impl<T> FileLike for T
where
	T: Read + Write + Seek + Truncate + Length,
	<T as Truncate>::Error: Into<LoftyError>,
	<T as Length>::Error: Into<LoftyError>,
{
}

pub(crate) trait ReadExt: Read {
	fn read_f80(&mut self) -> Result<F80>;
}

impl<R> ReadExt for R
where
	R: Read,
{
	fn read_f80(&mut self) -> Result<F80> {
		let mut bytes = [0; 10];
		self.read_exact(&mut bytes)?;

		Ok(F80::from_be_bytes(bytes))
	}
}

#[cfg(test)]
mod tests {
	use crate::config::{ParseOptions, WriteOptions};
	use crate::file::AudioFile;
	use crate::mpeg::MpegFile;
	use crate::tag::Accessor;

	use std::io::{Cursor, Read, Seek, Write};

	const TEST_ASSET: &str = "tests/files/assets/minimal/full_test.mp3";

	fn test_asset_contents() -> Vec<u8> {
		std::fs::read(TEST_ASSET).unwrap()
	}

	fn file() -> MpegFile {
		let file_contents = test_asset_contents();
		let mut reader = Cursor::new(file_contents);
		MpegFile::read_from(&mut reader, ParseOptions::new()).unwrap()
	}

	fn alter_tag(file: &mut MpegFile) {
		let tag = file.id3v2_mut().unwrap();
		tag.set_artist(String::from("Bar artist"));
	}

	fn revert_tag(file: &mut MpegFile) {
		let tag = file.id3v2_mut().unwrap();
		tag.set_artist(String::from("Foo artist"));
	}

	#[test_log::test]
	fn io_save_to_file() {
		// Read the file and change the artist
		let mut file = file();
		alter_tag(&mut file);

		let mut temp_file = tempfile::tempfile().unwrap();
		let file_content = std::fs::read(TEST_ASSET).unwrap();
		temp_file.write_all(&file_content).unwrap();
		temp_file.rewind().unwrap();

		// Save the new artist
		file.save_to(&mut temp_file, WriteOptions::new().preferred_padding(0))
			.expect("Failed to save to file");

		// Read the file again and change the artist back
		temp_file.rewind().unwrap();
		let mut file = MpegFile::read_from(&mut temp_file, ParseOptions::new()).unwrap();
		revert_tag(&mut file);

		temp_file.rewind().unwrap();
		file.save_to(&mut temp_file, WriteOptions::new().preferred_padding(0))
			.expect("Failed to save to file");

		// The contents should be the same as the original file
		temp_file.rewind().unwrap();
		let mut current_file_contents = Vec::new();
		temp_file.read_to_end(&mut current_file_contents).unwrap();

		assert_eq!(current_file_contents, test_asset_contents());
	}

	#[test_log::test]
	fn io_save_to_vec() {
		// Same test as above, but using a Cursor<Vec<u8>> instead of a file
		let mut file = file();
		alter_tag(&mut file);

		let file_content = std::fs::read(TEST_ASSET).unwrap();

		let mut reader = Cursor::new(file_content);
		file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
			.expect("Failed to save to vec");

		reader.rewind().unwrap();
		let mut file = MpegFile::read_from(&mut reader, ParseOptions::new()).unwrap();
		revert_tag(&mut file);

		reader.rewind().unwrap();
		file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
			.expect("Failed to save to vec");

		let current_file_contents = reader.into_inner();
		assert_eq!(current_file_contents, test_asset_contents());
	}

	#[test_log::test]
	fn io_save_using_references() {
		struct File {
			buf: Vec<u8>,
		}

		let mut f = File {
			buf: std::fs::read(TEST_ASSET).unwrap(),
		};

		// Same test as above, but using references instead of owned values
		let mut file = file();
		alter_tag(&mut file);

		{
			let mut reader = Cursor::new(&mut f.buf);
			file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
				.expect("Failed to save to vec");
		}

		{
			let mut reader = Cursor::new(&f.buf[..]);
			file = MpegFile::read_from(&mut reader, ParseOptions::new()).unwrap();
			revert_tag(&mut file);
		}

		{
			let mut reader = Cursor::new(&mut f.buf);
			file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
				.expect("Failed to save to vec");
		}

		let current_file_contents = f.buf;
		assert_eq!(current_file_contents, test_asset_contents());
	}
}
