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

	finish(ret, errors)
}

#[proc_macro_attribute]
#[doc(hidden)]
pub fn tag(args_input: TokenStream, input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args_input as AttributeArgs);
	let input = parse_macro_input!(input as ItemStruct);

	let mut errors = Vec::new();
	let ret = lofty_tag::parse(args, input, &mut errors);

	finish(ret, errors)
}

fn finish(ret: proc_macro2::TokenStream, errors: Vec<syn::Error>) -> TokenStream {
	let compile_errors = errors.iter().map(syn::Error::to_compile_error);

	TokenStream::from(quote! {
		#(#compile_errors)*
		#ret
	})
}
