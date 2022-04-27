//! Tools to create custom file resolvers

use crate::error::Result;
use crate::file::TaggedFile;

use std::ffi::OsStr;
use std::io::{Read, Seek};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

use once_cell::sync::Lazy;

// Adapted from: https://github.com/rust-lang/rust/blob/master/compiler/rustc_data_structures/src/atomic_ref.rs
// This is essentially an `AtomicPtr` but is guaranteed to always be valid
struct AtomicRef<T: 'static>(AtomicPtr<T>, PhantomData<&'static T>);

#[allow(trivial_casts)]
impl<T: 'static> AtomicRef<T> {
	const fn new(initial: &'static T) -> AtomicRef<T> {
		AtomicRef(AtomicPtr::new(initial as *const _ as *mut T), PhantomData)
	}
}

impl<T: 'static> std::ops::Deref for AtomicRef<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		// SAFETY: We never allow storing anything but a `'static` reference so
		// it's safe to lend it out for any amount of time.
		unsafe { &*self.0.load(Ordering::SeqCst) }
	}
}

/// A `Read + Seek` supertrait for use in [`ResolverFn`]s
pub trait SeekRead: Read + Seek {}
impl<T: Seek + Read> SeekRead for T {}

/// A resolver function
///
/// This function, provided a path and reader, attempts to return a `TaggedFile`
///
/// NOTE: The path will **only** be `Some` if used with `read_from_path`
pub type ResolverFn = fn(Option<&OsStr>, &mut dyn SeekRead) -> Result<TaggedFile>;
type ResolverCollection = Vec<(&'static str, AtomicRef<ResolverFn>)>;

static CUSTOM_RESOLVERS: Lazy<Mutex<ResolverCollection>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Register a custom file resolver
///
/// Provided a resolver function, and a name to associate it with, this will attempt
/// to load them into the resolver collection.
///
/// Both the resolver and name *must* be static.
///
/// # Panics
///
/// * Attempting to register an existing name (See [`remove_custom_resolver`])
/// * See [`Mutex::lock`]
pub fn register_custom_resolver(name: &'static str, func: &'static ResolverFn) {
	let mut res = CUSTOM_RESOLVERS.lock().unwrap();

	assert!(res.iter().all(|(n, _)| *n != name));
	res.push((name, AtomicRef::new(func)));
}

/// Remove a registered file resolver
///
/// # Panics
///
/// See [`Mutex::lock`]
pub fn remove_custom_resolver(name: &'static str) {
	let mut res = CUSTOM_RESOLVERS.lock().unwrap();

	res.iter()
		.position(|(n, _)| *n == name)
		.map(|pos| res.remove(pos));
}
