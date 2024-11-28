#![allow(dead_code)]

use core::ptr::Pointee;
use std::{
    alloc::{Layout, LayoutError},
    cell::Cell,
    marker::PhantomData,
    ptr::NonNull,
};

use crate::{gc_vtable::GcVTable, Collect, Collector};

pub struct Erased;

#[repr(transparent)]
pub(crate) struct GcBox<T: ?Sized>(NonNull<GcInner<()>>, PhantomData<T>);

impl<T: ?Sized> Clone for GcBox<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for GcBox<T> {}

impl GcBox<Erased> {
    /// # Safety
    /// The erased box must have come from a call to `GcBox<T>::erase`.
    pub unsafe fn restore_type<T: ?Sized>(self) -> GcBox<T> {
        GcBox(self.0, PhantomData)
    }
}

impl<T: ?Sized> GcBox<T> {
    pub fn inner_layout(data: Layout) -> Result<Layout, LayoutError> {
        let layout = Layout::new::<GcHeader>();
        let (layout, _) = layout.extend(Layout::new::<<T as Pointee>::Metadata>())?;
        let (layout, _) = layout.extend(data)?;
        Ok(layout.pad_to_align())
    }

    pub unsafe fn new(ptr: *mut u8, metadata: <T as Pointee>::Metadata, layout: Layout) -> GcBox<T>
    where
        T: Collect,
    {
        let layout = GcInner::<T>::layout(layout).unwrap();

        let inner: GcInner<(), <T as Pointee>::Metadata> = GcInner {
            header: GcHeader {
                vtable: Cell::new(GcVTable::new::<T>()),
                next_gray: Cell::new(None),
                colour: Cell::new(Colour::White),
                is_live: Cell::new(false),
                layout,
            },
            metadata,
            data: (),
        };

        unsafe {
            ptr.cast::<GcInner<(), <T as Pointee>::Metadata>>()
                .write(inner)
        };

        GcBox(unsafe { NonNull::new_unchecked(ptr.cast()) }, PhantomData)
    }

    pub fn into_raw(self) -> NonNull<()> {
        self.0.cast::<()>()
    }

    pub unsafe fn from_raw(ptr: NonNull<()>) -> GcBox<T> {
        GcBox(ptr.cast(), PhantomData)
    }

    pub unsafe fn collect_value(&self, c: &Collector) {
        if self.is_initialized() {
            unsafe {
                self.header().vtable.get().collect(self.erase(), c);
            }
        }
    }

    pub unsafe fn drop_in_place(&self) {
        if self.is_initialized() {
            unsafe { self.header().vtable.get().drop_in_place(self.erase()) };
        }
        self.set_uninit();
    }

    pub fn next_gc(&self) -> Option<GcBox<Erased>> {
        self.header().next_gray.get()
    }

    pub fn set_next(&self, next: Option<GcBox<Erased>>) {
        self.header().next_gray.set(next);
    }

    pub unsafe fn set_init(&self) {
        self.header().is_live.set(true);
    }

    pub fn set_uninit(&self) {
        self.header().is_live.set(false);
    }

    pub fn is_initialized(&self) -> bool {
        self.header().is_live.get()
    }

    pub fn colour(&self) -> Colour {
        self.header().colour.get()
    }

    pub unsafe fn set_colour(&self, c: Colour) {
        self.header().colour.set(c)
    }

    fn header(&self) -> &GcHeader {
        unsafe { self.0.cast::<GcHeader>().as_ref() }
    }

    pub fn vtable(&self) -> &'static GcVTable {
        self.header().vtable.get()
    }

    pub fn layout(&self) -> Layout {
        self.header().layout
    }

    pub unsafe fn set_vtable<U: ?Sized + Collect>(&self) {
        self.header().vtable.set(GcVTable::new::<U>())
    }

    pub fn metadata(&self) -> <T as Pointee>::Metadata {
        let ptr = self
            .0
            .as_ptr()
            .cast::<GcInner<(), <T as Pointee>::Metadata>>();

        unsafe { (&raw const (*ptr).metadata).read() }
    }

    pub fn inner_ptr(&self) -> *mut GcInner<T> {
        core::ptr::from_raw_parts_mut(self.0.as_ptr(), self.metadata())
    }

    pub fn data_ptr(&self) -> *mut T {
        let ptr = self.inner_ptr();

        unsafe { &raw mut (*ptr).data }
    }

    pub unsafe fn data(&self) -> &T {
        debug_assert!(self.is_initialized());
        unsafe { self.data_ptr().as_ref_unchecked() }
    }

    pub unsafe fn data_mut(&mut self) -> &mut T {
        debug_assert!(self.is_initialized());
        unsafe { self.data_ptr().as_mut_unchecked() }
    }

    pub fn erase(self) -> GcBox<Erased> {
        GcBox(self.0, PhantomData)
    }

    pub unsafe fn transmute<U: ?Sized + Collect>(self) -> GcBox<U> {
        let gc = GcBox(self.0, PhantomData);
        gc.header().vtable.set(GcVTable::new::<U>());
        gc
    }
}

struct GcHeader {
    vtable: Cell<&'static GcVTable>,
    next_gray: Cell<Option<GcBox<Erased>>>,
    colour: Cell<Colour>,
    is_live: Cell<bool>,
    /// The layout of the whole `GcInner`
    layout: Layout,
}

#[repr(C)]
pub(crate) struct GcInner<T: ?Sized, M = <T as Pointee>::Metadata> {
    header: GcHeader,
    metadata: M,
    data: T,
}

impl<T: ?Sized, M> GcInner<T, M> {
    pub fn layout(data: Layout) -> Result<Layout, LayoutError> {
        let layout = Layout::new::<GcHeader>();
        let (layout, _) = layout.extend(data)?;
        let (layout, _) = layout.extend(layout)?;
        Ok(layout.pad_to_align())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Colour {
    #[default]
    White,
    Weak,
    Gray,
    Black,
}
