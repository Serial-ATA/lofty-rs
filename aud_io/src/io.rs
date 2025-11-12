//! Various traits for reading and writing to file-like objects

use crate::error::{AudioError, Result};
use crate::math::F80;

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};

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
	type Error: Into<AudioError>;

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
	type Error: Into<AudioError>;

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
	<Self as Truncate>::Error: Into<AudioError>,
	<Self as Length>::Error: Into<AudioError>,
{
}

impl<T> FileLike for T
where
	T: Read + Write + Seek + Truncate + Length,
	<T as Truncate>::Error: Into<AudioError>,
	<T as Length>::Error: Into<AudioError>,
{
}

pub trait ReadExt: Read {
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

/// Stable version of [`Seek::stream_len()`]
pub trait SeekStreamLen: Seek {
	fn stream_len_hack(&mut self) -> Result<u64> {
		use std::io::SeekFrom;

		let current_pos = self.stream_position()?;
		let len = self.seek(SeekFrom::End(0))?;

		self.seek(SeekFrom::Start(current_pos))?;

		Ok(len)
	}
}

impl<T> SeekStreamLen for T where T: Seek {}
