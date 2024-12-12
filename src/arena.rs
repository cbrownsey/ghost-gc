use std::{
    alloc::{Allocator, Global, Layout},
    marker::PhantomData,
    ptr::Pointee,
};

use crate::{
    context::{Context, Pacing},
    gc_box::GcBox,
    Collect, Invariant, Mutation,
};
use alloc::boxed::Box;

type ContextAndAlloc<A> = (Box<Context>, A);

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
        let context: Box<Context> = Box::new(Context::new(Pacing::default()));
        let root = f(Mutation::new(&*context));

        Arena { context, root }
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
        let context: Box<Context<A>> = Box::new(Context::new_in(Pacing::default(), alloc));
        let root = f(&Mutation::new(&context));

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
        f(&mut self.root, Mutation::new(&self.context))
    }
}

pub trait Rootable {
    type Root<'l>;
}
