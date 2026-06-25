macro_rules! try_vec {
	($elem:expr; $size:expr) => {{ $crate::util::alloc::fallible_vec_from_element($elem, $size) }};
}

// A macro for handling the different `ParsingMode`s
//
// NOTE: All fields are optional, if `STRICT` or `RELAXED` are missing, it will
// 		 fall through to `DEFAULT`. If `DEFAULT` is missing, it will fall through
// 		 to an empty block.
//
// Usage:
//
// - parse_mode_choice!(
// 		ident_of_parsing_mode,
// 		STRICT: some_expr,
// 		RELAXED: some_expr,
// 		DEFAULT: some_expr,
// 	 )
macro_rules! parse_mode_choice {
	(
		$parse_mode:ident,
		$(STRICT: $strict_handler:expr,)?
		$(BESTATTEMPT: $best_attempt_handler:expr,)?
		$(RELAXED: $relaxed_handler:expr,)?
		DEFAULT: $default:expr
	) => {
		match $parse_mode {
			$(crate::config::ParsingMode::Strict => { $strict_handler },)?
			$(crate::config::ParsingMode::BestAttempt => { $best_attempt_handler },)?
			$(crate::config::ParsingMode::Relaxed => { $relaxed_handler },)?
			_ => { $default }
		}
	};
	(
		$parse_mode:ident,
		$(STRICT: $strict_handler:expr,)?
		$(BESTATTEMPT: $best_attempt_handler:expr,)?
		$(RELAXED: $relaxed_handler:expr $(,)?)?
	) => {
		match $parse_mode {
			$(crate::config::ParsingMode::Strict => { $strict_handler },)?
			$(crate::config::ParsingMode::BestAttempt => { $best_attempt_handler },)?
			$(crate::config::ParsingMode::Relaxed => { $relaxed_handler },)?
			#[allow(unreachable_patterns)]
			_ => {}
		}
	};
}

pub(crate) use parse_mode_choice;
pub(crate) use try_vec;
