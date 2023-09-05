use crate::properties::FileProperties;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlHeaderProperties {
	pub(crate) version: u64,
	pub(crate) read_version: u64,
	pub(crate) max_id_length: u8,
	pub(crate) max_size_length: u8,
	pub(crate) doc_type: String,
	pub(crate) doc_type_version: u64,
	pub(crate) doc_type_read_version: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlExtension {
	pub(crate) name: String,
	pub(crate) version: u64,
}

/// EBML audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EbmlProperties {
	pub(crate) header: EbmlHeaderProperties,
	pub(crate) extensions: Vec<EbmlExtension>,
}

impl From<EbmlProperties> for FileProperties {
	fn from(_input: EbmlProperties) -> Self {
		todo!()
	}
}
