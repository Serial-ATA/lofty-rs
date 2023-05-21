//! MusePack constants

// There are only 4 frequencies defined in the spec, but there are 8 possible indices in the header.
//
// The reference decoder defines the table as:
//
//    static const mpc_int32_t samplefreqs[8] = { 44100, 48000, 37800, 32000 };
//
// So it's safe to just fill the rest with zeroes
pub(super) const FREQUENCY_TABLE: [u32; 8] = [44100, 48000, 37800, 32000, 0, 0, 0, 0];

// Taken from mpcdec
/// This is the gain reference used in old ReplayGain
pub const MPC_OLD_GAIN_REF: f32 = 64.82;
