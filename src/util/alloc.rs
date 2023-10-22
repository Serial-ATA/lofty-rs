use crate::error::Result;
use crate::macros::err;
use crate::probe::ParseOptions;

use std::cell::UnsafeCell;

thread_local! {
	static ALLOCATION_LIMIT: UnsafeCell<usize> = const { UnsafeCell::new(ParseOptions::DEFAULT_ALLOCATION_LIMIT) };
}

pub(crate) unsafe fn update_allocation_limit(limit: usize) {
	ALLOCATION_LIMIT.with(|l| *l.get() = limit);
}

/// Provides the `fallible_repeat` method on `Vec`
///
/// It is intended to be used in [`try_vec!`](crate::macros::try_vec).
trait VecFallibleRepeat<T>: Sized {
	fn fallible_repeat(self, element: T, expected_size: usize) -> Result<Self>
	where
		T: Clone;
}

impl<T> VecFallibleRepeat<T> for Vec<T> {
	fn fallible_repeat(mut self, element: T, expected_size: usize) -> Result<Self>
	where
		T: Clone,
	{
		if expected_size == 0 {
			return Ok(self);
		}

		ALLOCATION_LIMIT.with(|limit| {
			if expected_size > unsafe { *limit.get() } {
				err!(TooMuchData);
			}
			Ok(())
		})?;

		self.try_reserve(expected_size)?;

		let ptr = self.as_mut_ptr();
		let mut current_length = self.len();
		while current_length != expected_size {
			unsafe {
				ptr.add(current_length).write(element.clone());
			}
			current_length += 1;
		}

		unsafe {
			self.set_len(current_length);
		}

		Ok(self)
	}
}

/// **DO NOT USE DIRECTLY**
///
/// Creates a `Vec` of the specified length, containing copies of `element`.
///
/// This should be used through [`try_vec!`](crate::macros::try_vec)
pub(crate) fn fallible_vec_from_element<T>(element: T, expected_size: usize) -> Result<Vec<T>>
where
	T: Clone,
{
	Vec::new().fallible_repeat(element, expected_size)
}

#[cfg(test)]
mod tests {
	use crate::util::alloc::fallible_vec_from_element;

	#[test]
	fn vec_fallible_repeat() {
		let u8_vec_len_20 = fallible_vec_from_element(0u8, 20).unwrap();
		assert_eq!(u8_vec_len_20.len(), 20);
		assert!(u8_vec_len_20.iter().all(|e| *e == 0));

		let u64_vec_len_89 = fallible_vec_from_element(0u64, 89).unwrap();
		assert_eq!(u64_vec_len_89.len(), 89);
		assert!(u64_vec_len_89.iter().all(|e| *e == 0));

		let u8_large_vec = fallible_vec_from_element(0u8, u32::MAX as usize);
		assert!(u8_large_vec.is_err());
	}
}
