// https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW35

/// Reserved for use where no type needs to be indicated
pub const RESERVED: u32 = 0;
/// UTF-8 string without any count or NULL terminator
pub const UTF8: u32 = 1;
/// A big-endian UTF-16 string
pub const UTF16: u32 = 2;
/// Deprecated unless it is needed for special Japanese characters
pub const S_JIS: u32 = 3;
/// The UTF-8 variant storage of a string for sorting only
pub const UTF8_SORT: u32 = 4;
/// The UTF-16 variant storage of a string for sorting only
pub const UTF16_SORT: u32 = 5;
/// A JPEG in a JFIF wrapper
pub const JPEG: u32 = 13;
/// A PNG in a PNG wrapper
pub const PNG: u32 = 14;
/// A big-endian signed integer in 1,2,3 or 4 bytes
pub const BE_SIGNED_INTEGER: u32 = 21;
/// A big-endian unsigned integer in 1,2,3 or 4 bytes; size of value determines integer size
pub const BE_UNSIGNED_INTEGER: u32 = 22;
/// A big-endian 32-bit floating point value (IEEE754)
pub const BE_FLOAT32: u32 = 23;
/// A big-endian 64-bit floating point value (IEEE754)
pub const BE_FLOAT64: u32 = 24;
/// Windows bitmap format graphics
pub const BMP: u32 = 27;
/// A QuickTime metadata atom
pub const QUICKTIME_METADATA: u32 = 28;
/// An 8-bit signed integer
pub const SIGNED_8BIT_INTEGER: u32 = 65;
/// A big-endian 16-bit signed integer
pub const BE_16BIT_SIGNED_INTEGER: u32 = 66;
/// A big-endian 32-bit signed integer
pub const BE_32BIT_SIGNED_INTEGER: u32 = 67;
/// A block of data representing a two dimensional (2D) point with 32-bit big-endian floating point x and y coordinates. It has the structure:
///
/// ```c
/// struct {
///     BEFloat32 x;
///     BEFloat32 y;
/// }
/// ```
pub const BE_POINT_F32: u32 = 70;
/// A block of data representing 2D dimensions with 32-bit big-endian floating point width and height. It has the structure:
///
/// ```c
/// struct {
///     BEFloat32 width;
///     BEFloat32 height;
/// }
/// ```
pub const BE_DIMENSIONS_F32: u32 = 71;
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
pub const BE_RECT_F32: u32 = 72;
/// A big-endian 64-bit signed integer
pub const BE_64BIT_SIGNED_INTEGER: u32 = 74;
/// An 8-bit unsigned integer
pub const UNSIGNED_8BIT_INTEGER: u32 = 75;
/// A big-endian 16-bit unsigned integer
pub const BE_16BIT_UNSIGNED_INTEGER: u32 = 76;
/// A big-endian 32-bit unsigned integer
pub const BE_32BIT_UNSIGNED_INTEGER: u32 = 77;
/// A big-endian 64-bit unsigned integer
pub const BE_64BIT_UNSIGNED_INTEGER: u32 = 78;
/// A block of data representing a 3x3 transformation matrix. It has the structure:
///
/// ```c
/// struct {
///     BEFloat64 matrix[3][3];
/// }
/// ```
pub const AFFINE_TRANSFORM_F64: u32 = 79;
