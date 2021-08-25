#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
/// Restrictions on the tag size
pub enum TagSizeRestrictions {
	/// No more than 128 frames and 1 MB total tag size
	S_128F_1M,
	/// No more than 64 frames and 128 KB total tag size
	S_64F_128K,
	/// No more than 32 frames and 40 KB total tag size
	S_32F_40K,
	/// No more than 32 frames and 4 KB total tag size
	S_32F_4K,
}

impl Default for TagSizeRestrictions {
	fn default() -> Self {
		Self::S_128F_1M
	}
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
/// Restrictions on text field sizes
pub enum TextSizeRestrictions {
	/// No size restrictions
	None,
	/// No longer than 1024 characters
	C_1024,
	/// No longer than 128 characters
	C_128,
	/// No longer than 30 characters
	C_30,
}

impl Default for TextSizeRestrictions {
	fn default() -> Self {
		Self::None
	}
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
/// Restrictions on all image sizes
pub enum ImageSizeRestrictions {
	/// No size restrictions
	None,
	/// All images are 256x256 or smaller
	P_256,
	/// All images are 64x64 or smaller
	P_64,
	/// All images are **exactly** 64x64
	P_64_64,
}

impl Default for ImageSizeRestrictions {
	fn default() -> Self {
		Self::None
	}
}

#[derive(Default, Clone, Copy)]
/// Restrictions on the content of an ID3v2 tag
pub struct TagRestrictions {
	/// Restriction on the size of the tag. See [`TagSizeRestrictions`]
	pub size: TagSizeRestrictions,
	/// Text encoding restrictions
	///
	/// `false` - No restrictions
	/// `true` - Strings are only encoded with [`TextEncoding::Latin1`](crate::TextEncoding::Latin1) or [`TextEncoding::UTF8`](crate::TextEncoding::UTF8)
	pub text_encoding: bool,
	/// Restrictions on all text field sizes. See [`TextSizeRestrictions`]
	pub text_fields_size: TextSizeRestrictions,
	/// Image encoding restrictions
	///
	/// `false` - No restrictions
	/// `true` - Images can only be `PNG` or `JPEG`
	pub image_encoding: bool,
	/// Restrictions on all image sizes. See [`ImageSizeRestrictions`]
	pub image_size: ImageSizeRestrictions,
}
