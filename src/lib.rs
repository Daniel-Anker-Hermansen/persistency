mod fully;
pub mod version;
pub mod link;
pub mod binary_tree;
pub mod cell;
pub mod vec;
pub(crate) mod util;

use std::{num::NonZero, ptr::NonNull, rc::Rc};

pub struct PersistenLinkedList<T> {
	value: Option<NonNull<PersistentLinkedListInner<T>>>,
	version: usize,
}

struct PersistentLinkedListInner<T> {
	value: Rc<T>,
	next: PersistentLinkedListPointer<T>,
	prev: PersistentLinkedListPointer<T>,
	copy: Option<NonNull<PersistentLinkedListInner<T>>>,
}

struct PersistentLinkedListPointer<T> {
	original_version: usize,
	original: Option<NonNull<PersistentLinkedListInner<T>>>,
	new_version: Option<NonZero<usize>>,
	new: Option<NonNull<PersistentLinkedListInner<T>>>,
}

impl<T> PersistenLinkedList<T> {
	pub fn new() -> PersistenLinkedList<T> {
		PersistenLinkedList {
			value: None,
			version: 0,
		}
	}

	pub fn get(&self, index: usize) -> Option<&T> {
		get_on_opt(self.value, index, self.version).map(|ptr| unsafe { &*ptr })
	}

	pub fn insert(&self, index: usize, value: T) -> Option<PersistenLinkedList<T>> {
		match self.value {
			Some(_) => insert_on_opt(self.value, index, value, self.version + 1).map(|ptr| {
				PersistenLinkedList {
					value: Some(ptr),
					version: self.version + 1,
				}
			}),
			None => (index == 0).then(|| {
				let inner = PersistentLinkedListInner::alloc(Rc::new(value), self.version + 1);
				PersistenLinkedList {
					value: Some(inner),
					version: self.version + 1,
				}
			}),
		}
	}

	pub fn crawl_debug(&self) {
		crawl_debug(self.value, self.version);
	}
}

fn crawl_debug<T>(opt: Option<NonNull<PersistentLinkedListInner<T>>>, version: usize) {
	if let Some(ptr) = opt {
		let ptr = unsafe { ptr.as_ref() };
		eprintln!("Node {:?} {{", ptr as *const _);
		eprintln!("\tprev: {:?}", ptr.prev.get(version).map(|p| unsafe { p.as_ref() } as *const _).unwrap_or(std::ptr::null()));
		eprintln!("\tnext: {:?}", ptr.next.get(version).map(|p| unsafe { p.as_ref() } as *const _).unwrap_or(std::ptr::null()));
		eprintln!("}}");
		crawl_debug(ptr.next.get(version), version);
	}
}

fn get_on_opt<T>(
	opt: Option<NonNull<PersistentLinkedListInner<T>>>,
	index: usize,
	version: usize,
) -> Option<*const T> {
	let ptr = opt?;
	let val = unsafe { ptr.as_ref() };
	if index == 0 {
		Some(&val.value as &T as *const T)
	} else {
		get_on_opt(val.next.get(version), index - 1, version)
	}
}

fn insert_on_opt<T>(
	opt: Option<NonNull<PersistentLinkedListInner<T>>>,
	index: usize,
	value: T,
	version: usize,
) -> Option<NonNull<PersistentLinkedListInner<T>>> {
	let ptr = unsafe { opt?.as_mut() };
	if index == 0 {
		let mut new_node = PersistentLinkedListInner::alloc(Rc::new(value), version);
		let new_node_ptr = unsafe { new_node.as_mut() };
		new_node_ptr.set_ptr(version, opt, |l| &mut l.next);
		new_node_ptr.set_ptr(version, ptr.prev.get(version), |l| &mut l.prev);
		new_node_ptr.cascade_ptrs(version);
		Some(new_node)
	} else {
		let next = ptr.next.get(version - 1);
		if next.is_none() && index == 1 {
			let mut new_node = PersistentLinkedListInner::alloc(Rc::new(value), version);
			let new_node_ptr = unsafe { new_node.as_mut() };
			new_node_ptr.set_ptr(version, opt, |l| &mut l.prev);
			new_node_ptr.cascade_ptrs(version);
		} else {
			insert_on_opt(next, index - 1, value, version)?;
		}
		Some(get_new_version(opt?))
	}
}

fn get_new_version<T>(
	opt: NonNull<PersistentLinkedListInner<T>>,
) -> NonNull<PersistentLinkedListInner<T>> {
	unsafe { opt.as_ref() }.copy.unwrap_or(opt)
}

#[cfg(bench)]
thread_local! {
	pub static ALLOC_COUNTER: RefCell<usize> = RefCell::new(0);
}

impl<T> PersistentLinkedListInner<T> {
	fn alloc(value: Rc<T>, version: usize) -> NonNull<PersistentLinkedListInner<T>> {
		let ret = PersistentLinkedListInner {
			value,
			next: PersistentLinkedListPointer::new(version),
			prev: PersistentLinkedListPointer::new(version),
			copy: None,
		};
		let b = Box::new(ret);
		#[cfg(bench)]
		ALLOC_COUNTER.with_borrow_mut(|v| *v += 1);
		NonNull::from(Box::leak(b))
	}

	fn copy(&mut self, value: Rc<T>, version: usize) -> &mut PersistentLinkedListInner<T> {
		let mut copy = PersistentLinkedListInner::alloc(value, version);
		let ptr = unsafe { copy.as_mut() };
		assert!(!ptr.next.update(version, self.next.get(version)));
		assert!(!ptr.prev.update(version, self.prev.get(version)));
		self.copy = Some(copy);
		ptr
	}

	fn set_ptr(
		&mut self,
		version: usize,
		ptr: Option<NonNull<PersistentLinkedListInner<T>>>,
		which: fn(&mut PersistentLinkedListInner<T>) -> &mut PersistentLinkedListPointer<T>,
	) -> Option<&mut PersistentLinkedListInner<T>> {
		if which(self).get(version) == ptr {
			None
		} else if which(self).update(version, ptr) {
			let copy = self.copy(self.value.clone(), version);
			assert!(!which(copy).update(version, ptr));
			Some(copy)
		} else {
			assert_eq!(ptr, which(self).get(version));
			Some(self)
		}
	}

	fn cascade_ptrs(&self, version: usize) {
		if let Some(next) = self.next.get(version) {
			let next = unsafe { get_new_version(next).as_mut() };
			if let Some(next) = next.set_ptr(version, Some(NonNull::from(self)), |l| &mut l.prev) {
				next.cascade_ptrs(version);
			}
		}
		if let Some(prev) = self.prev.get(version) {
			let prev = unsafe { get_new_version(prev).as_mut() };
			if let Some(prev) = prev.set_ptr(version, Some(NonNull::from(self)), |l| &mut l.next) {
				prev.cascade_ptrs(version);
			}
		}
	}
}

impl<T> PersistentLinkedListPointer<T> {
	fn new(version: usize) -> PersistentLinkedListPointer<T> {
		PersistentLinkedListPointer {
			original_version: version,
			original: None,
			new_version: None,
			new: None,
		}
	}

	fn get(&self, version: usize) -> Option<NonNull<PersistentLinkedListInner<T>>> {
		assert!(version >= self.original_version);
		match self.new_version {
			Some(v) if v.get() <= version => self.new,
			_ => self.original,
		}
	}

	/// Returns true if a copy is required
	fn update(
		&mut self,
		version: usize,
		ptr: Option<NonNull<PersistentLinkedListInner<T>>>,
	) -> bool {
		match self.new_version {
			Some(v) => {
				if v.get() == version {
					self.new = ptr;
					false
				} else {
					assert!(v.get() < version);
					true
				}
			}
			None => {
				if self.original_version == version {
					self.original = ptr;
				} else {
					assert!(self.original_version < version);
					assert!(version > 0);
					self.new_version = NonZero::new(version);
					self.new = ptr;
				}
				false
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::PersistenLinkedList;

	#[test]
	fn no_persistence_insert_begin() {
		let mut list = PersistenLinkedList::new();
		for i in 0..5 {
			list = list.insert(0, i).unwrap();
		}
		list.crawl_debug();
		for i in 0..5 {
			assert_eq!(list.get(i), Some(&(4 - i)));
		}
	}
	
	#[test]
	fn no_persistence_insert_end() {
		let mut list = PersistenLinkedList::new();
		for i in 0..5 {
			list = list.insert(i, i).unwrap();
		}
		list.crawl_debug();
		for i in 0..5 {
			assert_eq!(list.get(i), Some(&i));
		}
	}
	
	#[test]
	fn no_persistence_insert_middle() {
		let mut list = PersistenLinkedList::new().insert(0, 10).unwrap();
		for i in 0..5 {
			list = list.insert(1, i).unwrap();
		}
		list.crawl_debug();
		assert_eq!(list.get(0), Some(&10));
		for i in 0..5 {
			assert_eq!(list.get(i + 1), Some(&(4 - i)));
		}
	}

	#[test]
	fn persistence_insert_begin() {
		let mut lists = vec![PersistenLinkedList::new()];
		for i in 0..5 {
			lists.push(lists.last().unwrap().insert(0, i).unwrap());
		}
		for (len, list) in lists.into_iter().enumerate() {
			list.crawl_debug();
			for i in 0..len {
				assert_eq!(list.get(i), Some(&(len - i - 1)));
			}
		}
	}
}
