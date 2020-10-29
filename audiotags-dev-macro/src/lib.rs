#[macro_export]
macro_rules! impl_audiotag_config {
    ($tag:ident) => {
        impl AudioTagConfig for $tag {
            fn config(&self) -> &Config {
                &self.config
            }
            fn set_config(&mut self, config: Config) {
                self.config = config.clone();
            }
        }
    };
}

#[macro_export]
macro_rules! impl_tag {
    ($tag:ident , $inner:ident, $tag_type:expr) => {
        #[derive(Default)]
        pub struct $tag {
            inner: $inner,
            config: Config,
        }
        impl $tag {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
                Ok(Self {
                    inner: $inner::read_from_path(path)?,
                    config: Config::default(),
                })
            }
        }
        impl_audiotag_config!($tag);

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
                    config: Config::default(),
                }
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
        impl std::convert::From<Box<dyn AudioTag>> for $inner {
            fn from(inp: Box<dyn AudioTag>) -> Self {
                let t: $tag = inp.into();
                t.into()
            }
        }
    };
}
