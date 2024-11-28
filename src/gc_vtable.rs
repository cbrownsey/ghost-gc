use crate::{
    gc_box::{Erased, GcBox},
    Collect, Collector,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcVTable {
    collect: unsafe fn(GcBox<Erased>, &Collector),
    drop_in_place: unsafe fn(GcBox<Erased>),
}

impl GcVTable {
    pub unsafe fn collect(&self, ptr: GcBox<Erased>, c: &Collector) {
        unsafe { (self.collect)(ptr, c) }
    }

    pub unsafe fn drop_in_place(&self, ptr: GcBox<Erased>) {
        unsafe { (self.drop_in_place)(ptr) }
    }
}

impl GcVTable {
    pub const fn new<T: Collect + ?Sized>() -> &'static GcVTable {
        &const {
            GcVTable {
                collect: |erased: GcBox<Erased>, c| {
                    if T::NEEDS_TRACE {
                        let gc: GcBox<T> = unsafe { erased.restore_type() };
                        unsafe { &*gc.data_ptr() }.trace(c);
                    }
                },
                drop_in_place: |erased: GcBox<Erased>| {
                    let gc: GcBox<T> = unsafe { erased.restore_type() };
                    unsafe { std::ptr::drop_in_place(gc.data_ptr()) };
                },
            }
        }
    }
}
