mod internal;
mod lofty_file;
mod lofty_tag;
mod util;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

/// Creates a file usable by Lofty
///
/// See [here](https://github.com/Serial-ATA/lofty-rs/tree/main/examples/custom_resolver) for an example of how to use it.
#[proc_macro_derive(LoftyFile, attributes(lofty))]
pub fn lofty_file(input: TokenStream) -> TokenStream {
	act(input, lofty_file::parse)
}

#[proc_macro_derive(LoftyTag, attributes(lofty))]
#[doc(hidden)]
pub fn lofty_tag(input: TokenStream) -> TokenStream {
	act(input, lofty_tag::parse)
}

fn act(
	input: TokenStream,
	func: impl Fn(&DeriveInput, &DataStruct, &mut Vec<syn::Error>) -> proc_macro2::TokenStream,
) -> TokenStream {
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
	let ret = func(&input, data_struct, &mut errors);

	let compile_errors = errors.iter().map(syn::Error::to_compile_error);

	TokenStream::from(quote! {
		#(#compile_errors)*
		#ret
	})
}
