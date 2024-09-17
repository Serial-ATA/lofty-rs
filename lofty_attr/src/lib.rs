//! Macros for [Lofty](https://crates.io/crates/lofty)

#![allow(
	unknown_lints,
	clippy::too_many_lines,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::cast_possible_wrap,
	clippy::cast_possible_truncation,
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
	clippy::doc_markdown,
	let_underscore_drop,
	clippy::match_wildcard_for_single_variants,
	clippy::semicolon_if_nothing_returned,
	clippy::new_without_default,
	clippy::from_over_into,
	clippy::upper_case_acronyms,
	clippy::single_match_else,
	clippy::similar_names,
	clippy::tabs_in_doc_comments,
	clippy::len_without_is_empty,
	clippy::needless_late_init,
	clippy::type_complexity,
	clippy::type_repetition_in_bounds,
	unused_qualifications,
	clippy::return_self_not_must_use,
	clippy::bool_to_int_with_if,
	clippy::uninlined_format_args, /* This should be changed for any normal "{}", but I'm not a fan of it for any debug or width specific formatting */
	clippy::manual_let_else,
	clippy::struct_excessive_bools,
	clippy::match_bool,
	clippy::needless_pass_by_value
)]

mod attribute;
mod ebml;
mod internal;
mod lofty_file;
mod lofty_tag;
mod util;

use crate::lofty_file::LoftyFile;
use crate::lofty_tag::{LoftyTag, LoftyTagAttribute};

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

/// Creates a file usable by Lofty
///
/// See [here](https://github.com/Serial-ATA/lofty-rs/tree/main/examples/custom_resolver) for an example of how to use it.
#[proc_macro_derive(LoftyFile, attributes(lofty))]
pub fn lofty_file(input: TokenStream) -> TokenStream {
	let lofty_file = parse_macro_input!(input as LoftyFile);
	match lofty_file.emit() {
		Ok(ret) => ret,
		Err(e) => e.to_compile_error().into(),
	}
}

#[proc_macro_attribute]
#[doc(hidden)]
pub fn tag(args_input: TokenStream, input: TokenStream) -> TokenStream {
	let attribute = parse_macro_input!(args_input as LoftyTagAttribute);
	let input = parse_macro_input!(input as ItemStruct);

	let lofty_tag = LoftyTag::new(attribute, input);
	lofty_tag.emit()
}

#[proc_macro]
#[doc(hidden)]
pub fn ebml_master_elements(input: TokenStream) -> TokenStream {
	let ret = ebml::parse_ebml_master_elements(input);

	if let Err(err) = ret {
		return TokenStream::from(err.to_compile_error());
	}

	let (identifiers, elements) = ret.unwrap();
	let elements_map_inserts = elements.iter().map(|element| {
		let readable_ident = &element.readable_ident;
		let id = element.info.id;
		let children = element.info.children.iter().map(|child| {
			let readable_ident = &child.readable_ident;
			let id = child.info.id;
			let data_type = &child.info.data_type;
			quote! {
				(ElementId(#id), ChildElementDescriptor {
					ident: ElementIdent::#readable_ident,
					data_type: ElementDataType::#data_type,
				})
			}
		});

		quote! {
			m.insert(
				ElementId(#id),
				MasterElement {
					id: ElementIdent::#readable_ident,
					children: &[#( #children ),*][..]
				}
			);
		}
	});

	let mut ident_variants = Vec::new();
	for (ident, id) in &identifiers {
		ident_variants.push(quote! {
			#ident = #id,
		});
	}

	TokenStream::from(quote! {
		#[derive(Copy, Clone, Eq, PartialEq, Debug)]
		#[repr(u64)]
		pub(crate) enum ElementIdent {
			#( #ident_variants )*
		}

		fn master_elements() -> &'static ::std::collections::HashMap<ElementId, MasterElement> {
			static INSTANCE: ::std::sync::OnceLock<::std::collections::HashMap<ElementId, MasterElement>> = ::std::sync::OnceLock::new();
			INSTANCE.get_or_init(|| {
				let mut m = ::std::collections::HashMap::new();
				#( #elements_map_inserts )*
				m
			})
		}
	})
}
