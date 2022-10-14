use crate::util::bail;

use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::{AttributeArgs, ItemStruct, Lit, Meta, NestedMeta, Path};

enum SupportedFormat {
	Full(Path),
	ReadOnly(Path),
}

pub(crate) fn parse(
	attr: AttributeArgs,
	input: ItemStruct,
	errors: &mut Vec<syn::Error>,
) -> proc_macro2::TokenStream {
	let mut desc = None;
	let mut supported_formats = Vec::new();
	let mut read_only_encountered = false;

	for nested_meta in attr {
		match nested_meta {
			NestedMeta::Meta(Meta::NameValue(mnv)) if mnv.path.is_ident("description") => {
				if desc.is_some() {
					bail!(errors, mnv.span(), "Duplicate `description` entry");
				}

				if let Lit::Str(s) = &mnv.lit {
					desc = Some(s.value());
					continue;
				} else {
					bail!(
						errors,
						Span::call_site(),
						"Invalid `description` entry, expected string value"
					);
				}
			},
			NestedMeta::Meta(Meta::List(list)) if list.path.is_ident("supported_formats") => {
				for nested in list.nested {
					if let NestedMeta::Meta(meta) = nested {
						match meta {
							Meta::Path(path) => {
								supported_formats.push(SupportedFormat::Full(path.clone()));
								continue;
							},
							Meta::List(list) if list.path.is_ident("read_only") => {
								if read_only_encountered {
									bail!(errors, list.path.span(), "Duplicate `read_only` entry");
								}

								read_only_encountered = true;

								for item in list.nested.iter() {
									if let NestedMeta::Meta(Meta::Path(path)) = item {
										supported_formats
											.push(SupportedFormat::ReadOnly(path.clone()));
										continue;
									}
								}

								continue;
							},
							_ => {},
						}
					}
				}

				continue;
			},
			_ => {
				bail!(
					errors,
					nested_meta.span(),
					"Unexpected input, check the format of the arguments"
				);
			},
		}
	}

	let ident = &input.ident;
	let supported_types_iter = supported_formats.iter().map(|format| match format {
		SupportedFormat::Full(path) => format!(
			"* [`FileType::{ft}`](crate::FileType::{ft})\n",
			ft = path.get_ident().unwrap()
		),
		SupportedFormat::ReadOnly(path) => format!(
			"* [`FileType::{ft}`](crate::FileType::{ft}) **(READ ONLY)**\n",
			ft = path.get_ident().unwrap()
		),
	});
	let flattened_file_types = supported_formats.iter().map(|format| match format {
		SupportedFormat::Full(path) | SupportedFormat::ReadOnly(path) => path,
	});
	let read_only_file_types = supported_formats.iter().filter_map(|format| match format {
		SupportedFormat::ReadOnly(path) => Some(path),
		_ => None,
	});

	quote! {
		use crate::_this_is_internal;

		#[doc = #desc]
		#[doc = "\n"]
		#[doc = "## Supported file types\n\n"]
		#( #[doc = #supported_types_iter] )*
		#[doc = "\n"]
		#input

		impl #ident {
			pub(crate) const SUPPORTED_FORMATS: &'static [lofty::FileType] = &[
				#( lofty::FileType:: #flattened_file_types ),*
			];

			pub(crate) const READ_ONLY_FORMATS: &'static [lofty::FileType] = &[
				#( lofty::FileType:: #read_only_file_types ),*
			];
		}
	}
}
