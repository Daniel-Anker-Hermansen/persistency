use core::ptr::NonNull;

/// Allocate t in the heap and return a pointer to it.
pub fn alloc<T>(t: T) -> NonNull<T> {
	// SAFETY: The pointer is valid as it comes from a box
	unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(t))) }
}
