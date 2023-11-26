use crate::ebml::vint::VInt;
use crate::error::Result;
use crate::macros::{decode_err, try_vec};

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};
use lofty_attr::ebml_master_elements;

pub struct ElementHeader {
	pub(crate) id: VInt,
	pub(crate) size: VInt,
}

impl ElementHeader {
	fn read<R>(reader: &mut R, max_id_length: u8, max_vint_length: u8) -> Result<Self>
	where
		R: Read,
	{
		Ok(Self {
			id: VInt::parse_from_element_id(reader, max_id_length)?,
			size: VInt::parse(reader, max_vint_length)?,
		})
	}
}

#[derive(Copy, Clone)]
pub enum ElementDataType {
	SignedInt,
	UnsignedInt,
	Float,
	String,
	Utf8,
	Date,
	Master,
	Binary,
}

#[derive(Copy, Clone)]
struct MasterElement {
	id: ElementIdent,
	children: &'static [(VInt, ChildElementDescriptor)],
}

#[derive(Copy, Clone)]
pub(crate) struct ChildElementDescriptor {
	pub(crate) ident: ElementIdent,
	pub(crate) data_type: ElementDataType,
}

ebml_master_elements! {
	EBML: {
		id: 0x1A45_DFA3,
		children: [
			EBMLVersion: { 0x4286, UnsignedInt },
			EBMLReadVersion: { 0x42F7, UnsignedInt },
			EBMLMaxIDLength: { 0x42F2, UnsignedInt },
			EBMLMaxSizeLength: { 0x42F3, UnsignedInt },
			DocType: { 0x4282, String },
			DocTypeVersion: { 0x4287, UnsignedInt },
			DocTypeReadVersion: { 0x4285, UnsignedInt },
		],
	},
	DocTypeExtension: {
		id: 0x4281,
		children: [
			DocTypeExtensionName: { 0x4283, String },
			DocTypeExtensionVersion: { 0x4284, UnsignedInt },
		],
	},

	// The Root Element that contains all other Top-Level Elements
	Segment: {
		id: 0x1853_8067,
		children: [
			Info: { 0x1549_A966, Master },
			Cluster: { 0x1F43_B675, Master },
			Tracks: { 0x1654_AE6B, Master },
			Tags: { 0x1254_C367, Master },
			Attachments: { 0x1941_A469, Master },
			Chapters: { 0x1043_A770, Master },
		],
	},

	// segment.info
	Info: {
		id: 0x1549_A966,
		children: [
			TimecodeScale: { 0x2AD7_B1, UnsignedInt },
			MuxingApp: { 0x4D80, Utf8 },
			WritingApp: { 0x5741, Utf8 },
		],
	},

	// segment.tracks
	Tracks: {
		id: 0x1654_AE6B,
		children: [
			TrackEntry: { 0xAE, Master },
		],
	},

	// segment.tracks.trackEntry
	TrackEntry: {
		id: 0xAE,
		children: [
			TrackType: { 0x83, UnsignedInt },
			FlagEnabled: { 0xB9, UnsignedInt },
			FlagDefault: { 0x88, UnsignedInt },
			FlagLacing: { 0x9C, UnsignedInt },
			DefaultDuration: { 0x23E3_83, UnsignedInt },
			TrackTimecodeScale: { 0x2331_59, Float },
			MaxBlockAdditionID: { 0x55EE, UnsignedInt },
			Language: { 0x22B5_9C, String },
			CodecID: { 0x86, String },
			CodecDelay: { 0x56AA, UnsignedInt },
			SeekPreRoll: { 0x56BB, UnsignedInt },
			TrackTranslate: { 0x6624, Master },
			Video: { 0xE0, Master },
			Audio: { 0xE1, Master },
			TrackOperation: { 0xE2, Master },
			ContentEncodings: { 0x6D80, Master },
		],
	},

	// segment.attachments
	Attachments: {
		id: 0x1941_A469,
		children: [
			AttachedFile: { 0x61A7, Master },
		],
	},

	// segment.attachments.attachedFile
	AttachedFile: {
		id: 0x61A7,
		children: [
			FileDescription: { 0x467E, String },
			FileName: { 0x466E, Utf8 },
			FileMimeType: { 0x4660, String },
			FileData: { 0x465C, Binary },
			FileUID: { 0x46AE, UnsignedInt },
			FileReferral: { 0x4675, Binary },
			FileUsedStartTime: { 0x4661, UnsignedInt },
			FileUsedEndTime: { 0x4662, UnsignedInt },
		],
	},
}

struct ElementReaderContext {
	/// Previous master element
	previous_master: Option<MasterElement>,
	previous_master_length: u64,
	/// Current master element
	current_master: Option<MasterElement>,
	/// Remaining length of the master element
	master_length: u64,
	/// Maximum size in octets of all element IDs
	max_id_length: u8,
	/// Maximum size in octets of all element data sizes
	max_size_length: u8,
	/// Whether the reader is locked to the current master element
	///
	/// This is set with [`ElementReader::lock`], and is used to prevent
	/// the reader from reading past the end of the current master element.
	locked: bool,
}

impl Default for ElementReaderContext {
	fn default() -> Self {
		Self {
			previous_master: None,
			previous_master_length: 0,
			current_master: None,
			master_length: 0,
			// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebmlmaxidlength-element
			max_id_length: 4,
			// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebmlmaxsizelength-element
			max_size_length: 8,
			locked: false,
		}
	}
}

pub(crate) enum ElementReaderYield {
	Master((ElementIdent, u64)),
	Child((ChildElementDescriptor, u64)),
	Unknown(ElementHeader),
	Eof,
}

pub struct ElementReader<R> {
	reader: R,
	ctx: ElementReaderContext,
}

impl<R> Read for ElementReader<R>
where
	R: Read,
{
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let ret = self.reader.read(buf)?;
		self.ctx.master_length = self.ctx.master_length.saturating_sub(ret as u64);
		Ok(ret)
	}
}

impl<R> ElementReader<R>
where
	R: Read,
{
	pub(crate) fn new(reader: R) -> Self {
		Self {
			reader,
			ctx: ElementReaderContext::default(),
		}
	}

	pub(crate) fn set_max_id_length(&mut self, len: u8) {
		self.ctx.max_id_length = len
	}

	pub(crate) fn set_max_size_length(&mut self, len: u8) {
		self.ctx.max_size_length = len
	}

	fn store_previous_master(&mut self) {
		self.ctx.previous_master = self.ctx.current_master;
		self.ctx.previous_master_length = self.ctx.master_length;
	}

	fn next_master(&mut self) -> Result<ElementReaderYield> {
		let header = ElementHeader::read(
			&mut self.reader,
			self.ctx.max_id_length,
			self.ctx.max_size_length,
		)?;
		let Some(master) = MASTER_ELEMENTS.get(&header.id) else {
			// We encountered an unknown master element
			return Ok(ElementReaderYield::Unknown(header));
		};

		self.store_previous_master();
		self.ctx.current_master = Some(*master);
		self.ctx.master_length = header.size.value();
		Ok(ElementReaderYield::Master((
			master.id,
			self.ctx.master_length,
		)))
	}

	/// Lock the reader to the current master element
	pub(crate) fn lock(&mut self) {
		self.ctx.locked = true;
	}

	pub(crate) fn unlock(&mut self) {
		self.ctx.locked = false;
	}

	pub(crate) fn goto_previous_master(&mut self) -> Result<()> {
		if let Some(previous_master) = self.ctx.previous_master {
			self.ctx.current_master = Some(previous_master);
			self.ctx.master_length = self.ctx.previous_master_length;
			Ok(())
		} else {
			decode_err!(@BAIL Ebml, "Expected a parent element to be available")
		}
	}

	pub(crate) fn next(&mut self) -> Result<ElementReaderYield> {
		let Some(current_master) = self.ctx.current_master else {
			return self.next_master();
		};

		if self.ctx.master_length == 0 {
			if self.ctx.locked {
				return Ok(ElementReaderYield::Eof);
			}

			return self.next_master();
		}

		let header = ElementHeader::read(self, self.ctx.max_id_length, self.ctx.max_size_length)?;

		let Some((_, child)) = current_master
			.children
			.iter()
			.find(|(id, _)| *id == header.id)
		else {
			return Ok(ElementReaderYield::Unknown(header));
		};

		if child.data_type == ElementDataType::Master {
			self.store_previous_master();
			self.ctx.current_master = Some(
				*MASTER_ELEMENTS
					.get(&header.id)
					.expect("Nested master elements should be defined at this level."),
			);
			self.ctx.master_length = header.size.value();

			// We encountered a nested master element
			return Ok(ElementReaderYield::Master((
				child.ident,
				header.size.value(),
			)));
		}

		Ok(ElementReaderYield::Child((*child, header.size.value())))
	}

	pub(crate) fn skip(&mut self, length: u64) -> Result<()> {
		std::io::copy(&mut self.by_ref().take(length), &mut std::io::sink())?;
		Ok(())
	}

	pub(crate) fn read_signed_int(&mut self, element_length: u64) -> Result<i64> {
		// https://www.rfc-editor.org/rfc/rfc8794.html#section-7.1
		// A Signed Integer Element MUST declare a length from zero to eight octets
		if element_length > 8 {
			decode_err!(@BAIL Ebml, "Invalid size for signed int element")
		}

		let mut buf = [0; 8];
		self.read_exact(&mut buf[8 - element_length as usize..])?;
		let value = u64::from_be_bytes(buf);

		// Signed Integers are stored with two's complement notation with the leftmost bit being the sign bit.
		let value_width = element_length * 8;
		let shift = (64 - value_width) as u32;
		Ok((value.wrapping_shl(shift) as i64).wrapping_shr(shift))
	}

	pub(crate) fn read_unsigned_int(&mut self, element_length: u64) -> Result<u64> {
		// https://www.rfc-editor.org/rfc/rfc8794.html#section-7.2
		// An Unsigned Integer Element MUST declare a length from zero to eight octets
		if element_length > 8 {
			decode_err!(@BAIL Ebml, "Invalid size for unsigned int element")
		}

		let mut buf = [0; 8];
		self.read_exact(&mut buf[8 - element_length as usize..])?;
		Ok(u64::from_be_bytes(buf))
	}

	pub(crate) fn read_float(&mut self, element_length: u64) -> Result<f64> {
		// https://www.rfc-editor.org/rfc/rfc8794.html#section-7.3
		// A Float Element MUST declare a length of either zero octets (0 bit),
		// four octets (32 bit), or eight octets (64 bit)
		Ok(match element_length {
			0 => 0.0,
			4 => f64::from(self.read_f32::<BigEndian>()?),
			8 => self.read_f64::<BigEndian>()?,
			_ => decode_err!(@BAIL Ebml, "Invalid size for float element"),
		})
	}

	pub(crate) fn read_string(&mut self, element_length: u64) -> Result<String> {
		// https://www.rfc-editor.org/rfc/rfc8794.html#section-7.4
		// A String Element MUST declare a length in octets from zero to VINTMAX
		let mut content = try_vec![0; element_length as usize];
		self.read_exact(&mut content)?;

		// https://www.rfc-editor.org/rfc/rfc8794.html#section-13
		// Null Octets, which are octets with all bits set to zero,
		// MAY follow the value of a String Element or UTF-8 Element to serve as a terminator.
		if let Some(i) = content.iter().rposition(|x| *x != 0) {
			let new_len = i + 1;
			content.truncate(new_len);
		}

		String::from_utf8(content).map_err(Into::into)
	}

	pub(crate) fn read_utf8(&mut self, element_length: u64) -> Result<String> {
		// https://www.rfc-editor.org/rfc/rfc8794.html#section-7.5
		// A UTF-8 Element MUST declare a length in octets from zero to VINTMAX

		// Since the UTF-8 and String elements are both just turned into `String`s,
		// we can just reuse the `read_string` method.
		self.read_string(element_length)
	}

	pub(crate) fn read_date(&mut self) -> Result<String> {
		todo!()
	}

	pub(crate) fn read_binary(&mut self) -> Result<Vec<u8>> {
		todo!()
	}
}
