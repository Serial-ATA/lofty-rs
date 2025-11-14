pub(crate) trait ShrinkableInteger {
	fn shrink_le(self) -> impl Iterator<Item = u8>;
	fn shrink_be(self) -> impl Iterator<Item = u8>;
	fn occupied_bytes(self) -> u8;
}

macro_rules! impl_shrinkable_integer {
	($($ty:ty),*) => {
		$(
			impl ShrinkableInteger for $ty {
				fn shrink_le(self) -> impl Iterator<Item = u8> {
					let occupied_bytes = self.occupied_bytes() as usize;
					self.to_le_bytes().into_iter().take(size_of::<$ty>() - occupied_bytes)
				}

				fn shrink_be(self) -> impl Iterator<Item = u8> {
					let occupied_bytes = self.occupied_bytes() as usize;
					self.to_be_bytes().into_iter().skip(size_of::<$ty>() - occupied_bytes)
				}

				fn occupied_bytes(self) -> u8 {
					if self == 0 {
						return 1;
					}

					let ret = size_of::<$ty>() - (self.to_le().leading_zeros() >> 3) as usize;
					ret as u8
				}
			}
		)*
	}
}

impl_shrinkable_integer!(u16, u32, u64);

#[cfg(test)]
mod tests {
	use super::ShrinkableInteger;

	macro_rules! int_test {
		(
			$(
				{
					input: $input:expr,
					expected: $expected:expr $(,)?
				}
			),+ $(,)?
		) => {
			$(
				{
					let bytes = $input.occupied_bytes() as usize;
					assert_eq!(&$input.to_be_bytes()[4 - bytes..], &$expected[..]);
				}
			)+
		}
	}

	#[test_log::test]
	fn integer_shrinking_unsigned() {
		int_test! {
			{
				input: 0u32,
				expected: [0],
			},
			{
				input: 1u32,
				expected: [1],
			},
			{
				input: 32767u32,
				expected: [127, 255],
			},
			{
				input: 65535u32,
				expected: [255, 255],
			},
			{
				input: 8_388_607_u32,
				expected: [127, 255, 255],
			},
			{
				input: 16_777_215_u32,
				expected: [255, 255, 255],
			},
			{
				input: 33_554_431_u32,
				expected: [1, 255, 255, 255],
			},
			{
				input: u32::MAX,
				expected: [255, 255, 255, 255],
			},
		}
	}
}
