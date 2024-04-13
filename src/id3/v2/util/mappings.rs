use crate::tag::ItemKey;

pub(crate) const TIPL_MAPPINGS: &[(ItemKey, &str)] = &[
	(ItemKey::Producer, "producer"),
	(ItemKey::Arranger, "arranger"),
	(ItemKey::Engineer, "engineer"),
	(ItemKey::MixDj, "DJ-mix"),
	(ItemKey::MixEngineer, "mix"),
];
