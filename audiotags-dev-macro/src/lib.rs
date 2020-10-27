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
    ($tag:ident , $inner:ident) => {
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

        impl IntoAnyTag for $tag {
            fn into_anytag(&self) -> AnyTag<'_> {
                self.into()
            }
            fn into_any(&self) -> &dyn std::any::Any {
                self
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
    };
}
