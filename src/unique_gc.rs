use core::{
    alloc::Layout,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::Pointee,
};
use std::fmt::Debug;

use crate::{context::Mutation, gc::Gc, gc_box::GcBox, Collect, Invariant};

/// A thin, garbage collected pointer type, which is guaranteed to be unique.
pub struct UniqueGc<'b, T: ?Sized>(GcBox<T>, Invariant<'b>);

impl<'b, T> UniqueGc<'b, T> {
    /// Allocates garbage collected memory on the heap and then places `val`
    /// into it.
    ///
    /// This allocates regardless of if `T` is zero-sized.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let five = UniqueGc::new(5, mt);
    /// # });
    /// ```
    pub fn new(val: T, mt: &Mutation<'b>) -> UniqueGc<'b, T>
    where
        T: Collect,
    {
        let inner = mt.allocate::<T>((), Layout::new::<T>());
        // .context()
        // .allocate::<T>((), Layout::new::<T>(), mt.alloc());
        // Safety: No references exist, as the pointer was just created.
        unsafe { inner.data_ptr().write(val) };
        // Safety: The value was just written.
        unsafe { inner.set_init() };

        UniqueGc(inner, Invariant)
    }

    /// Constructs a new garbage collected pointer with uninitialized contents.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let mut five = UniqueGc::<u32>::new_uninit(mt);
    ///
    /// let five = unsafe {
    ///     five.as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # });
    /// ```
    pub fn new_uninit(mt: &Mutation<'b>) -> UniqueGc<'b, MaybeUninit<T>> {
        UniqueGc::new(MaybeUninit::uninit(), mt)
    }

    /// Constructs a new garbage collected pointer with uninitialized contents,
    /// with the memory being filled with `0` bytes.
    ///
    /// See [`MaybeUninit::zeroed`] for examples of correct and incorrect usage
    /// of this method.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let zero = UniqueGc::<u32>::new_zeroed(mt);
    /// let zero = unsafe { zero.assume_init() };
    ///
    /// assert_eq!(*zero, 0);
    /// # });
    /// ```
    pub fn new_zeroed(mt: &Mutation<'b>) -> UniqueGc<'b, MaybeUninit<T>> {
        UniqueGc::new(MaybeUninit::zeroed(), mt)
    }
}

impl<'b, T> UniqueGc<'b, [T]> {
    /// Constructs a new garbage collected slice with uninitialized contents.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let mut values = UniqueGc::<[u32]>::new_uninit_slice(3, mt);
    ///
    /// let values = unsafe {
    ///     values[0].as_mut_ptr().write(1);
    ///     values[1].as_mut_ptr().write(2);
    ///     values[2].as_mut_ptr().write(3);
    ///
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// # });
    /// ```
    pub fn new_uninit_slice(len: usize, mt: &Mutation<'b>) -> UniqueGc<'b, [MaybeUninit<T>]> {
        let inner = mt
            .context()
            .allocate::<[MaybeUninit<T>]>(len, Layout::array::<T>(len).unwrap());

        unsafe { inner.set_init() };

        UniqueGc(inner, Invariant)
    }

    /// Constructs a new garbage collected slice with uninitialized contents, with the memory being
    /// filled with `0` bytes.
    ///
    /// See [`MaybeUninit::zeroed`] for examples of correct and incorrect usage of this method.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let values = UniqueGc::<[u32]>::new_zeroed_slice(3, mt);
    /// let values = unsafe { values.assume_init() };
    ///
    /// assert_eq!(*values, [0, 0, 0]);
    /// # });
    /// ```
    pub fn new_zeroed_slice(len: usize, mt: &Mutation<'b>) -> UniqueGc<'b, [MaybeUninit<T>]> {
        let inner = mt
            .context()
            .allocate::<[MaybeUninit<T>]>(len, Layout::array::<T>(len).unwrap());

        unsafe { core::ptr::write_bytes(inner.data_ptr().cast::<T>(), 0, len) };

        unsafe { inner.set_init() };

        UniqueGc(inner, Invariant)
    }
}

impl<'b> UniqueGc<'b, str> {
    /// Constructs a new garbage collected string, copied from the passed value.
    ///
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let s = UniqueGc::from_str("Hello, World!", mt);
    /// assert_eq!(&*s, "Hello, World!");
    /// # });
    /// ```
    pub fn from_str(s: &str, mt: &Mutation<'b>) -> UniqueGc<'b, str> {
        let mut gc = UniqueGc::<[u8]>::new_uninit_slice(s.len(), mt);

        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), gc.as_mut_ptr().cast(), s.len()) };

        unsafe { gc.transmute() }
    }
}

impl<'b, T: Collect> UniqueGc<'b, MaybeUninit<T>> {
    /// Converts to `UniqueGc<'b, T>`.
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to
    /// guarantee that the value really is in an initialized state. Calling
    /// this when the content is not yet fully initialized causes immediate
    /// undefined behaviour.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let mut five = UniqueGc::<u32>::new_uninit(mt);
    ///
    /// let five = unsafe {
    ///     five.as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # });
    /// ```
    pub unsafe fn assume_init(self) -> UniqueGc<'b, T> {
        unsafe { self.transmute() }
    }

    /// Writes the value and converts to `UniqueGc<'b, T>`.
    ///
    /// This method converts the pointer similarly to [`UniqueGc::assume_init`],
    /// but writes `value` into it before the conversion, thus guaranteeing safety.
    /// In some scenarios use of this method may improve performance because the
    /// compiler may be able to optimize copying from stack.
    pub fn write(mut self, value: T) -> UniqueGc<'b, T> {
        (*self).write(value);
        unsafe { self.assume_init() }
    }
}

impl<'b, T: Collect> UniqueGc<'b, [MaybeUninit<T>]> {
    /// Converts to `Gc<'b, [T]>`.
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the values
    /// really are in an initialized state. Calling this when the content is not yet fully
    /// initialized causes immediate undefined behaviour.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc};
    /// # once_arena(|mt| {
    /// let mut values = UniqueGc::<[u32]>::new_uninit_slice(3, mt);
    ///
    /// let values = unsafe {
    ///     values[0].as_mut_ptr().write(1);
    ///     values[1].as_mut_ptr().write(2);
    ///     values[2].as_mut_ptr().write(3);
    ///
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// # });
    /// ```
    pub unsafe fn assume_init(self) -> UniqueGc<'b, [T]> {
        unsafe { self.transmute::<[T]>() }
    }
}

impl<'b, T: ?Sized> UniqueGc<'b, T> {
    /// Converts the `UniqueGc` into a regular [`Gc`].
    ///
    /// This consumes the `UniqueGc` and returns a regular [`Gc`] which points to the same data
    /// as the `UniqueGc`.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{once_arena, UniqueGc, Gc};
    /// # once_arena(|mt| {
    /// let gc: Gc<u32> = UniqueGc::into_gc(UniqueGc::new(5, mt));
    /// # });
    /// ```
    pub fn into_gc(this: Self) -> Gc<'b, T> {
        unsafe { Gc::from_box(this.0) }
    }

    // /// Consumes the `UniqueGc`, returning a raw pointer.
    // ///
    // /// The resultant pointer is only guaranteed to point to an allocation for the remainder of
    // /// [`Arena::view`] closure.
    // pub(crate) fn into_box(self) -> GcBox<T> {
    //     self.0
    // }

    // /// Constructs a `UniqueGc` from a raw pointer.
    // ///
    // /// # Safety
    // /// The pointer must have come from a previous call to [`UniqueGc::into_raw`].
    // pub(crate) unsafe fn from_box(ptr: GcBox<T>) -> UniqueGc<'b, T> {
    //     UniqueGc(ptr, PhantomData)
    // }

    /// # Safety
    /// Layouts have to match, pointed to data has to match.
    pub(crate) unsafe fn transmute<U: ?Sized + Collect>(self) -> UniqueGc<'b, U> {
        debug_assert_eq!(
            Layout::new::<<T as Pointee>::Metadata>(),
            Layout::new::<<U as Pointee>::Metadata>(),
        );

        UniqueGc::<'b, U>(unsafe { self.0.transmute::<U>() }, Invariant)
    }
}

impl<T: ?Sized> Deref for UniqueGc<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.data() }
    }
}

impl<T: ?Sized> DerefMut for UniqueGc<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.data_mut() }
    }
}

impl<T: ?Sized + Debug> Debug for UniqueGc<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for UniqueGc<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        (**self) == (**other)
    }
}

impl<T: ?Sized + Eq> Eq for UniqueGc<'_, T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for UniqueGc<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<T: ?Sized + Ord> Ord for UniqueGc<'_, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(other)
    }
}

impl<T: ?Sized + std::hash::Hash> std::hash::Hash for UniqueGc<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}
