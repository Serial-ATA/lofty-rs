//! Tools to create custom file resolvers
//!
//! For a full example of a custom resolver, see [this](https://github.com/Serial-ATA/lofty-rs/tree/main/examples/custom_resolver).
use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
use crate::probe::ParseOptions;
use crate::tag::TagType;

use std::collections::HashMap;
use std::io::{Read, Seek};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

/// A custom file resolver
///
/// This trait allows for the creation of custom [`FileType`]s, that can make use of
/// lofty's API. Registering a `FileResolver` ([`register_custom_resolver`]) makes it possible
/// to detect and read files using [`crate::probe::Probe`].
pub trait FileResolver: Send + Sync + AudioFile {
	/// The extension associated with the [`FileType`] without the '.'
	fn extension() -> Option<&'static str>;
	/// The primary [`TagType`] for the [`FileType`]
	fn primary_tag_type() -> TagType;
	/// The [`FileType`]'s supported [`TagType`]s
	fn supported_tag_types() -> &'static [TagType];

	/// Attempts to guess the [`FileType`] from a portion of the file content
	///
	/// NOTE: This will only provide (up to) the first 36 bytes of the file.
	///       This number is subject to change in the future, but it will never decrease.
	///       Such a change will **not** be considered breaking.
	fn guess(buf: &[u8]) -> Option<FileType>;
}

// Just broken out to its own type to make `CUSTOM_RESOLVER`'s type shorter :)
type ResolverMap = HashMap<&'static str, &'static dyn ObjectSafeFileResolver>;

pub(crate) static CUSTOM_RESOLVERS: Lazy<Arc<Mutex<ResolverMap>>> =
	Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub(crate) fn lookup_resolver(name: &'static str) -> Option<&'static dyn ObjectSafeFileResolver> {
	let res = CUSTOM_RESOLVERS.lock().ok()?;

	res.get(name).copied()
}

// A `Read + Seek` supertrait for use in [`ObjectSafeFileResolver::read_from`]
pub(crate) trait SeekRead: Read + Seek {}
impl<T: Seek + Read> SeekRead for T {}

// `FileResolver` isn't object safe itself, so we need this wrapper trait
pub(crate) trait ObjectSafeFileResolver: Send + Sync {
	fn extension(&self) -> Option<&'static str>;
	fn primary_tag_type(&self) -> TagType;
	fn supported_tag_types(&self) -> &'static [TagType];
	fn guess(&self, buf: &[u8]) -> Option<FileType>;

	// A mask for the `AudioFile::read_from` impl
	fn read_from(
		&self,
		reader: &mut dyn SeekRead,
		parse_options: ParseOptions,
	) -> Result<TaggedFile>;
}

// A fake `FileResolver` implementer, so we don't need to construct the type in `register_custom_resolver`
pub(crate) struct GhostlyResolver<T: 'static>(PhantomData<T>);
impl<T: FileResolver> ObjectSafeFileResolver for GhostlyResolver<T> {
	fn extension(&self) -> Option<&'static str> {
		T::extension()
	}

	fn primary_tag_type(&self) -> TagType {
		T::primary_tag_type()
	}

	fn supported_tag_types(&self) -> &'static [TagType] {
		T::supported_tag_types()
	}

	fn guess(&self, buf: &[u8]) -> Option<FileType> {
		T::guess(buf)
	}

	fn read_from(
		&self,
		reader: &mut dyn SeekRead,
		parse_options: ParseOptions,
	) -> Result<TaggedFile> {
		Ok(<T as AudioFile>::read_from(&mut Box::new(reader), parse_options)?.into())
	}
}

/// Register a custom file resolver
///
/// Provided a type and a name to associate it with, this will attempt
/// to load them into the resolver collection.
///
/// Conditions:
/// * Both the resolver and name *must* be static.
/// * `name` **must** match the name of your custom [`FileType`] variant (case sensitive!)
///
/// # Panics
///
/// * Attempting to register an existing name or type
/// * See [`Mutex::lock`]
pub fn register_custom_resolver<T: FileResolver + 'static>(name: &'static str) {
	let mut res = CUSTOM_RESOLVERS.lock().unwrap();
	assert!(
		res.iter().all(|(n, _)| *n != name),
		"Resolver `{}` already exists!",
		name
	);

	let ghost = GhostlyResolver::<T>(PhantomData::default());
	let b: Box<dyn ObjectSafeFileResolver> = Box::new(ghost);

	res.insert(name, Box::leak::<'static>(b));
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::ID3v2Tag;
	use crate::resolve::{register_custom_resolver, FileResolver};
	use crate::{Accessor, FileProperties, FileType, TagType};
	use lofty_attr::LoftyFile;

	use crate::probe::ParseOptions;
	use std::fs::File;
	use std::io::{Read, Seek};
	use std::panic;

	#[derive(LoftyFile, Default)]
	#[lofty(read_fn = "Self::read")]
	#[lofty(file_type = "Custom(\"MyFile\")")]
	struct MyFile {
		#[lofty(tag_type = "ID3v2")]
		id3v2_tag: Option<ID3v2Tag>,
		properties: FileProperties,
	}

	impl FileResolver for MyFile {
		fn extension() -> Option<&'static str> {
			Some("myfile")
		}

		fn primary_tag_type() -> TagType {
			TagType::ID3v2
		}

		fn supported_tag_types() -> &'static [TagType] {
			&[TagType::ID3v2]
		}

		fn guess(buf: &[u8]) -> Option<FileType> {
			if buf.starts_with(b"myfile") {
				return Some(FileType::Custom("MyFile"));
			}

			None
		}
	}

	impl MyFile {
		#[allow(clippy::unnecessary_wraps)]
		fn read<R: Read + Seek + ?Sized>(
			_reader: &mut R,
			_parse_options: ParseOptions,
		) -> crate::error::Result<Self> {
			let mut tag = ID3v2Tag::default();
			tag.set_artist(String::from("All is well!"));

			Ok(Self {
				id3v2_tag: Some(tag),
				properties: FileProperties::default(),
			})
		}
	}

	#[test]
	fn custom_resolver() {
		register_custom_resolver::<MyFile>("MyFile");

		let path = "examples/custom_resolver/test_asset.myfile";
		let read = crate::read_from_path(path).unwrap();
		assert_eq!(read.file_type(), FileType::Custom("MyFile"));

		let read_content = crate::read_from(&mut File::open(path).unwrap()).unwrap();
		assert_eq!(read_content.file_type(), FileType::Custom("MyFile"));

		assert!(
			panic::catch_unwind(|| {
				register_custom_resolver::<MyFile>("MyFile");
			})
			.is_err(),
			"We didn't panic on double register!"
		);
	}
}
