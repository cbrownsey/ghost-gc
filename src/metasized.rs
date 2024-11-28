use std::{
    alloc::{Layout, LayoutError},
    ptr::Pointee,
};

/// # Safety
/// For all constructible values of `x`, it must hold that
/// `Layout::for_value(&x) == MetaSized::meta_layout(metadata(&x))`. For all values of metadata
/// where there exists no constructible value, it must return an error.
///
/// This function *must* be pure. Repeated calls with the same metadata must return the same value.
pub unsafe trait MetaSized: Pointee {
    fn meta_layout(meta: Self::Metadata) -> Result<Layout, LayoutError>;
}

unsafe impl<T> MetaSized for [T] {
    fn meta_layout(meta: usize) -> Result<Layout, LayoutError> {
        Layout::array::<T>(meta)
    }
}

unsafe impl MetaSized for str {
    fn meta_layout(meta: usize) -> Result<Layout, LayoutError> {
        Layout::array::<u8>(meta)
    }
}
