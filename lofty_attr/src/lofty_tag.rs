use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
	Error, Expr, ExprLit, ItemStruct, Lit, LitStr, Meta, MetaList, MetaNameValue, Path, Result,
	Token,
};

enum SupportedFormat {
	Full(Path),
	ReadOnly(Path),
}

pub(crate) struct LoftyTagAttribute {
	description: LitStr,
	supported_formats: Vec<SupportedFormat>,
}

impl Parse for LoftyTagAttribute {
	fn parse(input: ParseStream) -> Result<Self> {
		let mut description = None;
		let mut supported_formats = Vec::new();

		let start_span = input.span();

		let args = Punctuated::<Meta, Token![,]>::parse_separated_nonempty(input)?;
		for nested_meta in args {
			match nested_meta {
				Meta::NameValue(mnv) if mnv.path.is_ident("description") => {
					if description.is_some() {
						return Err(Error::new(mnv.span(), "Duplicate `description` entry"));
					}

					description = Some(parse_description(mnv)?);
				},
				Meta::List(list) if list.path.is_ident("supported_formats") => {
					parse_supported_formats(list, &mut supported_formats)?;
				},
				_ => {
					return Err(Error::new(
						nested_meta.span(),
						"Unexpected input, check the format of the arguments",
					))
				},
			}
		}

		if description.is_none() {
			return Err(Error::new(start_span, "No description provided"));
		}

		Ok(Self {
			description: description.unwrap(),
			supported_formats,
		})
	}
}

pub(crate) fn create(
	lofty_tag_attribute: LoftyTagAttribute,
	input: ItemStruct,
) -> proc_macro2::TokenStream {
	let ident = &input.ident;
	let desc = lofty_tag_attribute.description;

	let supported_types_iter =
		lofty_tag_attribute
			.supported_formats
			.iter()
			.map(|format| match format {
				SupportedFormat::Full(path) => format!(
					"* [`FileType::{ft}`](crate::FileType::{ft})\n",
					ft = path.get_ident().unwrap()
				),
				SupportedFormat::ReadOnly(path) => format!(
					"* [`FileType::{ft}`](crate::FileType::{ft}) **(READ ONLY)**\n",
					ft = path.get_ident().unwrap()
				),
			});
	let flattened_file_types =
		lofty_tag_attribute
			.supported_formats
			.iter()
			.map(|format| match format {
				SupportedFormat::Full(path) | SupportedFormat::ReadOnly(path) => path,
			});
	let read_only_file_types = lofty_tag_attribute
		.supported_formats
		.iter()
		.filter_map(|format| match format {
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
			pub(crate) const SUPPORTED_FORMATS: &'static [::lofty::FileType] = &[
				#( ::lofty::FileType:: #flattened_file_types ),*
			];

			pub(crate) const READ_ONLY_FORMATS: &'static [::lofty::FileType] = &[
				#( ::lofty::FileType:: #read_only_file_types ),*
			];
		}
	}
}

fn parse_description(name_value: MetaNameValue) -> Result<LitStr> {
	match name_value.value {
		Expr::Lit(ExprLit {
			lit: Lit::Str(lit_str),
			..
		}) => Ok(lit_str),
		_ => Err(Error::new(
			name_value.span(),
			"Invalid `description` entry, expected string value",
		)),
	}
}

fn parse_supported_formats(
	meta_list: MetaList,
	supported_formats: &mut Vec<SupportedFormat>,
) -> Result<()> {
	let mut read_only_encountered = false;
	meta_list.parse_nested_meta(|meta| {
		if meta.path.is_ident("read_only") {
			if read_only_encountered {
				return Err(meta.error("Duplicate `read_only` entry"));
			}

			read_only_encountered = true;

			meta.parse_nested_meta(|nested_meta| {
				supported_formats.push(SupportedFormat::ReadOnly(nested_meta.path));
				Ok(())
			})?;
		} else {
			supported_formats.push(SupportedFormat::Full(meta.path));
		}

		Ok(())
	})
}
