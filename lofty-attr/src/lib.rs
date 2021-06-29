use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Error, ItemStruct, Meta, NestedMeta};

#[proc_macro_attribute]
#[allow(clippy::too_many_lines)]
pub fn impl_tag(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemStruct);
	let args = parse_macro_input!(args as AttributeArgs);

	if args.len() != 2 {
		return Error::new(
			input.ident.span(),
			"impl_tag requires an inner tag and TagType",
		)
		.to_compile_error()
		.into();
	}

	if let (NestedMeta::Meta(Meta::Path(inner)), NestedMeta::Meta(tag_type)) =
		(args[0].clone(), args[1].clone())
	{
		if let Some(inner) = inner.get_ident() {
			let input_ident = input.ident;

			let mut anytag_convert = quote! {
				impl<'a> From<AnyTag<'a>> for #input_ident {
					fn from(inp: AnyTag<'a>) -> Self {
						let mut tag = #input_ident::default();

						if let Some(v) = inp.title() {
							tag.set_title(v)
						}
						if let Some(v) = inp.artists_as_string() {
							tag.set_artist(&v)
						}
						if let Some(v) = inp.year {
							tag.set_year(v)
						}
						if let Some(v) = inp.album().title {
							tag.set_album_title(v)
						}
						if let Some(v) = inp.album().artists {
							tag.set_album_artist(&v.join("/"))
						}
						if let Some(v) = inp.track_number() {
							tag.set_track_number(v)
						}
						if let Some(v) = inp.total_tracks() {
							tag.set_total_tracks(v)
						}
						if let Some(v) = inp.disc_number() {
							tag.set_disc_number(v)
						}
						if let Some(v) = inp.total_discs() {
							tag.set_total_discs(v)
						}

						tag
					}
				}
			};

			if let Some(attr) = input.attrs.first() {
				if attr.path.is_ident("custom_convert") {
					anytag_convert = quote! {};
				}
			}

			let expanded = quote! {
				#[doc(hidden)]
				pub struct #input_ident {
					inner: #inner,
				}

				impl Default for #input_ident {
					fn default() -> Self {
						Self {
							inner: #inner::default(),
						}
					}
				}

				impl #input_ident {
					/// Creates a new default tag
					pub fn new() -> Self {
						Self::default()
					}
				}

				use std::any::Any;

				impl ToAnyTag for #input_ident {
					fn to_anytag(&self) -> AnyTag<'_> {
						self.into()
					}
				}

				impl ToAny for #input_ident {
					fn to_any(&self) -> &dyn Any {
						self
					}
					fn to_any_mut(&mut self) -> &mut dyn Any {
						self
					}
				}

				impl AudioTag for #input_ident {}

				// From wrapper to inner (same type)
				impl From<#input_ident> for #inner {
					fn from(inp: #input_ident) -> Self {
						inp.inner
					}
				}

				// From inner to wrapper (same type)
				impl From<#inner> for #input_ident {
					fn from(inp: #inner) -> Self {
						Self {
							inner: inp,
							#[cfg(feature = "duration")]
							duration: None,
						}
					}
				}

				impl<'a> From<&'a #input_ident> for AnyTag<'a> {
					fn from(inp: &'a #input_ident) -> Self {
						Self {
							title: inp.title(),
							artists: inp.artists_vec(),
							year: inp.year().map(|y| y as i32),
							album: Album::new(
								inp.album_title(),
								inp.album_artists_vec(),
								inp.album_covers(),
							),
							track_number: inp.track_number(),
							total_tracks: inp.total_tracks(),
							disc_number: inp.disc_number(),
							total_discs: inp.total_discs(),
							comments: None, // TODO
							date: inp.date(),
						}
					}
				}

				#anytag_convert

				// From dyn AudioTag to wrapper (any type)
				impl From<Box<dyn AudioTag>> for #input_ident {
					fn from(inp: Box<dyn AudioTag>) -> Self {
						let mut inp = inp;
						if let Some(t_refmut) = inp.to_any_mut().downcast_mut::<#input_ident>() {
							let t = std::mem::replace(t_refmut, #input_ident::new()); // TODO: can we avoid creating the dummy tag?
							t
						} else {
							let mut t = inp.to_dyn_tag(#tag_type);
							let t_refmut = t.to_any_mut().downcast_mut::<#input_ident>().unwrap();
							let t = std::mem::replace(t_refmut, #input_ident::new());
							t
						}
					}
				}

				// From dyn AudioTag to inner (any type)
				impl From<Box<dyn AudioTag>> for #inner {
					fn from(inp: Box<dyn AudioTag>) -> Self {
						let t: #input_ident = inp.into();
						t.into()
					}
				}
			};

			return TokenStream::from(expanded);
		}
	}

	Error::new(input.ident.span(), "impl_tag provided invalid arguments")
		.to_compile_error()
		.into()
}
