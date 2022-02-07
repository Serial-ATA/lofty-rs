macro_rules! tag_methods {
	(
		$(
			$(#[cfg($meta:meta)])?
			$name:ident,
			$ty:ty
		);*
	) => {
		paste::paste! {
			$(
				$(#[cfg($meta)])?
				#[doc = "Gets the [`" $ty "`] if it exists"]
				pub fn $name(&self) -> Option<&$ty> {
					self.$name.as_ref()
				}

				$(#[cfg($meta)])?
				#[doc = "Gets a mutable reference to the [`" $ty "`] if it exists"]
				pub fn [<$name _mut>](&mut self) -> Option<&mut $ty> {
					self.$name.as_mut()
				}

				$(#[cfg($meta)])?
				#[doc = "Removes the [`" $ty "`]"]
				pub fn [<remove_ $name>](&mut self) {
					self.$name = None
				}
			)*
		}
	}
}

macro_rules! feature_locked {
	(
		#![cfg($meta:meta)]
		$($item:item)+
	) => {
		$(
			#[cfg($meta)]
			$item
		)+
	}
}

macro_rules! try_vec {
	($elem:expr; $size:expr) => {{
		let mut v = Vec::new();
		v.try_reserve($size)?;
		v.resize($size, $elem);

		v
	}};
}

pub(crate) use {feature_locked, tag_methods, try_vec};
