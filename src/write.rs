#[cfg(doc)]
use crate::Gc;

use crate::locked::Unlock;

/// A marker type which indicates that any owning [`Gc`] has been marked as having been modified.
#[repr(transparent)]
pub struct Write<T: ?Sized>(T);

impl<T: ?Sized> Write<T> {
    /// # Safety
    /// The parent [`Gc`] must have been marked as mutated.
    pub unsafe fn new_unchecked(value: &T) -> &Write<T> {
        // Safety: Write is a thin wrapper around `T`.
        unsafe { std::mem::transmute(value) }
    }

    pub fn new_static(value: &T) -> &Write<T>
    where
        T: 'static,
    {
        unsafe { Write::new_unchecked(value) }
    }

    pub fn into_inner(&self) -> &T {
        &self.0
    }

    pub fn unlock(&self) -> &T::Unlocked
    where
        T: Unlock,
    {
        unsafe { self.0.unlock_unchecked() }
    }

    /// Projects a write permission into a write permision of the of the values contained by
    /// `self`.
    ///
    /// # Panics
    /// When the closure returns a reference to a value not contained within the bounds of `self`.
    pub fn project<U: ?Sized>(&self, f: impl for<'a> FnOnce(&'a T) -> &'a U) -> &Write<U> {
        self.try_project(f).unwrap()
    }

    /// Projects a write permission into a write permision of the of the values contained by
    /// `self`, returning an error if the closure returns a reference to a value not contained
    /// within the bounds of `self`.
    pub fn try_project<U: ?Sized>(
        &self,
        f: impl for<'a> FnOnce(&'a T) -> &'a U,
    ) -> Result<&Write<U>, WriteProjectError> {
        let size = size_of_val(self) as isize;
        let self_addr = (self as *const Write<T>).addr() as isize;
        let proj = f(&self.0);
        let proj_addr = (proj as *const U).addr() as isize;

        if (0..size).contains(&(proj_addr - self_addr)) {
            unsafe { Ok(Write::new_unchecked(proj)) }
        } else {
            Err(WriteProjectError)
        }
    }

    /// Projects a write permission into a write permission of one of the containing objects fields.
    ///
    /// # Safety
    /// The given closure must return a reference to a value which is owned by self. The closure
    /// *must not* dereference a [`Gc`], or in any way project into a value which is owned
    /// by another garbage collected pointer, and which could itself contain a garbage collected
    /// pointer.
    ///
    /// # Examples
    /// ```
    /// # use ghost_gc::{locked::LockedCell, Gc, once_arena, Collect, Collector};
    /// # once_arena(|mt| {
    /// #
    /// # unsafe impl<T: Collect> Collect for LinkedList<'_, T> {
    /// #   const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    /// #
    /// #   fn trace(&self, c: &Collector) {
    /// #       self.data.trace(c);
    /// #       match self.next.get() {
    /// #           Some(v) => v.trace(c),
    /// #           None => {}
    /// #       }
    /// #   }
    /// # }
    /// #
    /// #[derive(Debug)]
    /// struct LinkedList<'b, T> {
    ///     data: T,
    ///     next: LockedCell<Option<Gc<'b, Self>>>,
    /// }
    ///
    /// let head = Gc::new(LinkedList::<'_, u32> {
    ///     data: 0,
    ///     next: LockedCell::new(Some(Gc::new(LinkedList {
    ///         data: 1,
    ///         next: LockedCell::new(None)
    ///     }, mt)))
    /// }, mt);
    ///
    /// unsafe {
    ///     head.write().project_unchecked(|x| &x.data);
    ///     head.write().project_unchecked(|x| &x.next);
    /// }
    /// # });
    /// ```
    pub unsafe fn project_unchecked<U: ?Sized>(
        &self,
        f: impl for<'a> FnOnce(&'a T) -> &'a U,
    ) -> &Write<U> {
        let self_ref: &T = &self.0;

        let proj = f(self_ref);

        unsafe { Write::new_unchecked(proj) }
    }
}

#[derive(Debug)]
pub struct WriteProjectError;

#[cfg(test)]
mod tests {
    use crate::Write;

    #[test]
    fn basic_projection() {
        struct Test {
            a: u32,
            b: &'static str,
        }

        let t = Test {
            a: 17,
            b: "Hello, World!",
        };

        let w = Write::new_static(&t);

        let _: &Write<u32> = w.project(|f| &f.a);
        let _: &Write<&str> = w.project(|f| &f.b);
    }

    #[test]
    #[should_panic]
    fn incorrect_projection() {
        struct Test {
            _a: &'static str,
        }

        let t = Test {
            _a: "Hello, World!",
        };

        let w = Write::new_static(&t);

        let _ = w.project(|_| "Some other string.");
    }
}
