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
