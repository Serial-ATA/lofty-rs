use crate::attribute::AttributeValue;
use crate::util;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Expr, Fields, Path};

pub struct LoftyError {
	pub(crate) errors: Vec<syn::Error>,
	input: DeriveInput,
	message: String,
	source: Option<ErrorSourceAttr>,
}

impl Parse for LoftyError {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let input: DeriveInput = input.parse()?;

		let data_struct = match input.data {
			Data::Struct(
				ref data_struct @ DataStruct {
					fields: Fields::Named(_) | Fields::Unit,
					..
				},
			) => data_struct,
			_ => {
				return Err(util::err(
					input.ident.span(),
					"This macro can only be used on empty structs or structs with named fields",
				));
			},
		};

		let mut errors = Vec::new();
		let mut message = None;
		for attr in &input.attrs {
			match AttributeValue::from_attribute("error", attr) {
				Ok(Some(AttributeValue::NameValue(lhs, rhs))) if lhs == "message" => {
					message = Some(rhs.value());
				},
				Ok(None) => {},
				Ok(_) => errors.push(util::err(attr.span(), "unexpected attribute format")),
				Err(e) => errors.push(e),
			}
		}

		if message.is_none() {
			errors.push(util::err(input.ident.span(), "missing message attribute"));
		}

		let mut source_field = None;

		let has_fields = matches!(data_struct.fields, Fields::Named(_));
		for field in &data_struct.fields {
			if field.ident.as_ref().expect("should exist") == "source" {
				source_field = Some(field);
				continue;
			}

			errors.push(util::err(
				field.ident.span(),
				"`LoftyError`s should only have a `source` field",
			));
		}

		let source = match source_field {
			Some(field) => Some(ErrorSourceAttr::from_attr(&mut errors, &field.attrs)),
			None => {
				if has_fields {
					errors.push(util::err(input.ident.span(), "missing a `source` field"));
				}

				None
			},
		};

		Ok(LoftyError {
			errors,
			input,
			source,
			message: message.unwrap_or_default(),
		})
	}
}

#[derive(Default)]
struct ErrorSourceAttr {
	from_types: Vec<Path>,
}

impl ErrorSourceAttr {
	fn from_attr(errors: &mut Vec<syn::Error>, attrs: &[syn::Attribute]) -> Self {
		let mut from_types = Vec::new();
		for attr in attrs {
			match AttributeValue::from_attribute("error", attr) {
				Ok(Some(AttributeValue::SingleList(list_name, values))) if list_name == "from" => {
					for value in values {
						let Expr::Path(p) = value else {
							errors.push(util::err(value.span(), "unexpected expression"));
							continue;
						};

						from_types.push(p.path);
					}
				},
				Ok(None) => {},
				Ok(_) => errors.push(util::err(attr.span(), "unexpected attribute format")),
				Err(e) => errors.push(e),
			}
		}

		Self { from_types }
	}

	fn generate_impls(&self, target: &Ident) -> TokenStream {
		let mut impls = TokenStream::new();
		for from_type in &self.from_types {
			impls.extend(quote_spanned! {target.span()=>
				impl ::core::convert::From<#from_type> for #target {
					fn from(value: #from_type) -> #target {
						#target {
							source: value.into(),
						}
					}
				}
			});
		}

		impls
	}
}

impl LoftyError {
	pub(crate) fn emit(self) -> proc_macro::TokenStream {
		let mut ret = TokenStream::new();
		for error in self.errors {
			ret.extend(error.to_compile_error());
		}

		let target_ident = self.input.ident;
		let mut core_error_source_impl = TokenStream::new();
		if let Some(source) = self.source {
			ret.extend(source.generate_impls(&target_ident));

			core_error_source_impl.extend(quote! {
				fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
					Some(&*self.source)
				}
			})
		}

		let message = self.message;
		let error_impl = quote! {
			impl ::core::fmt::Display for #target_ident {
				fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
					f.write_str(#message)
				}
			}

			impl ::core::fmt::Debug for #target_ident {
				fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
					f.debug_struct("").finish_non_exhaustive()
				}
			}

			impl ::core::error::Error for #target_ident {
				#core_error_source_impl
			}
		};

		ret.extend(error_impl);

		proc_macro::TokenStream::from(ret)
	}
}
