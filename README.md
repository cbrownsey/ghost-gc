A garbage collected arena in which garbage collected boxes can be allocated, and
which can't escape from the arena, using invariant lifetimes.

The main type of this crate is the garbage collected pointer `Gc`. It is a thin
smart pointer branded with an invariant lifetime, to ensure that it is unable to
escape from the arena in which it was allocated.

The `Gc` is capable of holding any type which implements the `Collect` trait,
which includes most types which do not contain interior mutability.
