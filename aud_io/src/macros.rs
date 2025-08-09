#[macro_export]
macro_rules! try_vec {
	($elem:expr; $size:expr) => {{ $crate::alloc::fallible_vec_from_element($elem, $size)? }};
}

// Shorthand for return Err(AudioError::Variant)
//
// Usage:
// - err!(Variant)          -> return Err(AudioError::Variant)
// - err!(Variant(Message)) -> return Err(AudioError:(Message))
#[macro_export]
macro_rules! err {
	($variant:ident) => {
		return Err($crate::error::AudioError::$variant.into())
	};
	($variant:ident($reason:literal)) => {
		return Err($crate::error::AudioError::$variant($reason).into())
	};
}
