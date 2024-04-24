//! Utilities for upgrading old ID3v2 frame IDs

use std::collections::HashMap;

/// Upgrade an ID3v2.2 key to an ID3v2.4 key
///
/// # Examples
///
/// ```rust
/// use lofty::id3::v2::upgrade_v2;
///
/// let old_title = "TT2";
/// let new_title = upgrade_v2(old_title);
///
/// assert_eq!(new_title, Some("TIT2"));
/// ```
pub fn upgrade_v2(key: &str) -> Option<&'static str> {
	v2keys().get(key).copied()
}

/// Upgrade an ID3v2.3 key to an ID3v2.4 key
///
/// # Examples
///
/// ```rust
/// use lofty::id3::v2::upgrade_v3;
///
/// let old_involved_people_list = "IPLS";
/// let new_involved_people_list = upgrade_v3(old_involved_people_list);
///
/// assert_eq!(new_involved_people_list, Some("TIPL"));
/// ```
pub fn upgrade_v3(key: &str) -> Option<&'static str> {
	v3keys().get(key).copied()
}

macro_rules! gen_upgrades {
    (V2 => [$($($v2_key:literal)|* => $id3v24_from_v2:literal),+]; V3 => [$($($v3_key:literal)|* => $id3v24_from_v3:literal),+]) => {
		use std::sync::OnceLock;

		fn v2keys() -> &'static HashMap<&'static str, &'static str> {
			static INSTANCE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
			INSTANCE.get_or_init(|| {
				let mut map = HashMap::new();
				$(
					$(
						map.insert($v2_key, $id3v24_from_v2);
					)+
				)+
				map
			})
		}

		fn v3keys() -> &'static HashMap<&'static str, &'static str> {
			static INSTANCE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
			INSTANCE.get_or_init(|| {
				let mut map = HashMap::new();
				$(
					$(
						map.insert($v3_key, $id3v24_from_v3);
					)+
				)+
				map
			})
		}
	};
}

gen_upgrades!(
	// ID3v2.2 => ID3v2.4
	V2 => [
		// Standard frames
		"BUF" => "RBUF",
		"CNT" => "PCNT",
		"COM" => "COMM",
		"CRA" => "AENC",
		"ETC" => "ETCO",
		"GEO" => "GEOB",
		"IPL" => "TIPL",
		"MCI" => "MCDI",
		"MLL" => "MLLT",
		"PIC" => "APIC",
		"POP" => "POPM",
		"REV" => "RVRB",
		"SLT" => "SYLT",
		"STC" => "SYTC",
		"TAL" => "TALB",
		"TBP" => "TBPM",
		"TCM" => "TCOM",
		"TCO" => "TCON",
		"TCP" => "TCMP",
		"TCR" => "TCOP",
		"TDY" => "TDLY",
		"TEN" => "TENC",
		"TFT" => "TFLT",
		"TKE" => "TKEY",
		"TLA" => "TLAN",
		"TLE" => "TLEN",
		"TMT" => "TMED",
		"TOA" => "TOAL",
		"TOF" => "TOFN",
		"TOL" => "TOLY",
		"TOR" => "TDOR",
		"TOT" => "TOAL",
		"TP1" => "TPE1",
		"TP2" => "TPE2",
		"TP3" => "TPE3",
		"TP4" => "TPE4",
		"TPA" => "TPOS",
		"TPB" => "TPUB",
		"TRC" => "TSRC",
		"TRD" => "TDRC",
		"TRK" => "TRCK",
		"TS2" => "TSO2",
		"TSA" => "TSOA",
		"TSC" => "TSOC",
		"TSP" => "TSOP",
		"TSS" => "TSSE",
		"TST" => "TSOT",
		"TT1" => "TIT1",
		"TT2" => "TIT2",
		"TT3" => "TIT3",
		"TXT" => "TOLY",
		"TXX" => "TXXX",
		"TYE" => "TDRC",
		"UFI" => "UFID",
		"ULT" => "USLT",
		"WAF" => "WOAF",
		"WAR" => "WOAR",
		"WAS" => "WOAS",
		"WCM" => "WCOM",
		"WCP" => "WCOP",
		"WPB" => "WPUB",
		"WXX" => "WXXX",

		// iTunes non-standard frames

		// Podcast
		"PCS" => "PCST",
		"TCT" => "TCAT",
		"TDS" => "TDES",
		"TID" => "TGID",
		"WFD" => "WFED",

		// Identifiers
		"MVI" => "MVIN",
		"MVN" => "MVNM",
		"GP1" => "GRP1",
		"TDR" => "TDRL"
	];
	// ID3v2.3 => ID3v2.4
	V3 => [
		// Standard frames
		"TORY" => "TDOR",
		"TYER" => "TDRC",
		"IPLS" => "TIPL"
	]
);
