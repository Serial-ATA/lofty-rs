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

        impl IntoAnyTag for $tag {
            fn into_anytag(&self) -> AnyTag<'_> {
                self.into()
            }
            fn into_any(&self) -> &dyn Any {
                self
            }
            fn into_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        impl $tag {
            pub(crate) fn into_any_owned(self) -> Box<dyn Any> {
                Box::new(self)
            }
        }

        impl AudioTag for $tag {}

        impl From<&$tag> for $inner {
            fn from(inp: &$tag) -> Self {
                inp.inner.clone()
            }
        }

        impl From<$inner> for $tag {
            fn from(inp: $inner) -> Self {
                Self {
                    inner: inp,
                    config: Config::default(),
                }
            }
        }

        // downcasting

        // impl<'a> std::convert::TryFrom<&'a Box<dyn AudioTag>> for &'a $tag {
        //     type Error = crate::Error;
        //     fn try_from(inp: &'a Box<dyn AudioTag>) -> crate::Result<Self> {
        //         inp.into_any()
        //             .downcast_ref::<$tag>()
        //             .ok_or(crate::Error::DowncastError)
        //     }
        // }

        impl From<Box<dyn AudioTag>> for $tag {
            fn from(inp: Box<dyn AudioTag>) -> Self {
                let mut inp = inp;
                if let Some(t_refmut) = inp.into_any_mut().downcast_mut::<$tag>() {
                    let t = std::mem::replace(t_refmut, $tag::new()); // TODO: can we avoid creating the dummy tag?
                    t
                } else {
                    let mut t = inp.into_tag($tag_type);
                    let t_refmut = t.into_any_mut().downcast_mut::<$tag>().unwrap();
                    let t = std::mem::replace(t_refmut, $tag::new());
                    t
                }
            }
        }

        // impl std::convert::TryFrom<Box<dyn AudioTag>> for $inner {
        //     type Error = crate::Error;
        //     fn try_from(inp: Box<dyn AudioTag>) -> crate::Result<Self> {
        //         let t: &$tag = inp
        //             .into_any()
        //             .downcast_ref::<$tag>()
        //             .ok_or(crate::Error::DowncastError)?;
        //         Ok(t.into())
        //     }
        // }

        impl std::convert::From<Box<dyn AudioTag>> for $inner {
            fn from(inp: Box<dyn AudioTag>) -> Self {
                let t: $tag = inp.into();
                (&t).into()
            }
        }
    };
}
