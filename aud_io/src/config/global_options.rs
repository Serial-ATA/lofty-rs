use std::cell::UnsafeCell;

thread_local! {
	static GLOBAL_OPTIONS: UnsafeCell<GlobalOptions> = UnsafeCell::new(GlobalOptions::default());
}

pub(crate) unsafe fn global_options() -> &'static GlobalOptions {
	GLOBAL_OPTIONS.with(|global_options| unsafe { &*global_options.get() })
}

/// Options that control all interactions with `aud_io` for the current thread
///
/// # Examples
///
/// ```rust
/// use aud_io::config::{GlobalOptions, apply_global_options};
///
/// // I want to double the allocation limit
/// let global_options =
/// 	GlobalOptions::new().allocation_limit(GlobalOptions::DEFAULT_ALLOCATION_LIMIT * 2);
/// apply_global_options(global_options);
/// ```
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[non_exhaustive]
pub struct GlobalOptions {
	pub(crate) allocation_limit: usize,
}

impl GlobalOptions {
	/// Default allocation limit for any single allocation
	pub const DEFAULT_ALLOCATION_LIMIT: usize = 16 * 1024 * 1024;

	/// Creates a new `GlobalOptions`, alias for `Default` implementation
	///
	/// See also: [`GlobalOptions::default`]
	///
	/// # Examples
	///
	/// ```rust
	/// use aud_io::config::GlobalOptions;
	///
	/// let global_options = GlobalOptions::new();
	/// ```
	#[must_use]
	pub const fn new() -> Self {
		Self {
			allocation_limit: Self::DEFAULT_ALLOCATION_LIMIT,
		}
	}

	/// The maximum number of bytes to allocate for any single allocation
	///
	/// This is a safety measure to prevent allocating too much memory for a single allocation. If an allocation
	/// exceeds this limit, the allocator will return [`AudioError::TooMuchData`](crate::error::AudioError::TooMuchData).
	///
	/// # Examples
	///
	/// ```rust
	/// use aud_io::config::{GlobalOptions, apply_global_options};
	///
	/// // I have files with gigantic images, I'll double the allocation limit!
	/// let global_options =
	/// 	GlobalOptions::new().allocation_limit(GlobalOptions::DEFAULT_ALLOCATION_LIMIT * 2);
	/// apply_global_options(global_options);
	/// ```
	pub fn allocation_limit(&mut self, allocation_limit: usize) -> Self {
		self.allocation_limit = allocation_limit;
		*self
	}
}

impl Default for GlobalOptions {
	/// The default implementation for `GlobalOptions`
	///
	/// The defaults are as follows:
	///
	/// ```rust,ignore
	/// GlobalOptions {
	/// 	allocation_limit: Self::DEFAULT_ALLOCATION_LIMIT,
	/// }
	/// ```
	fn default() -> Self {
		Self::new()
	}
}

/// Applies the given `GlobalOptions` to the current thread
///
/// # Examples
///
/// ```rust
/// use aud_io::config::{GlobalOptions, apply_global_options};
///
/// // I want to double the allocation limit
/// let global_options =
/// 	GlobalOptions::new().allocation_limit(GlobalOptions::DEFAULT_ALLOCATION_LIMIT * 2);
/// apply_global_options(global_options);
/// ```
pub fn apply_global_options(options: GlobalOptions) {
	GLOBAL_OPTIONS.with(|global_options| unsafe {
		*global_options.get() = options;
	});
}
