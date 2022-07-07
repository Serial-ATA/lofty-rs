// See cfg-if comment in `Cargo.toml`
//
// macro_rules! feature_locked {
// 	(
// 		#![cfg($meta:meta)]
// 		$($item:item)+
// 	) => {
// 		$(
// 			#[cfg($meta)]
// 			$item
// 		)+
// 	}
// }

macro_rules! try_vec {
	($elem:expr; $size:expr) => {{
		let mut v = Vec::new();
		v.try_reserve_exact($size)?;
		v.resize($size, $elem);

		v
	}};
}

// Shorthand for return Err(LoftyError::new(ErrorKind::Foo))
//
// Usage:
// - err!(Variant)          -> return Err(LoftyError::new(ErrorKind::Variant))
// - err!(Variant(Message)) -> return Err(LoftyError::new(ErrorKind::Variant(Message)))
macro_rules! err {
	($variant:ident) => {
		return Err(crate::error::LoftyError::new(
			crate::error::ErrorKind::$variant,
		))
	};
	($variant:ident($reason:literal)) => {
		return Err(crate::error::LoftyError::new(
			crate::error::ErrorKind::$variant($reason),
		))
	};
}

pub(crate) use {err, try_vec};
