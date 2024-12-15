//! Shareable containers which can be made mutable when inside a [`Gc`].
//!
//! [`Gc`]: crate::Gc

use std::cell::{Cell, OnceCell, RefCell};

use crate::Collect;

/// A marker for types which allow a [`Collect`] implementation on an
/// interiorly mutable type.
pub trait Unlock {
    type Unlocked: ?Sized;

    /// # Safety
    /// The parent `Gc` must have been marked as unlocked.
    unsafe fn unlock_unchecked(&self) -> &Self::Unlocked;
}

#[derive(Default)]
#[repr(transparent)]
pub struct LockedCell<T: ?Sized>(Cell<T>);

impl<T> LockedCell<T> {
    pub const fn new(value: T) -> LockedCell<T> {
        LockedCell(Cell::new(value))
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T: Copy> LockedCell<T> {
    pub fn get(&self) -> T {
        self.0.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

impl<T: ?Sized> LockedCell<T> {
    pub const fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T> Clone for LockedCell<T>
where
    T: Copy,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> std::fmt::Debug for LockedCell<T>
where
    T: std::fmt::Debug + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LockedCell").field(&self.get()).finish()
    }
}

impl<T> From<T> for LockedCell<T> {
    fn from(value: T) -> Self {
        LockedCell(Cell::from(value))
    }
}

impl<T> PartialEq for LockedCell<T>
where
    T: PartialEq + Copy,
{
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T> Eq for LockedCell<T> where T: Eq + Copy {}

impl<T> PartialOrd for LockedCell<T>
where
    T: PartialOrd + Copy,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T> Ord for LockedCell<T>
where
    T: Ord + Copy,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: ?Sized> Unlock for LockedCell<T> {
    type Unlocked = core::cell::Cell<T>;

    unsafe fn unlock_unchecked(&self) -> &Self::Unlocked {
        &self.0
    }
}

unsafe impl<T: Copy + Collect> Collect for LockedCell<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &crate::Collector) {
        self.get().trace(c)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LockedRefCell<T: ?Sized>(core::cell::RefCell<T>);

impl<T> LockedRefCell<T> {
    pub const fn new(value: T) -> LockedRefCell<T> {
        Self(RefCell::new(value))
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> LockedRefCell<T>
where
    T: ?Sized,
{
    pub fn borrow(&self) -> core::cell::Ref<'_, T> {
        self.0.borrow()
    }

    pub fn try_borrow(&self) -> Result<core::cell::Ref<'_, T>, core::cell::BorrowError> {
        self.0.try_borrow()
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

impl<T: ?Sized> Unlock for LockedRefCell<T> {
    type Unlocked = core::cell::RefCell<T>;

    unsafe fn unlock_unchecked(&self) -> &Self::Unlocked {
        &self.0
    }
}

unsafe impl<T: ?Sized + Collect> Collect for LockedRefCell<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &crate::Collector) {
        self.borrow().trace(c);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct LockedOnceCell<T>(core::cell::OnceCell<T>);

impl<T> LockedOnceCell<T> {
    pub const fn new() -> LockedOnceCell<T> {
        LockedOnceCell(OnceCell::new())
    }

    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.0.get_mut()
    }

    pub fn into_inner(self) -> Option<T> {
        self.0.into_inner()
    }

    pub fn take(&mut self) -> Option<T> {
        self.0.take()
    }
}

impl<T> Unlock for LockedOnceCell<T> {
    type Unlocked = core::cell::OnceCell<T>;

    unsafe fn unlock_unchecked(&self) -> &Self::Unlocked {
        &self.0
    }
}

unsafe impl<T: Collect> Collect for LockedOnceCell<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &crate::Collector) {
        self.get().trace(c);
    }
}
