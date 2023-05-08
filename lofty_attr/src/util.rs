use std::fmt::Display;

use proc_macro2::Span;
use syn::{Attribute, Error, LitStr, Meta, MetaList, Type};

macro_rules! bail {
	($errors:ident, $span:expr, $msg:expr) => {
		$errors.push(crate::util::err($span, $msg));
		return proc_macro2::TokenStream::new();
	};
}

pub(crate) use bail;

pub(crate) fn get_attr(name: &str, attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
	let mut found = None;
	for attr in attrs {
		if let Some(list) = get_attr_list("lofty", attr) {
			let res = list.parse_nested_meta(|meta| {
				if meta.path.is_ident(name) {
					let value = meta.value()?;
					let value_str: LitStr = value.parse()?;
					found = Some(value_str.parse::<proc_macro2::TokenStream>().unwrap());
					return Ok(());
				}

				Err(meta.error(""))
			});

			if res.is_ok() {
				return found;
			}
		}
	}

	found
}

pub(crate) fn has_path_attr(attr: &Attribute, name: &str) -> bool {
	if let Some(list) = get_attr_list("lofty", attr) {
		let res = list.parse_nested_meta(|meta| {
			if meta.path.is_ident(name) {
				return Ok(());
			}

			Err(Error::new(Span::call_site(), ""))
		});

		return res.is_ok();
	}

	false
}

pub(crate) fn get_attr_list(path: &str, attr: &Attribute) -> Option<MetaList> {
	if attr.path().is_ident(path) {
		if let Meta::List(list) = &attr.meta {
			return Some(list.clone());
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
