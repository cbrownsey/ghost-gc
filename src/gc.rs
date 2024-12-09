use core::ops::Deref;
use std::{fmt::Debug, hash::Hash};

use crate::{
    context::Mutation, gc_box::GcBox, locked::Unlock, Collect, Invariant, UniqueGc, Weak, Write,
};

/// A thin, copyable, garbage collected pointer type.
pub struct Gc<'b, T: ?Sized>(GcBox<T>, Invariant<'b>);

impl<'b, T: Collect> Gc<'b, T> {
    /// Allocates garbage collected memory on the heap and then places `val`
    /// into it.
    ///
    /// This allocates regardless of if `T` is zero-sized.
    ///
    /// If initialization of a more complex type is required, see [`UniqueGc`].
    pub fn new(val: T, mt: &Mutation<'b>) -> Gc<'b, T> {
        let this = UniqueGc::new(val, mt);
        UniqueGc::into_gc(this)
    }
}

impl<'b> Gc<'b, str> {
    pub fn from_str(s: &str, mt: &Mutation<'b>) -> Gc<'b, str> {
        UniqueGc::into_gc(UniqueGc::from_str(s, mt))
    }
}

impl<'b, T: ?Sized> Gc<'b, T> {
    pub fn write(&self) -> &Write<T> {
        unsafe { Write::new_unchecked(self) }
    }

    pub fn unlock(&self) -> &T::Unlocked
    where
        T: Unlock,
    {
        self.write().unlock()
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.data_ptr()
    }

    pub fn downgrade(this: Gc<'b, T>) -> Weak<'b, T> {
        unsafe { Weak::from_box(this.0) }
    }

    // pub(crate) fn into_box(self) -> GcBox<T> {
    //     self.0
    // }

    pub(crate) unsafe fn from_box(ptr: GcBox<T>) -> Gc<'b, T> {
        Gc(ptr, Invariant)
    }
}

impl<T: ?Sized> Deref for Gc<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.data() }
    }
}

impl<T: ?Sized> Clone for Gc<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Gc<'_, T> {}

impl<T: Debug + ?Sized> Debug for Gc<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Gc<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + Eq> Eq for Gc<'_, T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for Gc<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<T: ?Sized + Ord> Ord for Gc<'_, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(other)
    }
}

impl<T: ?Sized + Hash> Hash for Gc<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

unsafe impl<T: ?Sized> Collect for Gc<'_, T> {
    const NEEDS_TRACE: bool = true;

    fn trace(&self, _c: &crate::Collector) {
        todo!()
    }
}
