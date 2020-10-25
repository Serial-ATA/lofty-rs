use core::fmt;
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::ErrorKind;

// [Table 3-5 Well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34) codes
/// Reserved for use where no type needs to be indicated.
#[allow(dead_code)]
pub const RESERVED: u32 = 0;
/// UTF-8 without any count or NULL terminator.
#[allow(dead_code)]
pub const UTF8: u32 = 1;
/// UTF-16 also known as UTF-16BE.
#[allow(dead_code)]
pub const UTF16: u32 = 2;
/// UTF-8 variant storage of a string for sorting only.
#[allow(dead_code)]
pub const UTF8_SORT: u32 = 4;
/// UTF-16 variant storage of a string for sorting only.
#[allow(dead_code)]
pub const UTF16_SORT: u32 = 5;
/// JPEG in a JFIF wrapper.
#[allow(dead_code)]
pub const JPEG: u32 = 13;
/// PNG in a PNG wrapper.
#[allow(dead_code)]
pub const PNG: u32 = 14;
/// A big-endian signed integer in 1,2,3 or 4 bytes.
#[allow(dead_code)]
pub const BE_SIGNED: u32 = 21;
/// A big-endian unsigned integer in 1,2,3 or 4 bytes.
#[allow(dead_code)]
pub const BE_UNSIGNED: u32 = 22;
/// A big-endian 32-bit floating point value (`IEEE754`).
#[allow(dead_code)]
pub const BE_F32: u32 = 23;
/// A big-endian 64-bit floating point value (`IEEE754`).
#[allow(dead_code)]
pub const BE_F64: u32 = 24;
/// Windows bitmap format graphics.
#[allow(dead_code)]
pub const BMP: u32 = 27;
/// QuickTime Metadata atom.
#[allow(dead_code)]
pub const QT_META: u32 = 28;
/// An 8-bit signed integer.
#[allow(dead_code)]
pub const I8: u32 = 65;
/// A big-endian 16-bit signed integer.
#[allow(dead_code)]
pub const BE_I16: u32 = 66;
/// A big-endian 32-bit signed integer.
#[allow(dead_code)]
pub const BE_I32: u32 = 67;
/// A block of data representing a two dimensional (2D) point with 32-bit big-endian floating point
/// x and y coordinates. It has the structure:<br/>
/// `{ BE_F32 x; BE_F32 y; }`
#[allow(dead_code)]
pub const BE_POINT_F32: u32 = 70;
/// A block of data representing 2D dimensions with 32-bit big-endian floating point width and
/// height. It has the structure:<br/>
/// `{ width: BE_F32, height: BE_F32 }`
#[allow(dead_code)]
pub const BE_DIMS_F32: u32 = 71;
/// A block of data representing a 2D rectangle with 32-bit big-endian floating point x and y
/// coordinates and a 32-bit big-endian floating point width and height size. It has the structure:<br/>
/// `{ x: BE_F32, y: BE_F32, width: BE_F32, height: BE_F32 }`<br/>
/// or the equivalent structure:<br/>
/// `{ origin: BE_Point_F32, size: BE_DIMS_F32 }`
#[allow(dead_code)]
pub const BE_RECT_F32: u32 = 72;
/// A big-endian 64-bit signed integer.
#[allow(dead_code)]
pub const BE_I64: u32 = 74;
/// An 8-bit unsigned integer.
#[allow(dead_code)]
pub const U8: u32 = 75;
/// A big-endian 16-bit unsigned integer.
#[allow(dead_code)]
pub const BE_U16: u32 = 76;
/// A big-endian 32-bit unsigned integer.
#[allow(dead_code)]
pub const BE_U32: u32 = 77;
/// A big-endian 64-bit unsigned integer.
#[allow(dead_code)]
pub const BE_U64: u32 = 78;
/// A block of data representing a 3x3 transformation matrix. It has the structure:<br/>
/// `{ matrix: [[BE_F64; 3]; 3] }`
#[allow(dead_code)]
pub const AFFINE_TRANSFORM_F64: u32 = 79;

/// An enum that holds different types of data defined by
/// [Table 3-5 Well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34).
#[derive(Clone, PartialEq)]
pub enum Data {
    /// A value containing reserved type data inside a `Vec<u8>`.
    Reserved(Vec<u8>),
    /// A value containing a `String` decoded from, or to be encoded to utf-8.
    Utf8(String),
    /// A value containing a `String` decoded from, or to be encoded to utf-16.
    Utf16(String),
    /// A value containing jpeg byte data inside a `Vec<u8>`.
    Jpeg(Vec<u8>),
    /// A value containing png byte data inside a `Vec<u8>`.
    Png(Vec<u8>),
    /// A value containing big endian signed integer inside a `Vec<u8>`.
    BeSigned(Vec<u8>),
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Data::Reserved(d) => write!(f, "Reserved{{ {:?} }}", d),
            Data::Utf8(d) => write!(f, "UTF8{{ {:?} }}", d),
            Data::Utf16(d) => write!(f, "UTF16{{ {:?} }}", d),
            Data::Jpeg(_) => write!(f, "JPEG"),
            Data::Png(_) => write!(f, "PNG"),
            Data::BeSigned(d) => write!(f, "Reserved{{ {:?} }}", d),
        }
    }
}

impl Data {
    /// Returns the length in bytes.
    pub fn len(&self) -> usize {
        match self {
            Data::Reserved(v) => v.len(),
            Data::Utf8(s) => s.len(),
            Data::Utf16(s) => s.encode_utf16().count(),
            Data::Jpeg(v) => v.len(),
            Data::Png(v) => v.len(),
            Data::BeSigned(v) => v.len(),
        }
    }

    /// Returns true if the data is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Attempts to write the typed data to the writer.
    pub fn write_typed(&self, writer: &mut impl Write) -> crate::Result<()> {
        let datatype = match self {
            Data::Reserved(_) => RESERVED,
            Data::Utf8(_) => UTF8,
            Data::Utf16(_) => UTF16,
            Data::Jpeg(_) => JPEG,
            Data::Png(_) => PNG,
            Data::BeSigned(_) => BE_SIGNED,
        };

        writer.write_u32::<BigEndian>(datatype)?;
        // Writing 4 byte locale indicator
        writer.write_u32::<BigEndian>(0)?;

        self.write_raw(writer)?;

        Ok(())
    }

    /// Attempts to write the raw data to the writer.
    pub fn write_raw(&self, writer: &mut impl Write) -> crate::Result<()> {
        match self {
            Data::Reserved(v) => { writer.write_all(v)?; }
            Data::Utf8(s) => { writer.write_all(s.as_bytes())?; }
            Data::Utf16(s) => {
                for c in s.encode_utf16() {
                    writer.write_u16::<BigEndian>(c)?;
                }
            }
            Data::Jpeg(v) => { writer.write_all(v)?; }
            Data::Png(v) => { writer.write_all(v)?; }
            Data::BeSigned(v) => { writer.write_all(v)?; }
        }

        Ok(())
    }
}

/// A template used for parsing data defined by
/// [Table 3-5 Well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34).
#[derive(Clone, Debug, PartialEq)]
pub struct DataT {
    /// A datatype defined by
    /// [Table 3-5 Well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34).
    datatype: u32,
}

impl DataT {
    /// Creates a data template containing the datatype.
    pub fn with(datatype: u32) -> Self {
        DataT { datatype }
    }

    /// Attempts to parse corresponding data from the reader.
    pub fn parse(&self, reader: &mut (impl Read + Seek), length: usize) -> crate::Result<Data> {
        Ok(match self.datatype {
            RESERVED => Data::Reserved(read_u8_vec(reader, length)?),
            UTF8 => Data::Utf8(read_utf8(reader, length)?),
            UTF16 => Data::Utf16(read_utf16(reader, length)?),
            JPEG => Data::Jpeg(read_u8_vec(reader, length)?),
            PNG => Data::Png(read_u8_vec(reader, length)?),
            BE_SIGNED => Data::BeSigned(read_u8_vec(reader, length)?),
            _ => return Err(crate::Error::new(
                ErrorKind::UnknownDataType(self.datatype),
                "Unknown datatype code".into(),
            )),
        })
    }
}

/// Attempts to read 8 bit unsigned integers from the reader to a vector of size length.
pub fn read_u8_vec(reader: &mut (impl Read + Seek), length: usize) -> crate::Result<Vec<u8>> {
    let mut buf = vec![0u8; length];

    reader.read_exact(&mut buf)?;

    Ok(buf)
}

/// Attempts to read 16 bit unsigned integers from the reader to a vector of size length.
pub fn read_u16_vec(reader: &mut (impl Read + Seek), length: usize) -> crate::Result<Vec<u16>> {
    let mut buf = vec![0u16; length];

    reader.read_u16_into::<BigEndian>(&mut buf)?;

    Ok(buf)
}

/// Attempts to read a utf-8 string from the reader.
pub fn read_utf8(reader: &mut (impl Read + Seek), length: usize) -> crate::Result<String> {
    let data = read_u8_vec(reader, length)?;

    Ok(String::from_utf8(data)?)
}

/// Attempts to read a utf-16 string from the reader.
pub fn read_utf16(reader: &mut (impl Read + Seek), length: usize) -> crate::Result<String> {
    let data = read_u16_vec(reader, length / 2)?;

    if length % 2 == 1 {
        reader.seek(SeekFrom::Current(1))?;
    }

    Ok(String::from_utf16(&data)?)
}
