use std::{
    mem::{ManuallyDrop, MaybeUninit},
    ops::Deref,
};

use crate::context::Collector;

/// Used to mark each garbage collected pointer that can be reached from the implementing value.
///
/// # Safety
/// To not cause undefined behaviour, the implementation of trace *must* call `trace` on every
/// value which can be reached from this value, and which implements `Collect`, and for which
/// `Collect::NEEDS_TRACE` is true.
///
/// The value of `NEEDS_TRACE` may be true, even if it doesn't actually need to be traced, but
/// *must* be true if the implementor can provide a reference to a `Gc`.
///
/// The implementing type must not contain interior mutability in such a way as to allow a
/// garbage collected pointer to be adopted, modified, or replaces, except if that interior
/// mutability is gated behind an [`Unlock`] implementation
///
/// [`Unlock`]: crate::locked::Unlock
pub unsafe trait Collect {
    const NEEDS_TRACE: bool;

    fn trace(&self, c: &Collector);
}

macro_rules! unsafe_impl_collect {
    ($t:ty) => {
        unsafe impl Collect for $t {
            const NEEDS_TRACE: bool = false;

            fn trace(&self, _: &Collector) {}
        }
    };
    ($($t:ty),*) => {
        $(
            unsafe_impl_collect!($t);
        )*
    };
}

unsafe_impl_collect!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    bool,
    (),
    f32,
    f64,
    str
);

unsafe impl<T> Collect for &T
where
    T: Collect,
{
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &Collector) {
        (*self).trace(c)
    }
}

unsafe impl<T> Collect for &mut T
where
    T: Collect,
{
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &Collector) {
        (**self).trace(c);
    }
}

macro_rules! unsafe_impl_collect_iterable {
    ($t:ty) => {
        unsafe impl<T> Collect for $t
        where
            T: Collect,
        {
            const NEEDS_TRACE: bool = T::NEEDS_TRACE;

            fn trace(&self, c: &Collector) {
                #[allow(for_loops_over_fallibles)]
                for el in self {
                    el.trace(c);
                }
            }
        }
    };
    ($($t:ty),*) => {
        $(
            unsafe_impl_collect_iterable!($t);
        )*
    }
}

unsafe_impl_collect_iterable!(
    [T],
    Option<T>,
    alloc::vec::Vec<T>,
    alloc::collections::BinaryHeap<T>,
    alloc::collections::BTreeSet<T>,
    alloc::collections::LinkedList<T>,
    alloc::collections::VecDeque<T>
);

unsafe impl<T: Collect, const N: usize> Collect for [T; N] {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &Collector) {
        for el in self {
            el.trace(c);
        }
    }
}

/// This implementation is sound because the value contained within the `MaybeUninit` cannot be
/// accessed without unsafe code. What this does mean is that an additional safety condition is
/// added to [`MaybeUninit::assume_init`], which is that it can only be called within the same
/// arena cycle that wrote to the `MaybeUninit`.
unsafe impl<T> Collect for MaybeUninit<T> {
    const NEEDS_TRACE: bool = false;

    fn trace(&self, _: &Collector) {}
}

unsafe impl<T> Collect for ManuallyDrop<T>
where
    T: Collect,
{
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &Collector) {
        self.deref().trace(c);
    }
}

unsafe impl<T, E> Collect for Result<T, E>
where
    T: Collect,
    E: Collect,
{
    const NEEDS_TRACE: bool = T::NEEDS_TRACE || E::NEEDS_TRACE;

    fn trace(&self, c: &Collector) {
        match self {
            Ok(v) => v.trace(c),
            Err(e) => e.trace(c),
        }
    }
}

unsafe impl<T> Collect for std::task::Poll<T>
where
    T: Collect,
{
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &Collector) {
        if let std::task::Poll::Ready(val) = self {
            val.trace(c);
        }
    }
}

macro_rules! tuple_impl {
    ($($t:ident),+) => {
        unsafe impl< $( $t: Collect ),+ > Collect for ($($t,)+)
        {
            const NEEDS_TRACE: bool = false $( || $t::NEEDS_TRACE )+;

            fn trace(&self, c: &Collector) {
                #[allow(non_snake_case)]
                let ( $( $t, )+ ) = self;

                $(
                    $t.trace(c);
                )+
            }
        }
    };
}

tuple_impl!(A);
tuple_impl!(A, B);
tuple_impl!(A, B, C);
tuple_impl!(A, B, C, D);
tuple_impl!(A, B, C, D, E);
tuple_impl!(A, B, C, D, E, F);
tuple_impl!(A, B, C, D, E, F, G);
tuple_impl!(A, B, C, D, E, F, G, H);
tuple_impl!(A, B, C, D, E, F, G, H, I);
tuple_impl!(A, B, C, D, E, F, G, H, I, J);
tuple_impl!(A, B, C, D, E, F, G, H, I, J, K);
tuple_impl!(A, B, C, D, E, F, G, H, I, J, K, L);
