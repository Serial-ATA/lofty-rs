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

unsigned_rounded_division!(u8, u16, u32, u64, usize);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
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
}
