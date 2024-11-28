#![feature(ptr_metadata, strict_provenance, ptr_as_ref_unchecked, allocator_api)]
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
pub mod locked;

pub use arena::{Arena, Rootable};
pub use collect::Collect;
pub use context::{Collector, Mutation};
pub use gc::Gc;
pub use gc_weak::Weak;
pub use unique_gc::UniqueGc;
pub use write::Write;

pub(crate) type Invariant<'l> = core::marker::PhantomData<fn(&'l ()) -> &'l ()>;

pub fn once_arena<F, R>(f: F) -> R
where
    F: for<'b> FnOnce(&Mutation<'b>) -> R,
{
    struct OnceRoot;

    impl Rootable for OnceRoot {
        type Root<'a> = OnceRoot;
    }

    let arena = Arena::<OnceRoot>::new(|_| OnceRoot);

    arena.view(|_, mt| f(mt))
}
