use core::{alloc::Layout, ptr::Pointee};
use std::{
    alloc::{Allocator, Global},
    cell::{Cell, RefCell},
    ptr::NonNull,
};

use crate::{
    gc_box::{Colour, Erased, GcBox, GcInner},
    Collect, Invariant,
};

#[repr(transparent)]
pub struct Mutation<'b>(Invariant<'b>, Context<dyn Allocator>);

impl<'b> Mutation<'b> {
    pub(crate) fn new<A>(ctx: &Context<A>) -> &Mutation<'b>
    where
        A: Allocator,
    {
        let ctx: &Context<dyn Allocator> = ctx;

        // Safety: `Mutation` is a transparent wrapper around `Context<dyn Allocator>`.
        unsafe { core::mem::transmute::<&Context<dyn Allocator>, &Mutation<'b>>(ctx) }
    }

    pub(crate) fn context(&self) -> &Context<dyn Allocator> {
        &self.1
    }

    pub(crate) fn allocate<T>(&self, meta: <T as Pointee>::Metadata, layout: Layout) -> GcBox<T>
    where
        T: Collect,
    {
        self.context().allocate(meta, layout)
    }
}

impl core::fmt::Debug for Mutation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Mutation")
    }
}

#[repr(transparent)]
pub struct Collector(Context<dyn Allocator>);

impl Collector {
    pub(crate) fn new<A>(ctx: &Context<A>) -> &Collector
    where
        A: Allocator,
    {
        let ctx: &Context<dyn Allocator> = ctx;

        // Safety: `Collector` is a transparent wrapper around a `Context<dyn Allocator>`.
        unsafe { core::mem::transmute::<&Context<dyn Allocator>, &Collector>(ctx) }
    }

    pub(crate) fn context(&self) -> &Context<dyn Allocator> {
        &self.0
    }
}

impl core::fmt::Debug for Collector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Collector")
    }
}

pub(crate) struct Context<A = Global>
where
    A: Allocator + ?Sized,
{
    newly_allocated: RefCell<Vec<GcBox<Erased>>>,
    objects: RefCell<Vec<GcBox<Erased>>>,
    trace_root: Cell<bool>,
    first_gray: Cell<Option<GcBox<Erased>>>,
    phase: Cell<CollectionPhase>,
    cycle_allocations: Cell<usize>,
    cycle_bytes: Cell<usize>,
    pacing: Pacing,
    alloc: A,
}

impl<A: Allocator> Context<A> {
    pub(crate) fn new_in(pacing: Pacing, alloc: A) -> Context<A>
    where
        A: Allocator + 'static,
    {
        Context {
            newly_allocated: Default::default(),
            objects: Default::default(),
            trace_root: Default::default(),
            first_gray: Default::default(),
            phase: Default::default(),
            cycle_allocations: Cell::new(0),
            cycle_bytes: Cell::new(0),
            pacing,
            alloc,
        }
    }

    fn trace_next(&self, root: &impl Collect) -> bool {
        if self.trace_root.get() {
            root.trace(Collector::new(self));
            self.set_root_traced();

            true
        } else if let Some(val) = self.take_next_box() {
            match val.colour() {
                Colour::White | Colour::Weak => unreachable!(),
                Colour::Gray => {
                    unsafe { val.trace_value(Collector::new(self)) };
                    unsafe { val.set_colour(Colour::Black) };
                }
                Colour::Black => {}
            }

            true
        } else {
            false
        }
    }

    fn take_next_box(&self) -> Option<GcBox<Erased>> {
        let ptr = self.first_gray.get()?;
        let next = ptr.next_gc().take();
        self.first_gray.set(next);
        Some(ptr)
    }

    pub fn set_root_untraced(&self) {
        self.trace_root.set(true);
    }

    pub fn set_root_traced(&self) {
        self.trace_root.set(false);
    }

    pub fn allocations(&self) -> usize {
        self.objects.borrow().len() + self.newly_allocated.borrow().len()
    }

    pub fn advance_phase(&self) -> bool {
        match self.phase.get() {
            CollectionPhase::Sleep => {
                self.objects
                    .borrow_mut()
                    .append(&mut *self.newly_allocated.borrow_mut());

                self.cycle_allocations.set(0);
                self.cycle_bytes.set(0);

                self.set_root_untraced();

                for obj in self.objects.borrow().iter() {
                    unsafe { obj.set_colour(Colour::White) };
                    obj.set_next(None);
                }

                self.phase.set(CollectionPhase::Mark);

                false
            }
            CollectionPhase::Mark => {
                self.phase.set(CollectionPhase::Sweep { index: 0 });

                false
            }
            CollectionPhase::Sweep { .. } => {
                self.phase.set(CollectionPhase::Sleep);

                true
            }
        }
    }

    pub fn advance_collection(&self, root: &impl Collect) {
        self.advance_cycle_by(root, self.pacing);
    }

    /// Advances the cycle by the given pacing. If the current phase ends, then this function will
    /// return without making any progress on the next one, regardless of the pacing value.
    pub fn advance_cycle_by(&self, root: &impl Collect, pacing: Pacing) {
        match self.phase.get() {
            CollectionPhase::Sleep => {
                let allocations = self.cycle_allocations.get();
                let bytes = self.cycle_bytes.get();

                if pacing.should_wake(allocations, bytes) {
                    self.advance_phase();
                }
            }
            CollectionPhase::Mark => {
                let mut marked = 0;

                dbg!(&self.first_gray);
                while self.trace_next(root) {
                    marked += 1;

                    dbg!(&self.first_gray);

                    if marked >= self.pacing.mark_stride {
                        return;
                    }
                }

                self.advance_phase();
            }
            CollectionPhase::Sweep { index } => {
                let objects = &mut *self.objects.borrow_mut();

                let mut current = index;
                let mut end =
                    std::cmp::min(index.saturating_add(pacing.sweep_stride), objects.len());

                while current < end {
                    dbg!(&objects, current, end);
                    let obj = objects[current];

                    match obj.colour() {
                        Colour::White => {
                            unsafe { obj.drop_in_place() };
                            objects.swap_remove(current);
                            unsafe { self.deallocate(obj) };
                            end -= 1;
                            continue;
                        }
                        Colour::Gray => unreachable!(),
                        Colour::Weak => {
                            unsafe { obj.drop_in_place() };
                            obj.set_uninit();
                            current += 1;
                            continue;
                        }
                        Colour::Black => {
                            current += 1;
                            continue;
                        }
                    }
                }

                if end == objects.len() {
                    self.advance_phase();
                }
            }
        }
    }

    /// Runs the collection cycle until all allocated objects have been marked and swept.
    pub fn run_full_cycle(&self, root: &impl Collect) {
        // Only need to reset phase if there are objects to be swept.
        // Resetting to sleep phase is always safe.
        if !self.newly_allocated.borrow().is_empty() {
            self.phase.set(CollectionPhase::Sleep);
        }

        if self.phase.get() == CollectionPhase::Sleep {
            self.advance_phase();
        }

        while self.phase.get() != CollectionPhase::Sleep {
            dbg!(self.phase.get());
            self.advance_cycle_by(root, Pacing::MAX_PACE);
        }

        debug_assert!(matches!(self.phase.get(), CollectionPhase::Sleep { .. }));
    }
}

impl<A: Allocator + ?Sized> Context<A> {
    pub fn push_box(&self, ptr: GcBox<Erased>) {
        ptr.set_next(self.first_gray.get());
        self.first_gray.set(Some(ptr));
    }

    pub fn allocate<T: ?Sized + Collect + Pointee>(
        &self,
        meta: T::Metadata,
        layout: Layout,
    ) -> GcBox<T> {
        let Ok(layout) = GcInner::<T>::layout(layout) else {
            todo!()
        };

        let ptr = self.alloc.allocate(layout).unwrap();

        let gc = unsafe { GcBox::new(ptr.as_ptr().cast(), meta, layout) };

        self.objects.borrow_mut().push(gc.erase());

        gc
    }

    pub unsafe fn deallocate(&self, gc: GcBox<Erased>) {
        let layout = gc.layout();

        let ptr = gc.inner_ptr().cast::<u8>();

        unsafe { self.alloc.deallocate(NonNull::new_unchecked(ptr), layout) };
    }
}

impl<A> Drop for Context<A>
where
    A: Allocator + ?Sized,
{
    fn drop(&mut self) {
        let newly_allocated: &[GcBox<Erased>] = &self.newly_allocated.borrow();
        let objects: &[GcBox<Erased>] = &self.objects.borrow();

        for obj in objects.iter().chain(newly_allocated) {
            unsafe { obj.vtable().drop_in_place(*obj) };

            unsafe { alloc::alloc::dealloc(obj.inner_ptr().cast::<u8>(), obj.layout()) };
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pacing {
    pub trigger_bytes: Option<usize>,
    pub trigger_allocations: Option<usize>,
    pub mark_stride: usize,
    pub sweep_stride: usize,
}

impl Pacing {
    /// The maximum possible pace for the garbage collector to run. It will always trigger, and
    /// never stop tracing.
    const MAX_PACE: Pacing = Pacing {
        trigger_bytes: Some(0),
        trigger_allocations: Some(0),
        mark_stride: usize::MAX,
        sweep_stride: usize::MAX,
    };

    fn should_wake(&self, allocations: usize, bytes: usize) -> bool {
        self.trigger_allocations.is_some_and(|n| allocations >= n)
            || self.trigger_bytes.is_some_and(|n| bytes >= n)
    }
}

impl Default for Pacing {
    fn default() -> Self {
        Self {
            trigger_bytes: Some(4192),
            trigger_allocations: Some(64),
            mark_stride: 16,
            sweep_stride: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CollectionPhase {
    #[default]
    Sleep,
    Mark,
    Sweep {
        index: usize,
    },
}
