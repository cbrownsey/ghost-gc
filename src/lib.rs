#![feature(
    ptr_metadata,
    strict_provenance,
    ptr_as_ref_unchecked,
    allocator_api,
    never_type
)]
#![deny(unsafe_op_in_unsafe_fn)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod arena;
mod collect;
mod context;
mod gc;
// mod gc_ptr;
mod gc_vtable;
mod unique_gc;
mod write;

mod gc_box;
mod gc_weak;
mod invariant;
pub mod locked;

pub use arena::{Arena, Rootable};
pub use collect::Collect;
pub use context::{Collector, Mutation};
pub use gc::Gc;
pub use gc_weak::Weak;
pub use unique_gc::UniqueGc;
pub use write::Write;

pub use invariant::Invariant;

pub fn once_arena<F, R>(f: F) -> R
where
    F: for<'b> FnOnce(&Mutation<'b>) -> R,
{
    struct OnceRoot;

    impl Rootable for OnceRoot {
        type Root<'a> = OnceRoot;
    }

    unsafe impl Collect for OnceRoot {
        const NEEDS_TRACE: bool = false;

        fn trace(&self, _c: &Collector) {}
    }

    let arena = Arena::<OnceRoot>::new(|_| OnceRoot);

    arena.view(|_, mt| f(mt))
}
