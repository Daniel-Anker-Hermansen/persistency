use std::ptr::NonNull;

trait BidirectionalPointerTarget<A>: Sized {
	fn add<const N: usize>(&mut self, pointer: NonNull<BidirectionalPointerForward<A, Self, N>>);
}

/// Forward part of a bidirectional pointer from A to B with N slots
struct BidirectionalPointerForward<A, B, const N: usize> {
	container: NonNull<A>,
	inner: [Option<BidirectionalPointerForwardInner<B>>; N],
}

struct BidirectionalPointerForwardInner<B> {
	pointer: NonNull<B>,
	version: usize,
}

// We cannot derive Clone + Clopy as that would put a needless Clone + Copy bound on B
impl<B> Clone for BidirectionalPointerForwardInner<B> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<B> Copy for BidirectionalPointerForwardInner<B> {}

impl<A, B, const N: usize> BidirectionalPointerForward<A, B, N> {
	fn new(container: NonNull<A>) -> BidirectionalPointerForward<A, B, N> {
		BidirectionalPointerForward {
			container,
			inner: [None; N],
		}
	}

	fn add(&mut self, mut pointer: NonNull<B>, version: usize) -> bool
	where
		B: BidirectionalPointerTarget<A>,
	{
		match self.inner.iter_mut().find(|pointer| pointer.is_none()) {
			Some(entry) => {
				*entry = Some(BidirectionalPointerForwardInner { pointer, version });
				unsafe { pointer.as_mut() }.add(NonNull::from(self));
				true
			},
			None => false,
		}
	}
}

/// Backward part of a bidrectional pointer from A to B with N slots
struct BidirectionalPointerBackward<A, B, const N: usize, const M: usize> {
	pointers: [Option<NonNull<BidirectionalPointerForward<A, B, N>>>; M],
}

impl<A, B, const N: usize, const M: usize> BidirectionalPointerBackward<A, B, N, M> {
	fn new() -> BidirectionalPointerBackward<A, B, N, M> {
		BidirectionalPointerBackward {
			pointers: [None; M],
		}
	}

	/// Returns true if the pointer was successfully added, otherwise returns false if there is
	/// no capacity left
	fn add(&mut self, pointer: NonNull<BidirectionalPointerForward<A, B, N>>) -> bool {
		match self.pointers.iter_mut().find(|pointer| pointer.is_none()) {
			Some(entry) => {
				*entry = Some(pointer);
				true
			}
			None => false,
		}
	}
}
