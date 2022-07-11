#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
/// Restrictions on the content of an ID3v2 tag
pub struct TagRestrictions {
	/// Restriction on the size of the tag. See [`TagSizeRestrictions`]
	pub size: TagSizeRestrictions,
	/// Text encoding restrictions
	///
	/// `false` - No restrictions
	/// `true` - Strings are only encoded with [`TextEncoding::Latin1`](crate::id3::v2::TextEncoding::Latin1) or [`TextEncoding::UTF8`](crate::id3::v2::TextEncoding::UTF8)
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

impl TagRestrictions {
	/// Read a [`TagRestrictions`] from a byte
	///
	/// NOTE: See <https://id3.org/id3v2.4.0-structure> section 3.2, item d
	pub fn from_byte(byte: u8) -> Self {
		let mut restrictions = TagRestrictions::default();

		let restriction_flags = byte;

		// xx000000
		match restriction_flags & 0x0C {
			64 => restrictions.size = TagSizeRestrictions::S_64F_128K,
			128 => restrictions.size = TagSizeRestrictions::S_32F_40K,
			192 => restrictions.size = TagSizeRestrictions::S_32F_4K,
			_ => {}, // 0, default
		}

		// 00x00000
		if restriction_flags & 0x20 == 0x20 {
			restrictions.text_encoding = true
		}

		// 000xx000
		match restriction_flags & 0x18 {
			8 => restrictions.text_fields_size = TextSizeRestrictions::C_1024,
			16 => restrictions.text_fields_size = TextSizeRestrictions::C_128,
			24 => restrictions.text_fields_size = TextSizeRestrictions::C_30,
			_ => {}, // 0, default
		}

		// 00000x00
		if restriction_flags & 0x04 == 0x04 {
			restrictions.image_encoding = true
		}

		// 000000xx
		match restriction_flags & 0x03 {
			1 => restrictions.image_size = ImageSizeRestrictions::P_256,
			2 => restrictions.image_size = ImageSizeRestrictions::P_64,
			3 => restrictions.image_size = ImageSizeRestrictions::P_64_64,
			_ => {}, // 0, default
		}

		restrictions
	}

	#[allow(clippy::trivially_copy_pass_by_ref)]
	/// Convert a [`TagRestrictions`] into a `u8`
	pub fn as_bytes(&self) -> u8 {
		let mut byte = 0;

		match self.size {
			TagSizeRestrictions::S_128F_1M => {},
			TagSizeRestrictions::S_64F_128K => byte |= 0x40,
			TagSizeRestrictions::S_32F_40K => byte |= 0x80,
			TagSizeRestrictions::S_32F_4K => byte |= 0x0C,
		}

		if self.text_encoding {
			byte |= 0x20
		}

		match self.text_fields_size {
			TextSizeRestrictions::None => {},
			TextSizeRestrictions::C_1024 => byte |= 0x08,
			TextSizeRestrictions::C_128 => byte |= 0x10,
			TextSizeRestrictions::C_30 => byte |= 0x18,
		}

		if self.image_encoding {
			byte |= 0x04
		}

		match self.image_size {
			ImageSizeRestrictions::None => {},
			ImageSizeRestrictions::P_256 => byte |= 0x01,
			ImageSizeRestrictions::P_64 => byte |= 0x02,
			ImageSizeRestrictions::P_64_64 => byte |= 0x03,
		}

		byte
	}
}
