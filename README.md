A garbage collected arena in which garbage collected boxes can be allocated, and
which can't escape from the arena, using invariant lifetimes.

# [`Gc`]

The main type of this crate. A garbage collected pointer which is branded with
an invariant lifetime. This is a thin pointer, any associated metadata is stored
inline with the data.

The `Gc` is capable of holding any type which implements the trait [`Collect`],
which includes most types which do not contain interior mutability.
