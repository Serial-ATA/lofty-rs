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

// Shorthand for FileDecodingError::new(FileType::Foo, "Message")
//
// Usage:
//
// - decode_err!(Variant, Message)
// - decode_err!(Message)
//
// or bail:
//
// - decode_err!(@BAIL Variant, Message)
// - decode_err!(@BAIL Message)
macro_rules! decode_err {
	($file_ty:ident, $reason:literal) => {
		Into::<crate::error::LoftyError>::into(crate::error::FileDecodingError::new(
			crate::file::FileType::$file_ty,
			$reason,
		))
	};
	($reason:literal) => {
		Into::<crate::error::LoftyError>::into(crate::error::FileDecodingError::from_description(
			$reason,
		))
	};
	(@BAIL $($file_ty:ident,)? $reason:literal) => {
		return Err(decode_err!($($file_ty,)? $reason))
	};
}

// TODO: document
macro_rules! parse_mode_choice {
	(
		$parse_mode:ident,
		$(STRICT: $strict_handler:expr,)?
		$(RELAXED: $relaxed_handler:expr,)?
		$(DEFAULT: $default:expr)?
	) => {
		match $parse_mode {
			$(crate::probe::ParsingMode::Strict => { $strict_handler },)?
			$(crate::probe::ParsingMode::Relaxed => { $relaxed_handler },)?
			_ => { $default }
		}
	};
	(
		$parse_mode:ident,
		$(STRICT: $strict_handler:expr,)?
		$(RELAXED: $relaxed_handler:expr $(,)?)?
	) => {
		match $parse_mode {
			$(crate::probe::ParsingMode::Strict => { $strict_handler },)?
			$(crate::probe::ParsingMode::Relaxed => { $relaxed_handler },)?
			#[allow(unreachable_patterns)]
			_ => { unreachable!() }
		}
	};
}

pub(crate) use {decode_err, err, parse_mode_choice, try_vec};
