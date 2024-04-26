pub(crate) mod alloc;
pub mod io;
pub(crate) mod math;
pub(crate) mod text;

pub(crate) fn flag_item(item: &str) -> Option<bool> {
	match item {
		"1" | "true" => Some(true),
		"0" | "false" => Some(false),
		_ => None,
	}
}
