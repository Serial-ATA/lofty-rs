// !!! DO NOT EDIT !!!
// !!! THIS FILE IS GENERATED BY `scripts/update-matroska-tags.py` !!!

use std::borrow::Cow;

/// A list of all specified Matroska tag names
///
/// The tag list is available [here](https://matroska.org/technical/tagging.html). It provides
/// descriptions and expected data types of each tag.
#[rustfmt::skip]
#[allow(missing_docs)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TagName {

	// Nesting Information
	Original,
	Sample,
	Country,

	// Organization Information
	TotalParts,
	PartNumber,
	PartOffset,

	// Titles
	Title,
	Subtitle,

	// Nested Information
	Url,
	SortWith,
	Instruments,
	Email,
	Address,
	Fax,
	Phone,

	// Entities
	Artist,
	LeadPerformer,
	Accompaniment,
	Composer,
	Arranger,
	Lyrics,
	Lyricist,
	Conductor,
	Director,
	AssistantDirector,
	DirectorOfPhotography,
	SoundEngineer,
	ArtDirector,
	ProductionDesigner,
	Choregrapher,
	CostumeDesigner,
	Actor,
	Character,
	WrittenBy,
	ScreenplayBy,
	EditedBy,
	Producer,
	Coproducer,
	ExecutiveProducer,
	DistributedBy,
	MasteredBy,
	EncodedBy,
	MixedBy,
	RemixedBy,
	ProductionStudio,
	ThanksTo,
	Publisher,
	Label,

	// Search and Classification
	Genre,
	Mood,
	OriginalMediaType,
	ContentType,
	Subject,
	Description,
	Keywords,
	Summary,
	Synopsis,
	InitialKey,
	Period,
	LawRating,

	// Temporal Information
	DateReleased,
	DateRecorded,
	DateEncoded,
	DateTagged,
	DateDigitized,
	DateWritten,
	DatePurchased,

	// Spatial Information
	RecordingLocation,
	CompositionLocation,
	ComposerNationality,

	// Personal
	Comment,
	PlayCounter,
	Rating,

	// Technical Information
	Encoder,
	EncoderSettings,
	Bps,
	Fps,
	Bpm,
	Measure,
	Tuning,
	ReplaygainGain,
	ReplaygainPeak,

	// Identifiers
	Isrc,
	Mcdi,
	Isbn,
	Barcode,
	CatalogNumber,
	LabelCode,
	Lccn,
	Imdb,
	Tmdb,
	Tvdb,
	Tvdb2,

	// Commercial
	PurchaseItem,
	PurchaseInfo,
	PurchaseOwner,
	PurchasePrice,
	PurchaseCurrency,

	// Legal
	Copyright,
	ProductionCopyright,
	License,
	TermsOfUse,
}

impl From<TagName> for Cow<'static, str> {
	fn from(value: TagName) -> Self {
		match value {
			TagName::Original => Cow::Borrowed("ORIGINAL"),
			TagName::Sample => Cow::Borrowed("SAMPLE"),
			TagName::Country => Cow::Borrowed("COUNTRY"),
			TagName::TotalParts => Cow::Borrowed("TOTAL_PARTS"),
			TagName::PartNumber => Cow::Borrowed("PART_NUMBER"),
			TagName::PartOffset => Cow::Borrowed("PART_OFFSET"),
			TagName::Title => Cow::Borrowed("TITLE"),
			TagName::Subtitle => Cow::Borrowed("SUBTITLE"),
			TagName::Url => Cow::Borrowed("URL"),
			TagName::SortWith => Cow::Borrowed("SORT_WITH"),
			TagName::Instruments => Cow::Borrowed("INSTRUMENTS"),
			TagName::Email => Cow::Borrowed("EMAIL"),
			TagName::Address => Cow::Borrowed("ADDRESS"),
			TagName::Fax => Cow::Borrowed("FAX"),
			TagName::Phone => Cow::Borrowed("PHONE"),
			TagName::Artist => Cow::Borrowed("ARTIST"),
			TagName::LeadPerformer => Cow::Borrowed("LEAD_PERFORMER"),
			TagName::Accompaniment => Cow::Borrowed("ACCOMPANIMENT"),
			TagName::Composer => Cow::Borrowed("COMPOSER"),
			TagName::Arranger => Cow::Borrowed("ARRANGER"),
			TagName::Lyrics => Cow::Borrowed("LYRICS"),
			TagName::Lyricist => Cow::Borrowed("LYRICIST"),
			TagName::Conductor => Cow::Borrowed("CONDUCTOR"),
			TagName::Director => Cow::Borrowed("DIRECTOR"),
			TagName::AssistantDirector => Cow::Borrowed("ASSISTANT_DIRECTOR"),
			TagName::DirectorOfPhotography => Cow::Borrowed("DIRECTOR_OF_PHOTOGRAPHY"),
			TagName::SoundEngineer => Cow::Borrowed("SOUND_ENGINEER"),
			TagName::ArtDirector => Cow::Borrowed("ART_DIRECTOR"),
			TagName::ProductionDesigner => Cow::Borrowed("PRODUCTION_DESIGNER"),
			TagName::Choregrapher => Cow::Borrowed("CHOREGRAPHER"),
			TagName::CostumeDesigner => Cow::Borrowed("COSTUME_DESIGNER"),
			TagName::Actor => Cow::Borrowed("ACTOR"),
			TagName::Character => Cow::Borrowed("CHARACTER"),
			TagName::WrittenBy => Cow::Borrowed("WRITTEN_BY"),
			TagName::ScreenplayBy => Cow::Borrowed("SCREENPLAY_BY"),
			TagName::EditedBy => Cow::Borrowed("EDITED_BY"),
			TagName::Producer => Cow::Borrowed("PRODUCER"),
			TagName::Coproducer => Cow::Borrowed("COPRODUCER"),
			TagName::ExecutiveProducer => Cow::Borrowed("EXECUTIVE_PRODUCER"),
			TagName::DistributedBy => Cow::Borrowed("DISTRIBUTED_BY"),
			TagName::MasteredBy => Cow::Borrowed("MASTERED_BY"),
			TagName::EncodedBy => Cow::Borrowed("ENCODED_BY"),
			TagName::MixedBy => Cow::Borrowed("MIXED_BY"),
			TagName::RemixedBy => Cow::Borrowed("REMIXED_BY"),
			TagName::ProductionStudio => Cow::Borrowed("PRODUCTION_STUDIO"),
			TagName::ThanksTo => Cow::Borrowed("THANKS_TO"),
			TagName::Publisher => Cow::Borrowed("PUBLISHER"),
			TagName::Label => Cow::Borrowed("LABEL"),
			TagName::Genre => Cow::Borrowed("GENRE"),
			TagName::Mood => Cow::Borrowed("MOOD"),
			TagName::OriginalMediaType => Cow::Borrowed("ORIGINAL_MEDIA_TYPE"),
			TagName::ContentType => Cow::Borrowed("CONTENT_TYPE"),
			TagName::Subject => Cow::Borrowed("SUBJECT"),
			TagName::Description => Cow::Borrowed("DESCRIPTION"),
			TagName::Keywords => Cow::Borrowed("KEYWORDS"),
			TagName::Summary => Cow::Borrowed("SUMMARY"),
			TagName::Synopsis => Cow::Borrowed("SYNOPSIS"),
			TagName::InitialKey => Cow::Borrowed("INITIAL_KEY"),
			TagName::Period => Cow::Borrowed("PERIOD"),
			TagName::LawRating => Cow::Borrowed("LAW_RATING"),
			TagName::DateReleased => Cow::Borrowed("DATE_RELEASED"),
			TagName::DateRecorded => Cow::Borrowed("DATE_RECORDED"),
			TagName::DateEncoded => Cow::Borrowed("DATE_ENCODED"),
			TagName::DateTagged => Cow::Borrowed("DATE_TAGGED"),
			TagName::DateDigitized => Cow::Borrowed("DATE_DIGITIZED"),
			TagName::DateWritten => Cow::Borrowed("DATE_WRITTEN"),
			TagName::DatePurchased => Cow::Borrowed("DATE_PURCHASED"),
			TagName::RecordingLocation => Cow::Borrowed("RECORDING_LOCATION"),
			TagName::CompositionLocation => Cow::Borrowed("COMPOSITION_LOCATION"),
			TagName::ComposerNationality => Cow::Borrowed("COMPOSER_NATIONALITY"),
			TagName::Comment => Cow::Borrowed("COMMENT"),
			TagName::PlayCounter => Cow::Borrowed("PLAY_COUNTER"),
			TagName::Rating => Cow::Borrowed("RATING"),
			TagName::Encoder => Cow::Borrowed("ENCODER"),
			TagName::EncoderSettings => Cow::Borrowed("ENCODER_SETTINGS"),
			TagName::Bps => Cow::Borrowed("BPS"),
			TagName::Fps => Cow::Borrowed("FPS"),
			TagName::Bpm => Cow::Borrowed("BPM"),
			TagName::Measure => Cow::Borrowed("MEASURE"),
			TagName::Tuning => Cow::Borrowed("TUNING"),
			TagName::ReplaygainGain => Cow::Borrowed("REPLAYGAIN_GAIN"),
			TagName::ReplaygainPeak => Cow::Borrowed("REPLAYGAIN_PEAK"),
			TagName::Isrc => Cow::Borrowed("ISRC"),
			TagName::Mcdi => Cow::Borrowed("MCDI"),
			TagName::Isbn => Cow::Borrowed("ISBN"),
			TagName::Barcode => Cow::Borrowed("BARCODE"),
			TagName::CatalogNumber => Cow::Borrowed("CATALOG_NUMBER"),
			TagName::LabelCode => Cow::Borrowed("LABEL_CODE"),
			TagName::Lccn => Cow::Borrowed("LCCN"),
			TagName::Imdb => Cow::Borrowed("IMDB"),
			TagName::Tmdb => Cow::Borrowed("TMDB"),
			TagName::Tvdb => Cow::Borrowed("TVDB"),
			TagName::Tvdb2 => Cow::Borrowed("TVDB2"),
			TagName::PurchaseItem => Cow::Borrowed("PURCHASE_ITEM"),
			TagName::PurchaseInfo => Cow::Borrowed("PURCHASE_INFO"),
			TagName::PurchaseOwner => Cow::Borrowed("PURCHASE_OWNER"),
			TagName::PurchasePrice => Cow::Borrowed("PURCHASE_PRICE"),
			TagName::PurchaseCurrency => Cow::Borrowed("PURCHASE_CURRENCY"),
			TagName::Copyright => Cow::Borrowed("COPYRIGHT"),
			TagName::ProductionCopyright => Cow::Borrowed("PRODUCTION_COPYRIGHT"),
			TagName::License => Cow::Borrowed("LICENSE"),
			TagName::TermsOfUse => Cow::Borrowed("TERMS_OF_USE"),
		}
	}
}