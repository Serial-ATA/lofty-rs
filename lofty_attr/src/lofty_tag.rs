use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
	Error, Expr, ExprLit, ExprPath, ItemStruct, Lit, LitStr, Meta, MetaList, MetaNameValue, Path,
	Result, Token,
};

enum SupportedFormat {
	Full(Path),
	ReadOnly(Path),
}

impl SupportedFormat {
	fn emit_doc_comment(&self) -> String {
		match self {
			SupportedFormat::Full(path) => format!(
				"* [`FileType::{ft}`](crate::file::FileType::{ft})\n",
				ft = path.get_ident().unwrap()
			),
			SupportedFormat::ReadOnly(path) => format!(
				"* [`FileType::{ft}`](crate::file::FileType::{ft}) **(READ ONLY)**\n",
				ft = path.get_ident().unwrap()
			),
		}
	}

	fn path(&self) -> &Path {
		match self {
			SupportedFormat::Full(path) | SupportedFormat::ReadOnly(path) => path,
		}
	}

	fn read_only(&self) -> Option<&Path> {
		match self {
			SupportedFormat::ReadOnly(path) => Some(path),
			_ => None,
		}
	}
}

struct ConflictingFormat<'a>(&'a Path);

impl ConflictingFormat<'_> {
	fn emit_doc_comment(self) -> String {
		format!(
			"* [`TagType::{ty}`](crate::tag::TagType::{ty})\n",
			ty = self.0.get_ident().unwrap()
		)
	}
}

pub(crate) struct LoftyTag {
	attribute: LoftyTagAttribute,
	input: ItemStruct,
}

impl LoftyTag {
	pub(crate) fn new(attribute: LoftyTagAttribute, input: ItemStruct) -> Self {
		LoftyTag { attribute, input }
	}

	pub(crate) fn emit(&self) -> TokenStream {
		let ident = &self.input.ident;
		let desc = &self.attribute.description;
		let tag_type = &self.attribute.tag_type;

		let supported_types_iter = self
			.attribute
			.supported_formats
			.iter()
			.map(SupportedFormat::emit_doc_comment);
		let flattened_file_types = self
			.attribute
			.supported_formats
			.iter()
			.map(SupportedFormat::path);
		let read_only_file_types = self
			.attribute
			.supported_formats
			.iter()
			.filter_map(SupportedFormat::read_only);

		let conflicting_tag_types_iter = self
			.attribute
			.conflicting_tags
			.iter()
			.map(ConflictingFormat)
			.map(ConflictingFormat::emit_doc_comment);
		let conflict_docs = if self.attribute.conflicting_tags.is_empty() {
			quote! {}
		} else {
			quote! {
				/// ## Conflicting tag types
				///
				/// This tag has conflicts with the following tag types. This means it cannot
				/// be written to a file alongside any of them. Conflicts must be resolved manually,
				/// or writes will fail.
				///
				#( #[doc = #conflicting_tag_types_iter] )*
				///
			}
		};

		let conflicting_tag_types_iter = self.attribute.conflicting_tags.iter();
		let conflict_fn_body = if self.attribute.conflicting_tags.is_empty() {
			quote! { false }
		} else {
			quote! {
				matches!(
					tag_type,
					#( ::lofty::tag::TagType:: #conflicting_tag_types_iter)|*
				)
			}
		};

		let conflicting_tag_types_iter = self.attribute.conflicting_tags.iter();
		let input = &self.input;
		TokenStream::from(quote! {
			use ::lofty::_this_is_internal;

			#[doc = #desc]
			///
			/// ## Supported file types
			///
			///
			#( #[doc = #supported_types_iter] )*
			///
			#conflict_docs
			#input

			const _: () = {
				let mut i = 0;
				while i < #ident::CONFLICTING_FORMATS.len() {
					let conflicting_ty = #ident::CONFLICTING_FORMATS[i];
					assert!(
						conflicting_ty.conflicts_with(::lofty::tag::TagType:: #tag_type),
						"conflicts must be commutative! make sure all conflicting types conflict with this type"
					);
					i += 1;
				}
			};

			impl #ident {
				pub(crate) const SUPPORTED_FORMATS: &'static [::lofty::file::FileType] = &[
					#( ::lofty::file::FileType:: #flattened_file_types ),*
				];

				pub(crate) const READ_ONLY_FORMATS: &'static [::lofty::file::FileType] = &[
					#( ::lofty::file::FileType:: #read_only_file_types ),*
				];

				pub(crate) const CONFLICTING_FORMATS: &'static [::lofty::tag::TagType] = &[
					#( ::lofty::tag::TagType:: #conflicting_tag_types_iter ),*
				];

				pub(crate) const fn conflicts_with(tag_type: ::lofty::tag::TagType) -> bool {
					#conflict_fn_body
				}
			}
		})
	}
}

pub(crate) struct LoftyTagAttribute {
	description: LitStr,
	tag_type: Path,
	supported_formats: Vec<SupportedFormat>,
	conflicting_tags: Vec<Path>,
}

impl Parse for LoftyTagAttribute {
	fn parse(input: ParseStream<'_>) -> Result<Self> {
		let mut description = None;
		let mut tag_type = None;
		let mut supported_formats = Vec::new();
		let mut conflicting_tags = Vec::new();

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
				Meta::NameValue(mnv) if mnv.path.is_ident("tag_type") => {
					if tag_type.is_some() {
						return Err(Error::new(mnv.span(), "Duplicate `tag_type` entry"));
					}

					tag_type = Some(parse_tag_type(mnv)?);
				},
				Meta::List(list) if list.path.is_ident("supported_formats") => {
					parse_supported_formats(list, &mut supported_formats)?;
				},
				Meta::List(list) if list.path.is_ident("conflicts_with") => {
					parse_conflicts_with(list, &mut conflicting_tags)?;
				},
				_ => {
					return Err(Error::new(
						nested_meta.span(),
						"Unexpected input, check the format of the arguments",
					));
				},
			}
		}

		if description.is_none() {
			return Err(Error::new(start_span, "No description provided"));
		}

		Ok(Self {
			description: description.unwrap(),
			tag_type: tag_type.unwrap(),
			supported_formats,
			conflicting_tags,
		})
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

fn parse_tag_type(name_value: MetaNameValue) -> Result<Path> {
	match name_value.value {
		Expr::Path(ExprPath { path, .. }) => Ok(path),
		_ => Err(Error::new(
			name_value.span(),
			"Invalid `tag_type` entry, expected identifier",
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

fn parse_conflicts_with(meta_list: MetaList, conflicts_with: &mut Vec<Path>) -> Result<()> {
	meta_list.parse_nested_meta(|meta| {
		conflicts_with.push(meta.path);
		Ok(())
	})
}
