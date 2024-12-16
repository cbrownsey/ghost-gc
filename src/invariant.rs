// using the hack from https://github.com/dtolnay/ghost

use std::marker::PhantomData;

/// A phantom type which marks the given lifetime as being invariant.
pub type Invariant<'l> = PhantomInvariant<'l>;

#[doc(hidden)]
pub use PhantomInvariant::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[doc(hidden)]
pub enum PhantomInvariant<'l> {
    #[allow(private_interfaces)]
    __Phantom(PhantomData<fn(&'l ()) -> &'l ()>, Never),
    /// A phantom type which marks the given lifetime as invariant.
    #[default]
    Invariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Never {}
