/// A three character language code, as specified by [ISO-639-2].
///
/// For now, this is used exclusively in ID3v2.
///
/// Excerpt from <https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html>:
///
/// > The three byte language field, present in several frames, is used to describe
/// > the language of the frame’s content, according to [ISO-639-2].
/// > The language should be represented in lower case. If the language is not known
/// > the string “XXX” should be used.
///
/// [ISO-639-2]: https://en.wikipedia.org/wiki/List_of_ISO_639-2_codes.
pub type Lang = [u8; 3];

/// English language code
pub const ENGLISH: Lang = *b"eng";

/// Unknown/unspecified language
pub const UNKNOWN_LANGUAGE: [u8; 3] = *b"XXX";
