//! Various configuration options to control Lofty

mod global_options;
mod parse_options;
mod write_options;

pub use global_options::{GlobalOptions, apply_global_options};
pub use parse_options::{ParseOptions, ParsingMode};
pub use write_options::WriteOptions;

pub(crate) use global_options::global_options;
