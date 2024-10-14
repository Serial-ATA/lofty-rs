use crate::ebml::vint::{ElementId, VInt};
use crate::error::Result;
use crate::macros::{decode_err, try_vec};

use std::io::{self, Read};
use std::ops::{Deref, DerefMut};

use byteorder::{BigEndian, ReadBytesExt};
use lofty_attr::ebml_master_elements;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ElementHeader {
	pub(crate) id: ElementId,
	pub(crate) size: VInt<u64>,
}

impl ElementHeader {
	fn read<R>(reader: &mut R, max_id_length: u8, max_vint_length: u8) -> Result<Self>
	where
		R: Read,
	{
		Ok(Self {
			id: ElementId::parse(reader, max_id_length)?,
			size: VInt::<u64>::parse(reader, max_vint_length)?,
		})
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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

#[derive(Copy, Clone, Debug)]
struct MasterElement {
	id: ElementIdent,
	children: &'static [(ElementId, ChildElementDescriptor)],
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ChildElementDescriptor {
	pub(crate) ident: ElementIdent,
	pub(crate) data_type: ElementDataType,
}

// This macro helps us define the EBML master elements and their children.
//
// It will generate the `ElementIdent` enum, and the `master_elements` function.
//
// The `ElementIdent` enum is used to represent **ONLY** the elements that we care about.
// When one of these elements is encountered, `ElementReader::next()` will return an
// `ElementReaderYield::Master` or `ElementReaderYield::Child`. Otherwise, it will return
// `ElementReaderYield::Unknown`.
//
// The `master_elements` function is used to map the element IDs to their respective
// `MasterElement` struct, which contains the element's identifier and its children.
// This is used to determine the children of a master element when it is encountered.
//
// If a master element is a child to another master element, it will be defined BOTH as a
// child element in the parent master element, and as a top level master element.
//
// To define a master element, use the following syntax:
//
// ELEMENT_IDENT_VARIANT: {
//     id: 0x1234_5678,
//     children: [
//         CHILD_ELEMENT_VARIANT: { 0x1234_5679, DataType },
//         CHILD_ELEMENT_VARIANT2: { 0x1234_567A, DataType },
//     ],
// },
//
// If `CHILD_ELEMENT_VARIANT2` is a master element, it should ALSO be defined at the top level with
// its own children.
//
// Then when parsing, `ELEMENT_IDENT_VARIANT`, `CHILD_ELEMENT_VARIANT`, and `CHILD_ELEMENT_VARIANT2`
// will be available as `ElementIdent` variants.
ebml_master_elements! {
	EBML: {
		id: 0x1A45_DFA3,
		children: [
			EBMLVersion: { 0x4286, UnsignedInt },
			EBMLReadVersion: { 0x42F7, UnsignedInt },
			EBMLMaxIDLength: { 0x42F2, UnsignedInt },
			EBMLMaxSizeLength: { 0x42F3, UnsignedInt },
			DocType: { 0x4282, String },
			DocTypeExtension: { 0x4281, Master },
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
			// SeekHead: { 0x114D_9B74, Master },
			Info: { 0x1549_A966, Master },
			Tracks: { 0x1654_AE6B, Master },
			Tags: { 0x1254_C367, Master },
			Attachments: { 0x1941_A469, Master },
			Chapters: { 0x1043_A770, Master },
		],
	},

	// segment.seekHead
	// SeekHead: {
	// 	id: 0x114D_9B74,
	// 	children: [
	// 		Seek: { 0x4DBB, Master },
	// 	],
	// },

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
			TrackNumber: { 0xD7, UnsignedInt },
			TrackUid: { 0x73C5, UnsignedInt },
			TrackType: { 0x83, UnsignedInt },
			FlagEnabled: { 0xB9, UnsignedInt },
			FlagDefault: { 0x88, UnsignedInt },
			DefaultDuration: { 0x23E3_83, UnsignedInt },
			TrackTimecodeScale: { 0x2331_59, Float },
			Language: { 0x22B5_9C, String },
			LanguageBCP47: { 0x22B59D, String },
			CodecID: { 0x86, String },
			CodecPrivate: { 0x63A2, Binary },
			CodecName: { 0x258688, Utf8 },
			CodecDelay: { 0x56AA, UnsignedInt },
			SeekPreRoll: { 0x56BB, UnsignedInt },
			Audio: { 0xE1, Master },
		],
	},

	// segment.tracks.trackEntry.audio
	Audio: {
		id: 0xE1,
		children: [
			SamplingFrequency: { 0xB5, Float },
			OutputSamplingFrequency: { 0x78B5, Float },
			Channels: { 0x9F, UnsignedInt },
			BitDepth: { 0x6264, UnsignedInt },
			Emphasis: { 0x52F1, UnsignedInt },
		],
	},


	// segment.tags
	Tags: {
		id: 0x1254_C367,
		children: [
			Tag: { 0x7373, Master },
		],
	},

	// segment.tags.tag
	Tag: {
		id: 0x7373,
		children: [
			Targets: { 0x63C0, Master },
			SimpleTag: { 0x67C8, Master },
		],
	},

	// segment.tags.tag.targets
	Targets: {
		id: 0x63C0,
		children: [
			TargetTypeValue: { 0x68CA, UnsignedInt },
			TargetType: { 0x63CA, String },
			TagTrackUID: { 0x63C5, UnsignedInt },
			TagEditionUID: { 0x63C9, UnsignedInt },
			TagChapterUID: { 0x63C4, UnsignedInt },
			TagAttachmentUID: { 0x63C6, UnsignedInt },
		],
	},

	// segment.tags.tag.simpleTag
	SimpleTag: {
		id: 0x67C8,
		children: [
			TagName: { 0x45A3, Utf8 },
			TagLanguage: { 0x447A, String },
			TagLanguageBCP47: { 0x447B, String },
			TagDefault: { 0x4484, UnsignedInt },
			TagDefaultBogus: { 0x44B4, UnsignedInt },
			TagString: { 0x4487, Utf8 },
			TagBinary: { 0x4485, Binary },
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

const MAX_DEPTH: u8 = 16;
const ROOT_DEPTH: u8 = 1;

#[derive(Copy, Clone, Debug)]
struct Depth {
	level: u8,
	length: VInt<u64>,
}

#[derive(Copy, Clone, Debug)]
struct MasterElementContext {
	element: MasterElement,
	depth: Depth,
}

#[derive(Debug)]
struct ElementReaderContext {
	depth: u8,
	masters: Vec<MasterElementContext>,
	/// Maximum size in octets of all element IDs
	max_id_length: u8,
	/// Maximum size in octets of all element data sizes
	max_size_length: u8,
	/// Whether the reader is locked to the master element at `lock_depth`
	///
	/// This is set with [`ElementReader::lock`], and is used to prevent
	/// the reader from reading past the end of the current master element.
	locked: bool,
	/// The depths at which we are locked
	///
	/// When we reach the end of one lock and unlock the reader, we need
	/// to know which depth to lock the reader at again (if any).
	///
	/// This will **always** be sorted, so the current lock will be at the end.
	lock_depths: Vec<usize>,
}

impl Default for ElementReaderContext {
	fn default() -> Self {
		Self {
			depth: 0,
			masters: Vec::with_capacity(MAX_DEPTH as usize),
			// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebmlmaxidlength-element
			max_id_length: 4,
			// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebmlmaxsizelength-element
			max_size_length: 8,
			locked: false,
			lock_depths: Vec::with_capacity(MAX_DEPTH as usize),
		}
	}
}

impl ElementReaderContext {
	fn current_master(&self) -> Option<MasterElementContext> {
		if self.depth == 0 {
			return None;
		}

		self.masters.get((self.depth - 1) as usize).copied()
	}

	fn current_master_length(&self) -> VInt<u64> {
		assert!(self.depth > 0);
		self.current_master()
			.expect("should have current master element")
			.depth
			.length
	}

	fn propagate_length_change(&mut self, length: u64) {
		for master in &mut self.masters {
			master.depth.length = master.depth.length.saturating_sub(length);
		}
	}

	fn remaining_lock_length(&self) -> VInt<u64> {
		assert!(self.locked && !self.lock_depths.is_empty());

		let lock_depth = *self.lock_depths.last().unwrap();
		self.masters[lock_depth - 1].depth.length
	}
}

#[derive(Debug)]
pub(crate) enum ElementReaderYield {
	Master((ElementIdent, VInt<u64>)),
	Child((ChildElementDescriptor, VInt<u64>)),
	Unknown(ElementHeader),
	Eof,
}

impl ElementReaderYield {
	pub fn ident(&self) -> Option<u64> {
		match self {
			ElementReaderYield::Master((ident, _)) => Some(*ident as u64),
			ElementReaderYield::Child((child, _)) => Some(child.ident as u64),
			ElementReaderYield::Unknown(header) => Some(header.id.value()),
			_ => None,
		}
	}

	pub fn size(&self) -> Option<u64> {
		match self {
			ElementReaderYield::Master((_, size)) | ElementReaderYield::Child((_, size)) => {
				Some(size.value())
			},
			ElementReaderYield::Unknown(header) => Some(header.size.value()),
			_ => None,
		}
	}
}

/// An EBML element reader.
pub struct ElementReader<R> {
	reader: R,
	pub(self) ctx: ElementReaderContext,
}

impl<R> Read for ElementReader<R>
where
	R: Read,
{
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		if self.ctx.locked {
			let lock_len = self.ctx.remaining_lock_length().value();
			if buf.len() > lock_len as usize {
				return Err(io::Error::new(
					io::ErrorKind::UnexpectedEof,
					"Cannot read past the end of the current master element",
				));
			}
		}

		let ret = self.reader.read(buf)?;
		if self.ctx.current_master().is_none() {
			return Ok(ret);
		}

		self.ctx.propagate_length_change(ret as u64);

		let current_master = self
			.ctx
			.current_master()
			.expect("should have current master element");
		if current_master.depth.length == 0 {
			self.goto_previous_master()?;
		}

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

	fn push_new_master(&mut self, master: MasterElement, size: VInt<u64>) -> Result<()> {
		log::debug!("New master element: {:?}", master.id);

		if self.ctx.depth == MAX_DEPTH {
			decode_err!(@BAIL Ebml, "Maximum depth reached");
		}

		// If we are at the root level, we do not increment the depth
		// since we are not actually inside a master element.
		// For example, we are moving from \EBML to \Segment.
		let at_root_level = self.ctx.depth == ROOT_DEPTH && self.ctx.current_master_length() == 0;
		if at_root_level {
			assert_eq!(self.ctx.masters.len(), 1);
			self.ctx.masters.clear();
		} else {
			self.ctx.depth += 1;
		}

		self.ctx.masters.push(MasterElementContext {
			element: master,
			depth: Depth {
				level: self.ctx.depth,
				length: size,
			},
		});

		Ok(())
	}

	fn goto_previous_master(&mut self) -> io::Result<()> {
		let lock_depth = self
			.ctx
			.lock_depths
			.last()
			.copied()
			.unwrap_or(ROOT_DEPTH as usize);
		if lock_depth == self.ctx.depth as usize || self.ctx.depth == 0 {
			return Ok(());
		}

		if self.ctx.depth == ROOT_DEPTH {
			return Err(io::Error::new(
				io::ErrorKind::Other,
				"Cannot go to previous master element, already at root",
			));
		}

		while self.ctx.current_master_length() == 0
			&& (self.ctx.depth as usize != lock_depth && self.ctx.depth != ROOT_DEPTH)
		{
			self.ctx.depth -= 1;
			let _ = self.ctx.masters.pop();
		}

		Ok(())
	}

	fn goto_next_master(&mut self) -> Result<ElementReaderYield> {
		self.exhaust_current_master()?;

		let header = ElementHeader::read(self, self.ctx.max_id_length, self.ctx.max_size_length)?;
		let Some(master) = master_elements().get(&header.id) else {
			// We encountered an unknown master element
			return Ok(ElementReaderYield::Unknown(header));
		};

		self.push_new_master(*master, header.size)?;

		Ok(ElementReaderYield::Master((master.id, header.size)))
	}

	pub(crate) fn next(&mut self) -> Result<ElementReaderYield> {
		let Some(current_master) = self.ctx.current_master() else {
			return self.goto_next_master();
		};

		if self.ctx.locked && self.ctx.remaining_lock_length() == 0 {
			return Ok(ElementReaderYield::Eof);
		}

		if current_master.depth.length == 0 {
			return self.goto_next_master();
		}

		let header = ElementHeader::read(self, self.ctx.max_id_length, self.ctx.max_size_length)?;

		let Some((_, child)) = current_master
			.element
			.children
			.iter()
			.find(|(id, _)| *id == header.id)
		else {
			return Ok(ElementReaderYield::Unknown(header));
		};

		if child.data_type == ElementDataType::Master {
			let master = *master_elements()
				.get(&header.id)
				.expect("Nested master elements should be defined at this level.");

			self.push_new_master(master, header.size)?;

			// We encountered a nested master element
			return Ok(ElementReaderYield::Master((child.ident, header.size)));
		}

		Ok(ElementReaderYield::Child((*child, header.size)))
	}

	pub(crate) fn exhaust_current_master(&mut self) -> Result<()> {
		let Some(current_master) = self.ctx.current_master() else {
			return Ok(());
		};

		self.skip(current_master.depth.length.value())?;
		Ok(())
	}

	pub(crate) fn lock(&mut self) {
		log::trace!("New lock at depth: {}", self.ctx.depth);

		self.ctx.locked = true;
		self.ctx.lock_depths.push(self.ctx.depth as usize);
	}

	pub(crate) fn unlock(&mut self) {
		let _ = self.ctx.lock_depths.pop();

		let [.., last] = &*self.ctx.lock_depths else {
			// We can only ever *truly* unlock if we are at the root level.
			log::trace!("Lock freed");

			self.ctx.locked = false;
			return;
		};

		log::trace!("Moving lock to depth: {}", last);
	}

	pub(crate) fn children(&mut self) -> ElementChildIterator<'_, R> {
		self.lock();
		ElementChildIterator::new(self)
	}

	pub(crate) fn skip(&mut self, length: u64) -> Result<()> {
		log::trace!("Skipping {} bytes", length);

		let current_master_length = self.ctx.current_master_length();
		if length > current_master_length.value() {
			decode_err!(@BAIL Ebml, "Cannot skip past the end of the current master element")
		}

		std::io::copy(&mut self.by_ref().take(length), &mut io::sink())?;
		Ok(())
	}

	pub(crate) fn skip_element(&mut self, element_header: ElementHeader) -> Result<()> {
		log::debug!(
			"Encountered unknown EBML element: {:X}, skipping",
			element_header.id.0
		);
		self.skip(element_header.size.value())?;
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

	/// Same as `read_unsigned_int`, but will warn if the value is out of range.
	pub(crate) fn read_flag(&mut self, element_length: u64) -> Result<bool> {
		let val = self.read_unsigned_int(element_length)?;
		if val > 1 {
			log::warn!("Flag value `{}` is out of range, assuming true", val);
		}

		Ok(val != 0)
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

	pub(crate) fn read_binary(&mut self, element_length: u64) -> Result<Vec<u8>> {
		// https://www.rfc-editor.org/rfc/rfc8794.html#section-7.8
		// A Binary Element MUST declare a length in octets from zero to VINTMAX.

		if element_length > VInt::<u64>::MAX {
			decode_err!(@BAIL Ebml, "Binary element length is too large")
		}

		let mut content = try_vec![0; element_length as usize];
		self.read_exact(&mut content)?;
		Ok(content)
	}
}

/// An iterator over the children of an EBML master element.
///
/// This is created by calling [`ElementReader::children`].
///
/// This is essentially a fancy wrapper around `ElementReader` that:
///
/// * Automatically skips unknown elements ([`ElementReaderYield::Unknown`]).
/// * [`Deref`]s to `ElementReader` so you can access the reader's methods.
/// * Unlocks the reader when dropped.
///   * If the reader is locked at multiple depths (meaning [`ElementReader::children`] was called
///     multiple times), it will move the lock to the previously locked depth.
pub(crate) struct ElementChildIterator<'a, R>
where
	R: Read,
{
	reader: &'a mut ElementReader<R>,
}

impl<'a, R> ElementChildIterator<'a, R>
where
	R: Read,
{
	pub(crate) fn new(reader: &'a mut ElementReader<R>) -> Self {
		Self { reader }
	}

	pub(crate) fn next(&mut self) -> Result<Option<ElementReaderYield>> {
		match self.reader.next() {
			Ok(ElementReaderYield::Unknown(header)) => {
				self.reader.skip_element(header)?;
				self.next()
			},
			Err(e) => Err(e),
			element => element.map(Some),
		}
	}

	pub(crate) fn master_exhausted(&self) -> bool {
		let lock_depth = *self
			.reader
			.ctx
			.lock_depths
			.last()
			.expect("a child iterator should always have a lock depth");
		assert!(lock_depth <= self.reader.ctx.depth as usize);

		self.reader.ctx.remaining_lock_length() == 0
	}
}

impl<'a, R> Deref for ElementChildIterator<'a, R>
where
	R: Read,
{
	type Target = ElementReader<R>;

	fn deref(&self) -> &Self::Target {
		self.reader
	}
}

impl<'a, R> DerefMut for ElementChildIterator<'a, R>
where
	R: Read,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.reader
	}
}

impl<'a, R> Drop for ElementChildIterator<'a, R>
where
	R: Read,
{
	fn drop(&mut self) {
		self.reader.unlock();
	}
}
