use crate::properties::FileProperties;

/// EBML audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlProperties {}

impl From<EbmlProperties> for FileProperties {
	fn from(_input: EbmlProperties) -> Self {
		todo!()
	}
}
