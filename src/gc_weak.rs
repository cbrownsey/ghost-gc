use std::{marker::PhantomData, ptr::NonNull};

use crate::{gc_box::GcBox, Collect, Gc, Invariant};

pub struct Weak<'b, T: ?Sized>(NonNull<()>, Invariant<'b>, PhantomData<*const T>);

impl<'b, T: ?Sized> Default for Weak<'b, T> {
    fn default() -> Self {
        Weak(
            NonNull::new(core::ptr::without_provenance_mut(usize::MAX)).unwrap(),
            Invariant,
            PhantomData,
        )
    }
}

impl<'b, T: ?Sized> Weak<'b, T> {
    pub fn new() -> Weak<'b, T> {
        Weak::default()
    }

    pub(crate) fn into_box(self) -> Option<GcBox<T>> {
        if self.0.addr().get() == usize::MAX {
            None
        } else {
            // Safety: If the pointer isn't `usize::MAX`, it must be a pointer which came from
            // `GcBox::into_raw`.
            Some(unsafe { GcBox::from_raw(self.0) })
        }
    }

    pub(crate) unsafe fn from_box(ptr: GcBox<T>) -> Weak<'b, T> {
        Weak(ptr.into_raw(), Invariant, PhantomData)
    }

    pub fn upgrade(self) -> Option<Gc<'b, T>> {
        if let Some(b) = self.into_box() {
            if b.is_initialized() {
                Some(unsafe { Gc::from_box(b) })
            } else {
                None
            }
        } else {
            None
        }
    }
}

unsafe impl<'b, T: ?Sized> Collect for Weak<'b, T> {
    const NEEDS_TRACE: bool = true;

    fn trace(&self, _c: &crate::Collector) {
        todo!()
    }
}
