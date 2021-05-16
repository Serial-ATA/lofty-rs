#[doc(hidden)]
#[macro_export]
macro_rules! impl_tag {
	($tag:ident, $inner:ident, $tag_type:expr) => {
		#[doc(hidden)]
		pub struct $tag {
			inner: $inner,
			#[cfg(feature = "duration")]
			#[allow(dead_code)]
			duration: Option<Duration>,
		}

		impl Default for $tag {
			fn default() -> Self {
				Self {
					inner: $inner::default(),
					#[cfg(feature = "duration")]
					duration: None,
				}
			}
		}

		impl $tag {
			/// Creates a new default tag
			pub fn new() -> Self {
				Self::default()
			}
		}

		use std::any::Any;

		impl ToAnyTag for $tag {
			fn to_anytag(&self) -> AnyTag<'_> {
				self.into()
			}
		}

		impl ToAny for $tag {
			fn to_any(&self) -> &dyn Any {
				self
			}
			fn to_any_mut(&mut self) -> &mut dyn Any {
				self
			}
		}

		impl AudioTag for $tag {}

		// From wrapper to inner (same type)
		impl From<$tag> for $inner {
			fn from(inp: $tag) -> Self {
				inp.inner
			}
		}

		// From inner to wrapper (same type)
		impl From<$inner> for $tag {
			fn from(inp: $inner) -> Self {
				Self {
					inner: inp,
					#[cfg(feature = "duration")]
					duration: None,
				}
			}
		}

		impl<'a> From<&'a $tag> for AnyTag<'a> {
			fn from(inp: &'a $tag) -> Self {
				Self {
					title: inp.title(),
					artists: inp.artists_vec(),
					year: inp.year().map(|y| y as i32),
					album: Album::new(
						inp.album_title(),
						inp.album_artists_vec(),
						inp.album_cover(),
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

		impl<'a> From<AnyTag<'a>> for $tag {
			fn from(inp: AnyTag<'a>) -> Self {
				let mut tag = $tag::default();

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

		// From dyn AudioTag to wrapper (any type)
		impl From<Box<dyn AudioTag>> for $tag {
			fn from(inp: Box<dyn AudioTag>) -> Self {
				let mut inp = inp;
				if let Some(t_refmut) = inp.to_any_mut().downcast_mut::<$tag>() {
					let t = std::mem::replace(t_refmut, $tag::new()); // TODO: can we avoid creating the dummy tag?
					t
				} else {
					let mut t = inp.to_dyn_tag($tag_type);
					let t_refmut = t.to_any_mut().downcast_mut::<$tag>().unwrap();
					let t = std::mem::replace(t_refmut, $tag::new());
					t
				}
			}
		}

		// From dyn AudioTag to inner (any type)
		impl From<Box<dyn AudioTag>> for $inner {
			fn from(inp: Box<dyn AudioTag>) -> Self {
				let t: $tag = inp.into();
				t.into()
			}
		}
	};
}

/// Convert a concrete tag type into another
#[macro_export]
macro_rules! convert {
	($inp:expr, $target_type:ty) => {
		$target_type::from(inp.to_anytag())
	};
}
