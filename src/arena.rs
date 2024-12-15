use std::alloc::{Allocator, Global};

use crate::{
    context::{Context, Pacing},
    Collect, Mutation,
};
use alloc::boxed::Box;

/// A garbage collected arena, inside which garbage collected pointers can be allocated.
pub struct Arena<R: Rootable, A = Global>
where
    A: Allocator,
{
    context: Box<Context<A>>,
    root: R::Root<'static>,
}

impl<R> Arena<R>
where
    R: Rootable,
{
    pub fn new<F>(f: F) -> Arena<R>
    where
        F: for<'b> FnOnce(&Mutation<'b>) -> R::Root<'b>,
    {
        Arena::new_in(f, Global)
    }

    pub fn new_paced<F>(f: F, pacing: Pacing) -> Arena<R>
    where
        F: for<'b> FnOnce(&Mutation<'b>) -> R::Root<'b>,
    {
        Arena::new_paced_in(f, pacing, Global)
    }
}

impl<R, A> Arena<R, A>
where
    A: Allocator,
    R: Rootable,
{
    pub fn new_in<F>(f: F, alloc: A) -> Arena<R, A>
    where
        F: for<'b> FnOnce(&Mutation<'b>) -> R::Root<'b>,
        A: Allocator + 'static,
    {
        Arena::new_paced_in(f, Pacing::default(), alloc)
    }

    pub fn new_paced_in<F>(f: F, pacing: Pacing, alloc: A) -> Arena<R, A>
    where
        F: for<'b> FnOnce(&Mutation<'b>) -> R::Root<'b>,
        A: Allocator + 'static,
    {
        let context: Box<Context<A>> = Box::new(Context::new_in(pacing, alloc));
        let root = f(Mutation::new(&context));

        Arena { context, root }
    }

    pub fn view<F, Ret>(&self, f: F) -> Ret
    where
        F: for<'b> FnOnce(&R::Root<'b>, &Mutation<'b>) -> Ret,
    {
        f(&self.root, Mutation::new(&self.context))
    }

    pub fn view_mut<F, Ret>(&mut self, f: F) -> Ret
    where
        F: for<'b> FnOnce(&mut R::Root<'b>, &Mutation<'b>) -> Ret,
    {
        self.context.set_root_untraced();
        f(&mut self.root, Mutation::new(&self.context))
    }

    pub fn run_collection(&mut self) {
        self.context.advance_collection(&self.root);
    }

    pub fn complete_collection(&mut self) {
        self.context.run_full_cycle(&self.root);
    }

    pub fn allocations(&self) -> usize {
        self.context.allocations()
    }
}

pub trait Rootable {
    type Root<'l>: Collect;
}
