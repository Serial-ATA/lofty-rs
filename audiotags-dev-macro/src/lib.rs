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
        impl AudioTagCommon for $tag {
            fn config(&self) -> &Config {
                &self.config
            }
            fn set_config(&mut self, config: Config) {
                self.config = config.clone();
            }
            fn into_anytag(&self) -> AnyTag<'_> {
                self.into()
            }
        }
    };
}
