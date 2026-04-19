use std::collections::TryReserveError;

use crate::config::global_options;

/// A memory allocation failed
#[derive(Debug)]
pub enum AllocationError {
	/// The requested allocation exceeds the [`GlobalOptions::allocation_limit()`]
	///
	/// [`GlobalOptions::allocation_limit()`]: crate::config::GlobalOptions::allocation_limit
	LimitExceeded,
	/// Unable to reserve the requested size
	ReserveError(TryReserveError),
}

impl core::fmt::Display for AllocationError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			AllocationError::LimitExceeded => {
				write!(f, "attempted allocation would exceed global limits")
			},
			AllocationError::ReserveError(e) => write!(f, "{e}"),
		}
	}
}

impl core::error::Error for AllocationError {}

impl From<TryReserveError> for AllocationError {
	fn from(input: TryReserveError) -> Self {
		Self::ReserveError(input)
	}
}

/// Provides the `fallible_repeat` method on `Vec`
///
/// It is intended to be used in [`try_vec!`](crate::macros::try_vec).
trait VecFallibleRepeat<T>: Sized {
	fn fallible_repeat(self, element: T, expected_size: usize) -> Result<Self, AllocationError>
	where
		T: Clone;
}

impl<T> VecFallibleRepeat<T> for Vec<T> {
	fn fallible_repeat(mut self, element: T, expected_size: usize) -> Result<Self, AllocationError>
	where
		T: Clone,
	{
		if expected_size == 0 {
			return Ok(self);
		}

		if expected_size > unsafe { global_options().allocation_limit } {
			return Err(AllocationError::LimitExceeded);
		}

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
pub(crate) fn fallible_vec_from_element<T>(
	element: T,
	expected_size: usize,
) -> Result<Vec<T>, AllocationError>
where
	T: Clone,
{
	Vec::new().fallible_repeat(element, expected_size)
}

/// Provides the `try_with_capacity` method on `Vec`
///
/// This can be used directly.
pub(crate) trait VecFallibleCapacity<T>: Sized {
	/// Same as `Vec::with_capacity`, but takes `GlobalOptions::allocation_limit` into account.
	///
	/// Named `try_with_capacity_stable` to avoid conflicts with the nightly `Vec::try_with_capacity`.
	fn try_with_capacity_stable(capacity: usize) -> Result<Self, AllocationError>;
}

impl<T> VecFallibleCapacity<T> for Vec<T> {
	fn try_with_capacity_stable(capacity: usize) -> Result<Self, AllocationError> {
		if capacity > unsafe { global_options().allocation_limit } {
			return Err(AllocationError::LimitExceeded);
		}

		let mut v = Vec::new();
		v.try_reserve(capacity)?;

		Ok(v)
	}
}

#[cfg(test)]
mod tests {
	use crate::util::alloc::fallible_vec_from_element;

	#[test_log::test]
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
