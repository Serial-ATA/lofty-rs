use crate::picture::MimeType;

/// The [well known] basic data types
///
/// This should cover all the data types you'll encounter in an MP4 file.
///
/// [well known]: https://developer.apple.com/documentation/quicktime-file-format/well-known_types
// OLD LINKS:
// * https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW35
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataType {
	/// Reserved for use where no type needs to be indicated
	Reserved = 0,
	/// UTF-8 string without any count or NULL terminator
	Utf8 = 1,
	/// A big-endian UTF-16 string
	Utf16 = 2,
	/// Deprecated unless it is needed for special Japanese characters
	SJis = 3,
	/// The UTF-8 variant storage of a string for sorting only
	Utf8Sort = 4,
	/// The UTF-16 variant storage of a string for sorting only
	Utf16Sort = 5,
	/// **DEPRECATED** A GIF image
	Gif = 12,
	/// A JPEG in a JFIF wrapper
	Jpeg = 13,
	/// A PNG in a PNG wrapper
	Png = 14,
	/// A big-endian signed integer in 1,2,3 or 4 bytes
	BeSignedInteger = 21,
	/// A big-endian unsigned integer in 1,2,3 or 4 bytes; size of value determines integer size
	BeUnsignedInteger = 22,
	/// A big-endian 32-bit floating point value (IEEE754)
	BeFloat32 = 23,
	/// A big-endian 64-bit floating point value (IEEE754)
	BeFloat64 = 24,
	/// Windows bitmap format graphics
	Bmp = 27,
	/// A QuickTime metadata atom
	QuicktimeMetadata = 28,
	/// An 8-bit signed integer
	Signed8BitInteger = 65,
	/// A big-endian 16-bit signed integer
	Be16BitSignedInteger = 66,
	/// A big-endian 32-bit signed integer
	Be32BitSignedInteger = 67,
	/// A block of data representing a two dimensional (2D) point with 32-bit big-endian floating point x and y coordinates. It has the structure:
	///
	/// ```c
	/// struct {
	///     BEFloat32 x;
	///     BEFloat32 y;
	/// }
	/// ```
	BePointF32 = 70,
	/// A block of data representing 2D dimensions with 32-bit big-endian floating point width and height. It has the structure:
	///
	/// ```c
	/// struct {
	///     BEFloat32 width;
	///     BEFloat32 height;
	/// }
	/// ```
	BeDimensionsF32 = 71,
	/// A block of data representing a 2D rectangle with 32-bit big-endian floating point x and y coordinates and a 32-bit big-endian floating point width and height size. It has the structure:
	///
	/// ```c
	/// struct {
	///     BEFloat32 x;
	///     BEFloat32 y;
	///     BEFloat32 width;
	///     BEFloat32 height;
	/// }
	/// ```
	///
	/// or the equivalent structure:
	///
	/// ```c
	/// struct {
	///     PointF32 origin;
	///     DimensionsF32 size;
	/// }
	/// ```
	BeRectF32 = 72,
	/// A big-endian 64-bit signed integer
	Be64BitSignedInteger = 74,
	/// An 8-bit unsigned integer
	Unsigned8BitInteger = 75,
	/// A big-endian 16-bit unsigned integer
	Be16BitUnsignedInteger = 76,
	/// A big-endian 32-bit unsigned integer
	Be32BitUnsignedInteger = 77,
	/// A big-endian 64-bit unsigned integer
	Be64BitUnsignedInteger = 78,
	/// A block of data representing a 3x3 transformation matrix. It has the structure:
	///
	/// ```c
	/// struct {
	///    BEFloat64 matrix[3][3];
	/// }
	/// ```
	AffineTransformF64 = 79,
	/// Some other data type
	Other(u32),
}

impl From<u32> for DataType {
	fn from(value: u32) -> Self {
		match value {
			0 => DataType::Reserved,
			1 => DataType::Utf8,
			2 => DataType::Utf16,
			3 => DataType::SJis,
			4 => DataType::Utf8Sort,
			5 => DataType::Utf16Sort,
			12 => DataType::Gif,
			13 => DataType::Jpeg,
			14 => DataType::Png,
			21 => DataType::BeSignedInteger,
			22 => DataType::BeUnsignedInteger,
			23 => DataType::BeFloat32,
			24 => DataType::BeFloat64,
			27 => DataType::Bmp,
			28 => DataType::QuicktimeMetadata,
			65 => DataType::Signed8BitInteger,
			66 => DataType::Be16BitSignedInteger,
			67 => DataType::Be32BitSignedInteger,
			70 => DataType::BePointF32,
			71 => DataType::BeDimensionsF32,
			72 => DataType::BeRectF32,
			74 => DataType::Be64BitSignedInteger,
			75 => DataType::Unsigned8BitInteger,
			76 => DataType::Be16BitUnsignedInteger,
			77 => DataType::Be32BitUnsignedInteger,
			78 => DataType::Be64BitUnsignedInteger,
			79 => DataType::AffineTransformF64,
			other => DataType::Other(other),
		}
	}
}

impl From<DataType> for u32 {
	fn from(value: DataType) -> Self {
		match value {
			DataType::Reserved => 0,
			DataType::Utf8 => 1,
			DataType::Utf16 => 2,
			DataType::SJis => 3,
			DataType::Utf8Sort => 4,
			DataType::Utf16Sort => 5,
			DataType::Gif => 12,
			DataType::Jpeg => 13,
			DataType::Png => 14,
			DataType::BeSignedInteger => 21,
			DataType::BeUnsignedInteger => 22,
			DataType::BeFloat32 => 23,
			DataType::BeFloat64 => 24,
			DataType::Bmp => 27,
			DataType::QuicktimeMetadata => 28,
			DataType::Signed8BitInteger => 65,
			DataType::Be16BitSignedInteger => 66,
			DataType::Be32BitSignedInteger => 67,
			DataType::BePointF32 => 70,
			DataType::BeDimensionsF32 => 71,
			DataType::BeRectF32 => 72,
			DataType::Be64BitSignedInteger => 74,
			DataType::Unsigned8BitInteger => 75,
			DataType::Be16BitUnsignedInteger => 76,
			DataType::Be32BitUnsignedInteger => 77,
			DataType::Be64BitUnsignedInteger => 78,
			DataType::AffineTransformF64 => 79,
			DataType::Other(other) => other,
		}
	}
}

impl From<MimeType> for DataType {
	fn from(value: MimeType) -> Self {
		DataType::from(&value)
	}
}

impl From<&MimeType> for DataType {
	fn from(value: &MimeType) -> Self {
		match value {
			MimeType::Gif => DataType::Gif,
			MimeType::Jpeg => DataType::Jpeg,
			MimeType::Png => DataType::Png,
			MimeType::Bmp => DataType::Bmp,
			_ => DataType::Reserved,
		}
	}
}

impl DataType {
	/// A data type can only occupy 24 bits
	pub const MAX: u32 = 16_777_215;
}
