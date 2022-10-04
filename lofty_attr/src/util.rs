use std::fmt::Display;

use proc_macro2::Span;
use syn::{Attribute, Lit, Meta, MetaList, NestedMeta, Type};

macro_rules! bail {
	($errors:ident, $span:expr, $msg:literal) => {
		$errors.push(crate::util::err($span, $msg));
		return proc_macro2::TokenStream::new();
	};
}

pub(crate) use bail;

pub(crate) fn get_attr(name: &str, attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
	for attr in attrs {
		if let Some(list) = get_attr_list("lofty", attr) {
			if let Some(NestedMeta::Meta(Meta::NameValue(mnv))) = list.nested.first() {
				if mnv
					.path
					.segments
					.first()
					.expect("path shouldn't be empty")
					.ident == name
				{
					if let Lit::Str(lit_str) = &mnv.lit {
						return Some(lit_str.parse::<proc_macro2::TokenStream>().unwrap());
					}
				}
			}
		}
	}

	None
}

pub(crate) fn has_path_attr(attr: &Attribute, name: &str) -> bool {
	if let Some(list) = get_attr_list("lofty", attr) {
		if let Some(NestedMeta::Meta(Meta::Path(p))) = list.nested.first() {
			if p.is_ident(name) {
				return true;
			}
		}
	}

	false
}

pub(crate) fn get_attr_list(path: &str, attr: &Attribute) -> Option<MetaList> {
	if attr.path.is_ident(path) {
		if let Ok(Meta::List(list)) = attr.parse_meta() {
			return Some(list);
		}
	}

	None
}

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
		let idents_of_path = path
			.segments
			.iter()
			.into_iter()
			.fold(String::new(), |mut acc, v| {
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
