use core::{
    alloc::{Layout, LayoutError},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    ptr::{NonNull, Pointee},
};
use std::cell::Cell;

use crate::{gc_vtable::GcVTable, Collect, Collector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) enum Colour {
    /// The allocation is not pointed to by any `Gc` or `WeakGc` that have been traced.
    #[default]
    White,
    /// The allocation is pointed to by a traced `Gc`, but the contained value has not yet been
    /// traced itself.
    Gray,
    /// The allocation is pointed to by a traced `WeakGc`. The contained value does not need tracing.
    Weak,
    /// The allocation is pointed to by a traced `Gc`, it has also been traced.
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErasedPtr(NonNull<()>);

impl ErasedPtr {
    fn inner(&self) -> &GcInner<()> {
        unsafe { &*self.0.as_ptr().cast::<GcInner<()>>() }
    }

    pub fn colour(&self) -> Colour {
        self.inner().colour.get()
    }

    pub fn set_colour(&self, c: Colour) {
        self.inner().colour.set(c);
    }

    pub unsafe fn set_init(&self) {
        self.inner().initialized.set(true);
    }

    pub fn set_uninit(&self) {
        self.inner().initialized.set(false);
    }

    pub fn is_initialized(&self) -> bool {
        self.inner().initialized.get()
    }

    pub fn next_gc(&self) -> Option<ErasedPtr> {
        self.inner().next_gray.get()
    }

    pub fn set_next_gc(&self, gc: Option<ErasedPtr>) {
        self.inner().next_gray.set(gc);
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr().cast()
    }

    pub fn vtable(&self) -> &'static GcVTable {
        self.inner().vtable
    }

    pub unsafe fn trace_value(&self, c: &Collector) {
        self.vtable().collect(*self, c);
    }

    pub unsafe fn drop_in_place(&self) {
        unsafe { self.vtable().drop_in_place(*self) };
    }

    pub fn layout(&self) -> Layout {
        self.inner().layout
    }

    pub unsafe fn restore_type<T: ?Sized>(self) -> GcPtr<T> {
        GcPtr(self.0, PhantomData)
    }
}

#[repr(transparent)]
pub(crate) struct GcPtr<T: ?Sized, const INIT: bool = true>(NonNull<()>, PhantomData<*const T>);

impl<T: ?Sized + Collect + Pointee> GcPtr<T, false> {
    pub unsafe fn new(ptr: *mut u8, meta: T::Metadata, layout: Layout) -> GcPtr<T, false> {
        let gc = GcInner {
            vtable: GcVTable::new::<T>(),
            next_gray: Cell::new(None),
            colour: Cell::new(Colour::White),
            initialized: Cell::new(false),
            layout,
            meta,
            data: (),
        };

        unsafe { ptr.cast::<GcInner<(), T::Metadata>>().write(gc) };

        GcPtr(unsafe { NonNull::new_unchecked(ptr.cast()) }, PhantomData)
    }
}

impl<T: ?Sized + Pointee, const INIT: bool> GcPtr<T, INIT> {
    pub(crate) fn colour(&self) -> Colour {
        unsafe { &(*self.inner_ptr()).colour }.get()
    }

    pub(crate) fn set_colour(&self, colour: Colour) {
        unsafe { &(*self.inner_ptr()).colour }.set(colour)
    }

    pub(crate) fn vtable(&self) -> &'static GcVTable {
        unsafe { (&raw const (*self.inner_ptr()).vtable).read() }
    }

    pub(crate) fn set_vtable(&self)
    where
        T: Collect,
    {
        unsafe {
            (&raw mut (*self.inner_ptr()).vtable).write(GcVTable::new::<T>());
        }
    }

    pub(crate) unsafe fn transmute<U: ?Sized + Collect>(self) -> GcPtr<U, INIT> {
        let gc = GcPtr::<U, INIT>(self.0, PhantomData);
        gc.set_vtable();
        gc
    }

    fn metadata(&self) -> T::Metadata {
        unsafe { self.0.cast::<GcInner<(), T::Metadata>>().read() }.meta
    }

    pub fn data_ptr(&self) -> *mut T {
        let meta = self.metadata();

        let ptr: *mut GcInner<T> = core::ptr::from_raw_parts_mut(self.0.as_ptr(), meta);

        unsafe { &raw mut (*ptr).data }
    }

    pub unsafe fn data(&self) -> &T {
        debug_assert!(self.inner().initialized.get(), "data field not initialized");
        unsafe { &*self.data_ptr() }
    }

    pub fn inner_ptr(&self) -> *mut GcInner<T> {
        let meta = self.metadata();
        let ptr: *mut GcInner<T> = core::ptr::from_raw_parts_mut(self.0.as_ptr(), meta);

        ptr
    }

    pub fn inner(&self) -> &GcInner<T> {
        unsafe { &*self.inner_ptr() }
    }

    pub fn as_ptr(&self) -> NonNull<GcInner<T>> {
        let metadata = self.metadata();
        NonNull::from_raw_parts(self.0, metadata)
    }
}

impl<T: ?Sized> GcPtr<T, false> {
    pub unsafe fn assume_init(self) -> GcPtr<T, true> {
        GcPtr(self.0, PhantomData)
    }
}

impl<T: ?Sized> GcPtr<T> {
    pub fn erase(&self) -> ErasedPtr {
        ErasedPtr(self.0)
    }
}

impl<T: ?Sized> AsRef<T> for GcPtr<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.data() }
    }
}

impl<T: ?Sized> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: ?Sized, const INIT: bool> Clone for GcPtr<T, INIT> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, const INIT: bool> Copy for GcPtr<T, INIT> {}

impl<T: ?Sized + Debug, const INIT: bool> Debug for GcPtr<T, INIT> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match INIT {
            true => f
                .debug_tuple("GcPtr")
                .field(&unsafe { self.data() })
                .finish(),
            false => f.debug_tuple("GcPtr").field(&"<uninit>").finish(),
        }
    }
}

impl<T: ?Sized + PartialEq, const A: bool, const B: bool> PartialEq<GcPtr<T, A>> for GcPtr<T, B> {
    fn eq(&self, other: &GcPtr<T, A>) -> bool {
        if A && B {
            let (a, b) = unsafe { (self.data(), other.data()) };

            a.eq(b)
        } else {
            false
        }
    }
}

impl<T: ?Sized + Eq> Eq for GcPtr<T, true> {}

impl<T: ?Sized + PartialOrd, const A: bool, const B: bool> PartialOrd<GcPtr<T, A>> for GcPtr<T, B> {
    fn partial_cmp(&self, other: &GcPtr<T, A>) -> Option<core::cmp::Ordering> {
        if A && B {
            let (a, b) = unsafe { (self.data(), other.data()) };

            a.partial_cmp(b)
        } else {
            None
        }
    }
}

impl<T: ?Sized + Ord> Ord for GcPtr<T, true> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for GcPtr<T, true> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

macro_rules! generate_gc_inner {
    ($( $(#[$m:meta])* $field:ident : $t:ty),+) => {
        #[repr(C)]
        pub(crate) struct GcInner<T: ?Sized, M = <T as Pointee>::Metadata> {
            $( $(#[$m])* $field: $t, )+
            meta: M,
            data: T,
        }

        impl<T: ?Sized, M> GcInner<T, M> {
            pub fn layout(data_layout: Layout) -> Result<Layout, LayoutError> {
                let layout = Layout::new::<()>();
                $( let (layout, _) = layout.extend(Layout::new::< $t >())?; )+
                let (layout, _) = layout.extend(Layout::new::<M>())?;
                let (layout, _) = layout.extend(data_layout)?;
                Ok(layout.pad_to_align())
            }
        }
    };
}

generate_gc_inner! {
    vtable: &'static GcVTable,
    /// Creates a linked list chaining the garbage collected allocations together. This field
    /// points to the allocation that will be inspected next.
    next_gray: Cell<Option<ErasedPtr>>,
    colour: Cell<Colour>,
    initialized: Cell<bool>,
    layout: Layout
}

#[cfg(test)]
mod tests {
    use super::*;
}
