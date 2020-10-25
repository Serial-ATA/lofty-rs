use std::fmt::{Debug, Formatter, Result};
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ReadBytesExt};

use crate::{Atom, AtomT, core::atom, Data, DataT, ErrorKind, Ident};

/// An enum representing the different types of content an atom might have.
#[derive(Clone, PartialEq)]
pub enum Content {
    /// A value containing a list of children atoms.
    Atoms(Vec<Atom>),
    /// A value containing raw data.
    RawData(Data),
    /// A value containing data defined by a
    /// [Table 3-5 Well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34)
    /// code.
    TypedData(Data),
    /// Empty content.
    Empty,
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Content::Atoms(a) => write!(f, "Content::Atoms{{ {:#?} }}", a),
            Content::RawData(d) => write!(f, "Content::RawData{{ {:?} }}", d),
            Content::TypedData(d) => write!(f, "Content::TypedData{{ {:?} }}", d),
            Content::Empty => write!(f, "Content::Empty"),
        }
    }
}

impl Content {
    /// Creates new empty content of type [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn atoms() -> Self {
        Self::Atoms(Vec::new())
    }

    /// Creates new content of type [Self::Atoms](enum.Content.html#variant.Atoms) containing the
    /// atom.
    pub fn atom(atom: Atom) -> Self {
        Self::Atoms(vec![atom])
    }

    /// Creates new content of type [Self::Atoms](enum.Content.html#variant.Atoms) containing a
    /// data [`Atom`](struct.Atom.html) with the data.
    pub fn data_atom_with(data: Data) -> Self {
        Self::atom(Atom::data_atom_with(data))
    }

    /// Creates new content of type [Self::Atoms](Content::Atoms) containing a new
    /// [`Atom`](struct.Atom.html) with the identifier, offset and content.
    pub fn atom_with(ident: Ident, offset: usize, content: Self) -> Self {
        Self::atom(Atom::with(ident, offset, content))
    }

    /// Adds the atom to the list of children atoms if `self` is of type [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn add_atom(self, atom: Atom) -> Self {
        if let Self::Atoms(mut atoms) = self {
            atoms.push(atom);
            Self::Atoms(atoms)
        } else {
            self
        }
    }

    /// Adds a new [`Atom`](struct.Atom.html) with the provided `identifier`, `offset` and `content`
    /// to the list of children if `self` is of type [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn add_atom_with(self, ident: Ident, offset: usize, content: Self) -> Self {
        self.add_atom(Atom::with(ident, offset, content))
    }

    /// Returns the length in bytes.
    pub fn len(&self) -> usize {
        match self {
            Self::Atoms(v) => v.iter().map(|a| a.len()).sum(),
            Self::RawData(d) => d.len(),
            Self::TypedData(d) => 8 + d.len(),
            Self::Empty => 0,
        }
    }

    /// Returns true if the content is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Attempts to write the content to the `writer`.
    pub fn write_to(&self, writer: &mut impl Write) -> crate::Result<()> {
        match self {
            Self::Atoms(v) => {
                for a in v {
                    a.write_to(writer)?;
                }
            }
            Self::RawData(d) => d.write_raw(writer)?,
            Self::TypedData(d) => d.write_typed(writer)?,
            Self::Empty => (),
        }

        Ok(())
    }
}

/// A template representing the different types of content an atom template might have.
#[derive(Clone, PartialEq)]
pub enum ContentT {
    /// A
    Atoms(Vec<AtomT>),
    /// A value containing a data template specifying the datatype.
    RawData(DataT),
    /// A template representing typed data that is defined by a
    /// [Table 3-5 Well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34)
    /// code prior to the data parsed.
    TypedData,
    /// Empty content.
    Empty,
}

impl Debug for ContentT {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ContentT::Atoms(a) => write!(f, "ContentT::Atoms{{ {:#?} }}", a),
            ContentT::RawData(d) => write!(f, "ContentT::RawData{{ {:?} }}", d),
            ContentT::TypedData => write!(f, "ContentT::TypedData"),
            ContentT::Empty => write!(f, "ContentT::Empty"),
        }
    }
}

impl ContentT {
    /// Creates a new empty content template of type [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn atoms_t() -> Self {
        Self::Atoms(Vec::new())
    }

    /// Creates a new content template of type [Self::Atoms](enum.Content.html#variant.Atoms)
    /// containing the `atom` template.
    pub fn atom_t(atom: AtomT) -> Self {
        Self::Atoms(vec![atom])
    }

    /// Creates a new content template of type [Self::Atoms](enum.Content.html#variant.Atoms)
    /// containing a data atom template.
    pub fn data_atom_t() -> Self {
        Self::atom_t(AtomT::data_atom())
    }

    /// Creates a new content template of type [Self::Atoms](enum.Content.html#variant.Atoms)
    /// containing a new atom template with the `identifier`, `offset` and `content`.
    pub fn atom_t_with(ident: Ident, offset: usize, content: Self) -> Self {
        Self::atom_t(AtomT::with(ident, offset, content))
    }

    /// Adds the atom template to the list of children atom templates if `self` is of type
    /// [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn add_atom_t(self, atom: AtomT) -> Self {
        if let Self::Atoms(mut atoms) = self {
            atoms.push(atom);
            Self::Atoms(atoms)
        } else {
            self
        }
    }

    /// Adds a data atom template to the list of children if `self` is of type
    /// [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn add_data_atom_t(self) -> Self {
        self.add_atom_t(AtomT::data_atom())
    }

    /// Adds a new atom template with the provided `identifier`, `offset` and `content` template to
    /// the list of children, if `self` is of type [Self::Atoms](enum.Content.html#variant.Atoms).
    pub fn add_atom_t_with(self, ident: Ident, offset: usize, content: Self) -> Self {
        self.add_atom_t(AtomT::with(ident, offset, content))
    }

    /// Attempts to parse corresponding content from the `reader`.
    pub fn parse(&self, reader: &mut (impl Read + Seek), length: usize) -> crate::Result<Content> {
        Ok(match self {
            ContentT::Atoms(v) => Content::Atoms(atom::parse_atoms(v, reader, length)?),
            ContentT::RawData(d) => Content::RawData(d.parse(reader, length)?),
            ContentT::TypedData => {
                if length >= 8 {
                    let datatype = match reader.read_u32::<BigEndian>() {
                        Ok(d) => d,
                        Err(e) => return Err(crate::Error::new(
                            crate::ErrorKind::Io(e),
                            "Error reading typed data head".into(),
                        )),
                    };

                    // Skipping 4 byte locale indicator
                    reader.seek(SeekFrom::Current(4))?;

                    Content::TypedData(DataT::with(datatype).parse(reader, length - 8)?)
                } else {
                    return Err(crate::Error::new(
                        ErrorKind::Parsing,
                        "Typed data head to short".into(),
                    ));
                }
            }
            ContentT::Empty => Content::Empty,
        })
    }
}