#[allow(missing_docs)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[rustfmt::skip]
#[non_exhaustive]
pub enum AudioObjectType {
	// https://en.wikipedia.org/wiki/MPEG-4_Part_3#MPEG-4_Audio_Object_Types

	#[default]
	NULL = 0,
	AacMain = 1,                                       // AAC Main Profile
	AacLowComplexity = 2,                              // AAC Low Complexity
	AacScalableSampleRate = 3,                         // AAC Scalable Sample Rate
	AacLongTermPrediction = 4,                         // AAC Long Term Predictor
	SpectralBandReplication = 5,                       // Spectral band Replication
	AACScalable = 6,                                   // AAC Scalable
	TwinVQ = 7,                                        // Twin VQ
	CodeExcitedLinearPrediction = 8,                   // CELP
	HarmonicVectorExcitationCoding = 9,                // HVXC
	TextToSpeechtInterface = 12,                       // TTSI
	MainSynthetic = 13,                                // Main Synthetic
	WavetableSynthesis = 14,                           // Wavetable Synthesis
	GeneralMIDI = 15,                                  // General MIDI
	AlgorithmicSynthesis = 16,                         // Algorithmic Synthesis
	ErrorResilientAacLowComplexity = 17,               // ER AAC LC
	ErrorResilientAacLongTermPrediction = 19,          // ER AAC LTP
	ErrorResilientAacScalable = 20,                    // ER AAC Scalable
	ErrorResilientAacTwinVQ = 21,                      // ER AAC TwinVQ
	ErrorResilientAacBitSlicedArithmeticCoding = 22,   // ER Bit Sliced Arithmetic Coding
	ErrorResilientAacLowDelay = 23,                    // ER AAC Low Delay
	ErrorResilientCodeExcitedLinearPrediction = 24,    // ER CELP
	ErrorResilientHarmonicVectorExcitationCoding = 25, // ER HVXC
	ErrorResilientHarmonicIndividualLinesNoise = 26,   // ER HILN
	ErrorResilientParametric = 27,                     // ER Parametric
	SinuSoidalCoding = 28,                             // SSC
	ParametricStereo = 29,                             // PS
	MpegSurround = 30,                                 // MPEG Surround
	MpegLayer1 = 32,                                   // MPEG Layer 1
	MpegLayer2 = 33,                                   // MPEG Layer 2
	MpegLayer3 = 34,                                   // MPEG Layer 3
	DirectStreamTransfer = 35,                         // DST Direct Stream Transfer
	AudioLosslessCoding = 36,                          // ALS Audio Lossless Coding
	ScalableLosslessCoding = 37,                       // SLC Scalable Lossless Coding
	ScalableLosslessCodingNoneCore = 38,               // SLC non-core
	ErrorResilientAacEnhancedLowDelay = 39,            // ER AAC ELD
	SymbolicMusicRepresentationSimple = 40,            // SMR Simple
	SymbolicMusicRepresentationMain = 41,              // SMR Main
	UnifiedSpeechAudioCoding = 42,                     // USAC
	SpatialAudioObjectCoding = 43,                     // SAOC
	LowDelayMpegSurround = 44,                         // LD MPEG Surround
	SpatialAudioObjectCodingDialogueEnhancement = 45,  // SAOC-DE
	AudioSync = 46,                                    // Audio Sync
}

impl TryFrom<u8> for AudioObjectType {
	type Error = ();

	#[rustfmt::skip]
	fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
		match value {
			1  => Ok(Self::AacMain),
			2  => Ok(Self::AacLowComplexity),
			3  => Ok(Self::AacScalableSampleRate),
			4  => Ok(Self::AacLongTermPrediction),
			5  => Ok(Self::SpectralBandReplication),
			6  => Ok(Self::AACScalable),
			7  => Ok(Self::TwinVQ),
			8  => Ok(Self::CodeExcitedLinearPrediction),
			9  => Ok(Self::HarmonicVectorExcitationCoding),
			12 => Ok(Self::TextToSpeechtInterface),
			13 => Ok(Self::MainSynthetic),
			14 => Ok(Self::WavetableSynthesis),
			15 => Ok(Self::GeneralMIDI),
			16 => Ok(Self::AlgorithmicSynthesis),
			17 => Ok(Self::ErrorResilientAacLowComplexity),
			19 => Ok(Self::ErrorResilientAacLongTermPrediction),
			20 => Ok(Self::ErrorResilientAacScalable),
			21 => Ok(Self::ErrorResilientAacTwinVQ),
			22 => Ok(Self::ErrorResilientAacBitSlicedArithmeticCoding),
			23 => Ok(Self::ErrorResilientAacLowDelay),
			24 => Ok(Self::ErrorResilientCodeExcitedLinearPrediction),
			25 => Ok(Self::ErrorResilientHarmonicVectorExcitationCoding),
			26 => Ok(Self::ErrorResilientHarmonicIndividualLinesNoise),
			27 => Ok(Self::ErrorResilientParametric),
			28 => Ok(Self::SinuSoidalCoding),
			29 => Ok(Self::ParametricStereo),
			30 => Ok(Self::MpegSurround),
			32 => Ok(Self::MpegLayer1),
			33 => Ok(Self::MpegLayer2),
			34 => Ok(Self::MpegLayer3),
			35 => Ok(Self::DirectStreamTransfer),
			36 => Ok(Self::AudioLosslessCoding),
			37 => Ok(Self::ScalableLosslessCoding),
			38 => Ok(Self::ScalableLosslessCodingNoneCore),
			39 => Ok(Self::ErrorResilientAacEnhancedLowDelay),
			40 => Ok(Self::SymbolicMusicRepresentationSimple),
			41 => Ok(Self::SymbolicMusicRepresentationMain),
			42 => Ok(Self::UnifiedSpeechAudioCoding),
			43 => Ok(Self::SpatialAudioObjectCoding),
			44 => Ok(Self::LowDelayMpegSurround),
			45 => Ok(Self::SpatialAudioObjectCodingDialogueEnhancement),
			46 => Ok(Self::AudioSync),
			_ => Err(()),
		}
	}
}
