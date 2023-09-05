use crate::ebml::vint::VInt;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

pub struct ElementHeader {
	pub(crate) id: VInt,
	pub(crate) size: VInt,
}

impl ElementHeader {
	fn read<R>(reader: &mut R, max_vint_length: u8) -> Result<Self>
	where
		R: Read,
	{
		Ok(Self {
			id: VInt::parse(reader, max_vint_length)?,
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

macro_rules! define_master_elements {
	($(
		$_readable_ident:ident : {
			id: $vint_id:literal,
			children: [
				$($_readable_child_ident:ident : { $child_id:literal, $data_ty:ident }),* $(,)?
			] $(,)?
		}
	),+ $(,)?) => {
		#[derive(Copy, Clone, Eq, PartialEq)]
		pub(crate) enum ElementIdent {
			$(
			$_readable_ident,
			$($_readable_child_ident,)*
			)+
		}

		static MASTER_ELEMENTS: once_cell::sync::Lazy<std::collections::HashMap<VInt, MasterElement>> = once_cell::sync::Lazy::new(|| {
			let mut m = std::collections::HashMap::new();
			$(
				m.insert(
					VInt($vint_id),
					MasterElement {
						id: ElementIdent::$_readable_ident,
						children: &[$((VInt($child_id), ChildElementDescriptor {
							ident: ElementIdent::$_readable_child_ident,
							data_type: ElementDataType::$data_ty,
						})),*][..]
					}
				);
			)+
			m
		});
	}
}

define_master_elements! {
	EBML: {
		id: 0x1A45DFA3,
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
}

struct ElementReaderContext {
	/// Current master element
	current_master: Option<MasterElement>,
	/// Remaining length of the master element
	master_length: u64,
	/// Maximum size in octets of all element IDs
	max_id_length: u8,
	/// Maximum size in octets of all element data sizes
	max_size_length: u8,
}

impl Default for ElementReaderContext {
	fn default() -> Self {
		Self {
			current_master: None,
			master_length: 0,
			// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebmlmaxidlength-element
			max_id_length: 4,
			// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebmlmaxsizelength-element
			max_size_length: 8,
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

	fn next_master(&mut self) -> Result<ElementReaderYield> {
		let header = ElementHeader::read(&mut self.reader, self.ctx.max_size_length)?;
		let Some(master) = MASTER_ELEMENTS.get(&header.id) else {
			// We encountered an unknown master element
			return Ok(ElementReaderYield::Unknown(header));
		};

		self.ctx.current_master = Some(*master);
		self.ctx.master_length = header.size.value();
		Ok(ElementReaderYield::Master((
			master.id,
			self.ctx.master_length,
		)))
	}

	pub(crate) fn next(&mut self) -> Result<ElementReaderYield> {
		let Some(current_master) = self.ctx.current_master else {
			return self.next_master();
		};

		if self.ctx.master_length == 0 {
			return self.next_master();
		}

		let header = ElementHeader::read(&mut self.reader, self.ctx.max_size_length)?;

		let Some((_, child)) = current_master
			.children
			.iter()
			.find(|(id, _)| *id == header.id)
		else {
			return Ok(ElementReaderYield::Unknown(header));
		};

		Ok(ElementReaderYield::Child((*child, header.size.value())))
	}

	pub(crate) fn skip(&mut self, length: u64) -> Result<()> {
		std::io::copy(&mut self.reader.by_ref().take(length), &mut std::io::sink())?;
		Ok(())
	}

	pub(crate) fn read_signed_int(&mut self) -> Result<i64> {
		todo!()
	}

	pub(crate) fn read_unsigned_int(&mut self) -> Result<u64> {
		todo!()
	}

	pub(crate) fn read_float(&mut self, element_length: u64) -> Result<f64> {
		Ok(match element_length {
			0 => 0.0,
			4 => self.reader.read_f32::<BigEndian>()? as f64,
			8 => self.reader.read_f64::<BigEndian>()?,
			_ => decode_err!(@BAIL Ebml, "Invalid size for float element"),
		})
	}

	pub(crate) fn read_string(&mut self) -> Result<String> {
		todo!()
	}

	pub(crate) fn read_utf8(&mut self) -> Result<String> {
		todo!()
	}

	pub(crate) fn read_date(&mut self) -> Result<String> {
		todo!()
	}

	pub(crate) fn read_binary(&mut self) -> Result<Vec<u8>> {
		todo!()
	}
}
