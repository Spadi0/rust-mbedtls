/* Copyright (c) Fortanix, Inc.
 *
 * Licensed under the GNU General Public License, version 2 <LICENSE-GPL or
 * https://www.gnu.org/licenses/gpl-2.0.html> or the Apache License, Version
 * 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>, at your
 * option. This file may not be copied, modified, or distributed except
 * according to those terms. */

macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

macro_rules! callback {
	//{ ($($arg:ident: $ty:ty),*) -> $ret:ty } => {
	//};
	{ $n:ident$( : $sync:ident )*($($arg:ident: $ty:ty),*) -> $ret:ty } => {
		#[cfg(not(feature="threading"))]
		pub trait $n {
			unsafe extern "C" fn call(user_data: *mut ::mbedtls_sys::types::raw_types::c_void, $($arg:$ty),*) -> $ret;

			fn data_ptr(&mut self) -> *mut ::mbedtls_sys::types::raw_types::c_void;
		}

		#[cfg(feature="threading")]
		pub trait $n $( : $sync )* {
			unsafe extern "C" fn call(user_data: *mut ::mbedtls_sys::types::raw_types::c_void, $($arg:$ty),*) -> $ret;

			fn data_ptr(&mut self) -> *mut ::mbedtls_sys::types::raw_types::c_void;
		}

		#[cfg(not(feature="threading"))]
		impl<F> $n for F where F: FnMut($($ty),*) -> $ret {
			unsafe extern "C" fn call(user_data: *mut ::mbedtls_sys::types::raw_types::c_void, $($arg:$ty),*) -> $ret {
				(&mut*(user_data as *mut F))($($arg),*)
			}

			fn data_ptr(&mut self) -> *mut ::mbedtls_sys::types::raw_types::c_void {
				self as *mut F as *mut _
			}
		}

		#[cfg(feature="threading")]
		impl<F> $n for F where F: Sync + FnMut($($ty),*) -> $ret {
			unsafe extern "C" fn call(user_data: *mut ::mbedtls_sys::types::raw_types::c_void, $($arg:$ty),*) -> $ret {
				(&mut*(user_data as *mut F))($($arg),*)
			}

			fn data_ptr(&mut self) -> *mut ::mbedtls_sys::types::raw_types::c_void {
				self as *mut F as *mut _
			}
		}
	};
}

macro_rules! define {
	{ $(#[$m:meta])* struct $name:ident$(<$l:tt>)*($inner:ident) { $($defs:tt)* } } => {
		define_struct!(define $(#[$m])* struct $name $(lifetime $l)* inner $inner);
		define_struct!(<< $name $(lifetime $l)* inner $inner >> $($defs)*);
	};
	{ $(#[$m:meta])* enum $n:ident -> $raw:ty { $(#[$doc:meta] $rust:ident => $c:ident,)* } } => { define_enum!($(#[$m])* enum $n ty $raw : $(doc ($doc) rust $rust c $c),*); };
	{ $(#[$m:meta])* enum $n:ident -> $raw:ty { $(             $rust:ident => $c:ident,)* } } => { define_enum!($(#[$m])* enum $n ty $raw : $(doc (    ) rust $rust c $c),*); };
}

macro_rules! define_enum {
	{$(#[$m:meta])* enum $n:ident ty $raw:ty : $(doc ($($doc:meta)*) rust $rust:ident c $c:ident),*} => {
		$(#[$m])*
		pub enum $n {
			$($(#[$doc])* $rust,)*
		}

		impl Into<$raw> for $n {
			fn into(self) -> $raw {
				match self {
					$($n::$rust => ::mbedtls_sys::$c,)*
				}
			}
		}
	};
}

macro_rules! define_struct {
	{ define $(#[$m:meta])* struct $name:ident $(lifetime $l:tt)* inner $inner:ident } => {
		as_item!(
		#[allow(dead_code)]
		$(#[$m])*
		pub struct $name<$($l)*> {
			inner: ::mbedtls_sys::$inner,
			$(r: ::core::marker::PhantomData<&$l ()>,)*
		}
		);

		as_item!(
		#[cfg(feature="threading")]
		unsafe impl<$($l)*> Send for $name<$($l)*> {}
		);
	};

	{ << $name:ident $(lifetime $l:tt)* inner $inner:ident >> fn init = $ctor:ident; $($defs:tt)* } => {
		define_struct!(init $name () init $ctor $(lifetime $l)* );
		define_struct!(<< $name $(lifetime $l)* inner $inner >> $($defs)*);
	};
	{ << $name:ident $(lifetime $l:tt)* inner $inner:ident >> pub fn new = $ctor:ident; $($defs:tt)* } => {
		define_struct!(init $name (pub) new $ctor $(lifetime $l)* );
		define_struct!(<< $name $(lifetime $l)* inner $inner >> $($defs)*);
	};
	{ init $name:ident ($($vis:tt)*) $new:ident $ctor:ident $(lifetime $l:tt)* } => {
		as_item!(
		#[allow(dead_code)]
		impl<$($l)*> $name<$($l)*> {
			$($vis)* fn $new() -> Self {
				let mut inner;
				unsafe{
					inner=::core::mem::uninitialized();
					::mbedtls_sys::$ctor(&mut inner)
				};
				$name{
					inner:inner,
					$(r: ::core::marker::PhantomData::<&$l _>,)*
				}
			}
		}
		);
	};

	{ << $name:ident $(lifetime $l:tt)* inner $inner:ident >> fn drop = $dtor:ident; $($defs:tt)* } => {
		define_struct!(drop $name dtor $dtor $(lifetime $l)* );
		define_struct!(<< $name $(lifetime $l)* inner $inner >> $($defs)*);
	};
	{ drop $name:ident dtor $dtor:ident $(lifetime $l:tt)* } => {
		as_item!(
		impl<$($l)*> Drop for $name<$($l)*> {
			fn drop(&mut self) {
				unsafe{::mbedtls_sys::$dtor(&mut self.inner)};
			}
		}
		);
	};

	{ << $name:ident $(lifetime $l:tt)* inner $inner:ident >> impl<$l2:tt> Into<*>; $($defs:tt)* } => {
		define_struct!(into $name inner $inner $(lifetime $l)* lifetime2 $l2 );
		define_struct!(<< $name $(lifetime $l)* inner $inner >> $($defs)*);
	};
	{ into $name:ident inner $inner:ident $(lifetime $l:tt)* lifetime2 $l2:tt } => {
		as_item!(
		impl<$l2,$($l),*> Into<*const $inner> for &$l2 $name<$($l)*> {
			fn into(self) -> *const $inner {
				&self.inner
			}
		}
		);

		as_item!(
		impl<$l2,$($l),*> Into<*mut $inner> for &$l2 mut $name<$($l)*> {
			fn into(self) -> *mut $inner {
				&mut self.inner
			}
		}
		);
	};

	{ << $name:ident $(lifetime $l:tt)* inner $inner:ident >> impl<$l2:tt> UnsafeFrom<*>; $($defs:tt)* } => {
		define_struct!(unsafe_from $name inner $inner $(lifetime $l)* lifetime2 $l2 );
		define_struct!(<< $name $(lifetime $l)* inner $inner >> $($defs)*);
	};
	{ unsafe_from $name:ident inner $inner:ident $(lifetime $l:tt)* lifetime2 $l2:tt } => {
		as_item!(
		impl<$l2,$($l),*> ::private::UnsafeFrom<*const $inner> for &$l2 $name<$($l)*> {
			unsafe fn from(ptr: *const $inner) -> Option<Self> {
				(ptr as *const $name).as_ref()
			}
		}
		);

		as_item!(
		impl<$l2,$($l),*> ::private::UnsafeFrom<*mut $inner> for &$l2 mut $name<$($l)*> {
			unsafe fn from(ptr: *mut $inner) -> Option<Self> {
				(ptr as *mut $name).as_mut()
			}
		}
		);
	};

	{ << $name:ident $(lifetime $l:tt)* inner $inner:ident >> } => {};
	{ lifetime $l:tt } => {};
}

macro_rules! setter {
	{ $rfn:ident($n:ident : $rty:ty) = $cfn:ident } => {
		pub fn $rfn(&mut self, $n: $rty) {
			unsafe{::mbedtls_sys::$cfn(&mut self.inner,$n.into())}
		}
	}
}

// can't make this work without as as_XXX! macro, and there is no as_method!...
macro_rules! setter_callback {
	{ $s:ident<$l:tt>::$rfn:ident($n:ident : $($rty:tt)+) = $cfn:ident } => {
		as_item!(
		impl<$l> $s<$l> {
			pub fn $rfn<F: $($rty)+>(&mut self, $n: Option<&$l mut F>) {
				unsafe{::mbedtls_sys::$cfn(
					&mut self.inner,
					$n.as_ref().map(|_|F::call as _),
					$n.map(|f|f.data_ptr()).unwrap_or(::core::ptr::null_mut())
				)}
			}
		}
		);
	}
}

macro_rules! getter {
	{ $rfn:ident() -> $rty:ty = .$cfield:ident } => {
		pub fn $rfn(&self) -> $rty {
			self.inner.$cfield.into()
		}
	};
	{ $rfn:ident() -> $rty:ty = fn $cfn:ident } => {
		pub fn $rfn(&self) -> $rty {
			unsafe{::mbedtls_sys::$cfn(&self.inner).into()}
		}
	};
}
