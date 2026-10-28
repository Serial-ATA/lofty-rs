pub(crate) const LYRICS3_HEADER: &str = "LYRICSBEGIN";
pub(crate) const LYRICS3_FOOTER: &str = "LYRICSEND";

/// The [spec](https://id3.org/Lyrics3) isn't clear on this. It says:
///
/// > The maximum length of the lyrics is 5100 bytes.
///
/// But later mentions:
///
/// > [...] Is relatively arbitrary in length, however, it shouldn't be larger than 4096 bytes.
///
/// Since 5100 appears multiple times in the spec, we'll just accept that as the intended value.
pub const MAX_LYRICS_LEN: usize = 5100;

use crate::tag::ItemKey;
pub(crate) const VALID_ITEMKEYS: [ItemKey; 1] = [ItemKey::Lyrics];
