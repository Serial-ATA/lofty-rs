/// Perform a rounded division.
///
/// This is implemented for all unsigned integers.
///
/// NOTE: If the result is less than 1, it will be rounded up to 1.
pub(crate) trait RoundedDivision<Rhs = Self> {
	type Output;

	fn div_round(self, rhs: Rhs) -> Self::Output;
}

macro_rules! unsigned_rounded_division {
	($($t:ty),*) => {
		$(
			impl RoundedDivision for $t {
				type Output = $t;

				fn div_round(self, rhs: Self) -> Self::Output {
					(self + (rhs >> 1)) / rhs
				}
			}
		)*
	};
}

unsigned_rounded_division!(u8, u16, u32, u64, u128, usize);

/// An 80-bit extended precision floating-point number.
///
/// This is used in AIFF.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub(crate) struct F80 {
	signed: bool,
	// 15-bit exponent with a bias of 16383
	exponent: u16,
	fraction: u64,
}

impl F80 {
	/// Create a new `F80` from big-endian bytes.
	///
	/// See [here](https://en.wikipedia.org/wiki/Extended_precision#/media/File:X86_Extended_Floating_Point_Format.svg) for a diagram of the format.
	pub fn from_be_bytes(bytes: [u8; 10]) -> Self {
		let signed = bytes[0] & 0x80 != 0;
		let exponent = (u16::from(bytes[0] & 0x7F) << 8) | u16::from(bytes[1]);

		let mut fraction_bytes = [0; 8];
		fraction_bytes.copy_from_slice(&bytes[2..]);
		let fraction = u64::from_be_bytes(fraction_bytes);

		Self {
			signed,
			exponent,
			fraction,
		}
	}

	/// Convert the `F80` to an `f64`.
	pub fn as_f64(&self) -> f64 {
		// Apple® Apple Numerics Manual, Second Edition, Table 2-7:
		//
		// Biased exponent e  Integer i  Fraction f          Value v                     Class of v
		// 0 <= e <= 32766       1         (any)     v = (-1)^s * 2^(e-16383) * (1.f)   Normalized
		// 0 <= e <= 32766       0         f != 0    v = (-1)^s * 2^(e-16383) * (0.f)   Denormalized
		// 0 <= e <= 32766       0         f = 0     v = (-1)^s * 0                     Zero
		//    e = 32767        (any)       f = 0     v = (-1)^s * Infinity              Infinity
		//    e = 32767        (any)       f != 0    v is a NaN                         NaN

		let sign = if self.signed { 1 } else { 0 };

		// e = 32767
		if self.exponent == 32767 {
			if self.fraction == 0 {
				return f64::from_bits((sign << 63) | f64::INFINITY.to_bits());
			}

			return f64::from_bits((sign << 63) | f64::NAN.to_bits());
		}

		// 0 <= e <= 32766, i = 0, f = 0
		if self.fraction == 0 {
			return f64::from_bits(sign << 63);
		}

		// 0 <= e <= 32766, 0 <= i <= 1, f >= 0

		let fraction = self.fraction & 0x7FFF_FFFF_FFFF_FFFF;
		let exponent = self.exponent as i16 - 16383 + 1023;
		let bits = (sign << 63) | ((exponent as u64) << 52) | (fraction >> 11);

		f64::from_bits(bits)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test_log::test]
	fn test_div_round() {
		#[derive(Debug)]
		struct TestEntry {
			lhs: u32,
			rhs: u32,
			result: u32,
		}

		#[rustfmt::skip]
		let tests = [
			TestEntry { lhs: 1, rhs: 1, result: 1 },
			TestEntry { lhs: 1, rhs: 2, result: 1 },
			TestEntry { lhs: 2, rhs: 2, result: 1 },
			TestEntry { lhs: 3, rhs: 2, result: 2 },
			TestEntry { lhs: 4, rhs: 2, result: 2 },
			TestEntry { lhs: 5, rhs: 2, result: 3 },

			// Should be rounded up to 1
			TestEntry { lhs: 800, rhs: 1500, result: 1 },
			TestEntry { lhs: 1500, rhs: 3000, result: 1 },

			// Shouldn't be rounded
			TestEntry { lhs: 0, rhs: 4000, result: 0 },
			TestEntry { lhs: 1500, rhs: 4000, result: 0 },
		];

		for test in &tests {
			let result = test.lhs.div_round(test.rhs);
			assert_eq!(result, test.result, "{}.div_round({})", test.lhs, test.rhs);
		}
	}

	#[test_log::test]
	fn test_f80() {
		fn cmp_float_nearly_equal(a: f64, b: f64) -> bool {
			if a.is_infinite() && b.is_infinite() {
				return true;
			}

			if a.is_nan() && b.is_nan() {
				return true;
			}

			(a - b).abs() < f64::EPSILON
		}

		#[derive(Debug)]
		struct TestEntry {
			input: [u8; 10],
			output_f64: f64,
		}

		let tests = [
			TestEntry {
				input: [0; 10],
				output_f64: 0.0,
			},
			TestEntry {
				input: [0x7F, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0],
				output_f64: f64::INFINITY,
			},
			TestEntry {
				input: [0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0],
				output_f64: f64::NEG_INFINITY,
			},
			TestEntry {
				input: [0x7F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0],
				output_f64: f64::NAN,
			},
			TestEntry {
				input: [0xFF, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0],
				output_f64: -f64::NAN,
			},
			TestEntry {
				input: [0x3F, 0xFC, 0x80, 0, 0, 0, 0, 0, 0, 0],
				output_f64: 0.125,
			},
			TestEntry {
				input: [0x3F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0],
				output_f64: 1.0,
			},
			TestEntry {
				input: [0x40, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0],
				output_f64: 2.0,
			},
			TestEntry {
				input: [0x40, 0x00, 0xC0, 0, 0, 0, 0, 0, 0, 0],
				output_f64: 3.0,
			},
			TestEntry {
				input: [0x40, 0x0E, 0xBB, 0x80, 0, 0, 0, 0, 0, 0],
				output_f64: 48000.0,
			},
		];

		for test in &tests {
			let f80 = F80::from_be_bytes(test.input);
			let f64 = f80.as_f64();
			assert!(
				cmp_float_nearly_equal(f64, test.output_f64),
				"F80::as_f64({f80:?}) == {f64} (expected {})",
				test.output_f64
			);
		}
	}
}
