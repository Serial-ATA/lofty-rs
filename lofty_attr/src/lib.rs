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

mod internal;
mod lofty_file;
mod lofty_tag;
mod util;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Data, DataStruct, DeriveInput, Fields, ItemStruct};

/// Creates a file usable by Lofty
///
/// See [here](https://github.com/Serial-ATA/lofty-rs/tree/main/examples/custom_resolver) for an example of how to use it.
#[proc_macro_derive(LoftyFile, attributes(lofty))]
pub fn lofty_file(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	let data_struct = match input.data {
		Data::Struct(
			ref data_struct @ DataStruct {
				fields: Fields::Named(_),
				..
			},
		) => data_struct,
		_ => {
			return TokenStream::from(
				util::err(
					input.ident.span(),
					"This macro can only be used on structs with named fields",
				)
				.to_compile_error(),
			);
		},
	};

	let mut errors = Vec::new();
	let ret = lofty_file::parse(&input, data_struct, &mut errors);

	finish(&ret, &errors)
}

#[proc_macro_attribute]
#[doc(hidden)]
pub fn tag(args_input: TokenStream, input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args_input as AttributeArgs);
	let input = parse_macro_input!(input as ItemStruct);

	let mut errors = Vec::new();
	let ret = lofty_tag::parse(args, input, &mut errors);

	finish(&ret, &errors)
}

fn finish(ret: &proc_macro2::TokenStream, errors: &[syn::Error]) -> TokenStream {
	let compile_errors = errors.iter().map(syn::Error::to_compile_error);

	TokenStream::from(quote! {
		#(#compile_errors)*
		#ret
	})
}
