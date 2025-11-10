// Shorthand for `return Err(AudioError::Foo)`
//
// Usage:
// - err!(Variant)          -> return Err(AudioError::::Variant)
// - err!(Variant(Message)) -> return Err(AudioError::Variant(Message))
#[macro_export]
macro_rules! err {
	($variant:ident) => {
		return Err($crate::error::AudioError::$variant.into())
	};
	($variant:ident($reason:literal)) => {
		return Err($crate::error::AudioError::$variant($reason).into())
	};
}
