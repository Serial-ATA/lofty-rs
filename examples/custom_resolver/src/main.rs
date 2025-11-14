#![allow(missing_docs)]

use lofty::ape::ApeTag;
use lofty::config::{GlobalOptions, ParseOptions};
use lofty::error::Result as LoftyResult;
use lofty::file::FileType;
use lofty::id3::v2::Id3v2Tag;
use lofty::properties::FileProperties;
use lofty::resolve::FileResolver;
use lofty::tag::{TagSupport, TagType};
use lofty_attr::LoftyFile;

use std::fs::File;

#[rustfmt::skip]
// This `LoftyFile` derive will setup most of the necessary boilerplate
// for you.
#[derive(LoftyFile)]
// `read_fn` is the function that will house your parsing logic.
// See `lofty::AudioFile::read_from` for the expected signature.
#[lofty(read_fn = "Self::parse_my_file")]
// The `FileType` variant of the file
#[lofty(file_type = "MyFile")]
struct MyFile {
	// A file has two requirements, at least one tag field, and a properties field.

	// Tag field requirements:
	// * Fields *must* end with "_tag" to set them apart from the others.
	// * The type of the field *must* implement `TagExt`


	// Specify a tag type
	#[lofty(tag_type = "Id3v2")]
	// Let's say our file *always* has an ID3v2Tag present.
	pub id3v2_tag: Id3v2Tag,

	// Our APE tag is optional in this format, so we wrap it in an `Option`
	#[lofty(tag_type = "Ape")]
	pub ape_tag: Option<ApeTag>,

	// The properties field *must* be present and named as such.
	// The only requirement for this field is that the type *must* implement `Into<FileProperties>`.
	pub properties: FileProperties,
}

impl MyFile {
	#[allow(clippy::unnecessary_wraps)]
	pub fn parse_my_file<R>(_reader: &mut R, _parse_options: ParseOptions) -> LoftyResult<Self>
	where
		R: std::io::Read + std::io::Seek,
	{
		// Your parsing logic...

		Ok(Self {
			id3v2_tag: Id3v2Tag::default(),
			ape_tag: None,
			properties: FileProperties::default(),
		})
	}
}

// Now, we can setup a resolver for our new file
impl FileResolver for MyFile {
	// The extension of the file, if it has one
	fn extension() -> Option<&'static str> {
		Some("myfile")
	}

	// The primary `TagType` of the file, or the one most
	// likely to be used with it
	fn primary_tag_type() -> TagType {
		TagType::Id3v2
	}

	// All of the `TagType`s this file supports, including the
	// primary one.
	fn tag_support(tag_type: TagType) -> TagSupport {
		match tag_type {
			TagType::Id3v2 | TagType::Ape => TagSupport::ReadWrite,
			_ => TagSupport::Unsupported,
		}
	}

	// This is used to guess the `FileType` when reading the file contents.
	// We are given the first (up to) 36 bytes to work with.
	fn guess(buf: &[u8]) -> Option<FileType> {
		if buf.starts_with(b"myfiledata") {
			Some(FileType::Custom("MyFile"))
		} else {
			None
		}
	}
}

fn main() {
	// Now that we've setup our file, we can register it.
	//
	// `register_custom_resolver` takes the type of our file, alongside a name.
	// The name will be used in the `FileType` variant (e.g. FileType::Custom("MyFile")).
	// The name should preferably match the name of the file struct to avoid confusion.
	lofty::resolve::register_custom_resolver::<MyFile>("MyFile");

	// By default, lofty will not check for custom files.
	// We can enable this by updating our `GlobalOptions`.
	let global_options = GlobalOptions::new().use_custom_resolvers(true);
	lofty::config::apply_global_options(global_options);

	// Now when using the following functions, your custom file will be checked

	let path = "examples/custom_resolver/test_asset.myfile";

	// Detected from the "myfile" extension
	let _ = lofty::read_from_path(path).unwrap();

	let mut file = File::open(path).unwrap();

	// The file's content starts with "myfiledata"
	let _ = lofty::read_from(&mut file).unwrap();
}
