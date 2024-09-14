/// Identifiers for flag atoms
///
/// Any identifier in here will be treated as having [`AtomData::Bool`] as its data type when parsing.
/// See [`Ilst::set_flag`] for more information.
///
/// [`AtomData::Bool`]: crate::mp4::AtomData::Bool
/// [`Ilst::set_flag`]: crate::mp4::Ilst::set_flag
pub mod flags {
	use crate::mp4::AtomIdent;

	/// Podcast flag (`pcst`)
	pub const PODCAST: AtomIdent<'_> = AtomIdent::Fourcc(*b"pcst");
	/// Gapless playback flag (`pgap`)
	pub const GAPLESS: AtomIdent<'_> = AtomIdent::Fourcc(*b"pgap");
	/// Show work and movement flag (`shwm`)
	pub const SHOW_WORK: AtomIdent<'_> = AtomIdent::Fourcc(*b"shwm");
	/// HD video flag (`hdvd`)
	pub const HD_VIDEO: AtomIdent<'_> = AtomIdent::Fourcc(*b"hdvd");
	/// Compilation flag (`cpil`)
	pub const COMPILATION: AtomIdent<'_> = AtomIdent::Fourcc(*b"cpil");
}

pub(crate) const WELL_KNOWN_TYPE_SET: u8 = 0;
