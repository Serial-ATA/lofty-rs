use std::ops::RangeBounds;
use std::vec::ExtractIf;

/// Wrapper for [`ExtractIf`] to ensure that all filtered items are removed on drop
pub(crate) struct ExhaustiveExtractIf<'a, T, F>
where
	F: FnMut(&mut T) -> bool,
{
	iter: Option<ExtractIf<'a, T, F>>,
}

impl<'a, T, F> From<ExtractIf<'a, T, F>> for ExhaustiveExtractIf<'a, T, F>
where
	F: FnMut(&mut T) -> bool,
{
	fn from(input: ExtractIf<'a, T, F>) -> Self {
		Self { iter: Some(input) }
	}
}

impl<T, F> Iterator for ExhaustiveExtractIf<'_, T, F>
where
	F: FnMut(&mut T) -> bool,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.as_mut().and_then(Iterator::next)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.as_ref().map_or((0, None), Iterator::size_hint)
	}
}

impl<T, F> Drop for ExhaustiveExtractIf<'_, T, F>
where
	F: FnMut(&mut T) -> bool,
{
	fn drop(&mut self) {
		if let Some(iter) = self.iter.take() {
			iter.for_each(drop);
		}
	}
}

pub(crate) trait VecExtractIfExt<T> {
	/// Same as [`Vec::extract_if()`], except the returned iterator drops any remaining items
	fn extract_if_exhaustive<R, F>(&mut self, range: R, filter: F) -> ExhaustiveExtractIf<'_, T, F>
	where
		F: FnMut(&mut T) -> bool,
		R: RangeBounds<usize>;
}

impl<T> VecExtractIfExt<T> for Vec<T> {
	fn extract_if_exhaustive<R, F>(&mut self, range: R, filter: F) -> ExhaustiveExtractIf<'_, T, F>
	where
		F: FnMut(&mut T) -> bool,
		R: RangeBounds<usize>,
	{
		self.extract_if(range, filter).into()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test_log::test]
	fn extract_if_exhaustive() {
		let mut v = vec![1, 2, 3, 4];

		let mut iter = v.extract_if_exhaustive(.., |item| *item % 2 == 0);
		assert_eq!(iter.next(), Some(2));
		drop(iter);

		assert_eq!(v, &[1, 3]);
	}
}
