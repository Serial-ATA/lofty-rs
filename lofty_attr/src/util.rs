use std::fmt::Display;

use proc_macro2::Span;
use syn::Type;

// https://stackoverflow.com/questions/55271857/how-can-i-get-the-t-from-an-optiont-when-using-syn
pub(crate) fn extract_type_from_option(ty: &Type) -> Option<Type> {
	use syn::{GenericArgument, Path, PathArguments, PathSegment};

	fn extract_type_path(ty: &Type) -> Option<&Path> {
		match *ty {
			Type::Path(ref typepath) if typepath.qself.is_none() => Some(&typepath.path),
			_ => None,
		}
	}

	fn extract_option_segment(path: &Path) -> Option<&PathSegment> {
		let idents_of_path = path.segments.iter().fold(String::new(), |mut acc, v| {
			acc.push_str(&v.ident.to_string());
			acc.push('|');
			acc
		});
		vec!["Option|", "std|option|Option|", "core|option|Option|"]
			.into_iter()
			.find(|s| idents_of_path == *s)
			.and_then(|_| path.segments.last())
	}

	extract_type_path(ty)
		.and_then(extract_option_segment)
		.and_then(|path_seg| {
			let type_params = &path_seg.arguments;
			// It should have only on angle-bracketed param ("<String>"):
			match *type_params {
				PathArguments::AngleBracketed(ref params) => params.args.first(),
				_ => None,
			}
		})
		.and_then(|generic_arg| match *generic_arg {
			GenericArgument::Type(ref ty) => Some(ty.clone()),
			_ => None,
		})
}

pub(crate) fn err<T: Display>(span: Span, error: T) -> syn::Error {
	syn::Error::new(span, error)
}
