use std::{collections::BTreeMap, ptr::NonNull};

use crate::version::Version;

enum OwnedOrPointer<T: ?Sized> {
	Owned(Box<T>),
	Pointer(Option<NonNull<T>>),
}

/// Fully persistent memory cell. Note that all versions passed to functions invoked on a cell must
/// come from the same version tree. A new version can be created with `Version::new`, and then
/// relative version can be created with `Version::insert_after` or with functions defined on
/// various persistent data structures i.e `PersistentCell::insert_after`. Note that the same
/// version tree may be used in multiple data structures. All operations run in amortized O(log m)
/// time where m is the number of version in the cell.
// TODO: Should this type be ?Sized. Is the box necessary. Is it better to just use a version as a
// reference instead of a direct pointer? That would cause up to two searches per access instead of
// one doubling the running time in the worst case. Making this type not ?Sized would cascade to
// `Vec`. 
pub struct PersistentCell<T: ?Sized> {
	tree: BTreeMap<Version, OwnedOrPointer<T>>,
}

impl<T: ?Sized> PersistentCell<T> {
	pub fn new() -> PersistentCell<T> {
		PersistentCell {
			tree: BTreeMap::new(),
		}
	}

	/// Gets the value in this version. This is the last inserted value in an ancestor of this
	/// version. Returns None if this version is from before the first version of the tree.
	pub fn get(&self, version: Version) -> Option<&T> {
		match self.tree.range(..=version).last()?.1 {
			OwnedOrPointer::Owned(v) => Some(v),
			// SAFETY: the pointer points to a value in the tree as it is constructed
			// in `get_actual`. Values are never removed from the tree and the values
			// are stored in a box so this pointer is always valid.
			OwnedOrPointer::Pointer(v) => unsafe { v.map(|ptr| ptr.as_ref()) },
		}
	}

	/// Gets a mutable reference to the value for this version. Returns None if there is no
	/// value for this exact version. If you want a mutable reference to the first ancestor use
	/// `get_mut_ancestor` instead. Note that mutating this element mutates it also for
	/// versions in the future.
	pub fn get_mut(&mut self, version: Version) -> Option<&mut T> {
		match self.tree.range_mut(..=version).last()?.1 {
			OwnedOrPointer::Owned(v) => Some(v),
			_ => None,
		}
	}

	/// Inserts a new value in a new version after the given version.
	pub fn insert_after(&mut self, version: Version, value: Box<T>) -> Version {
		let v1 = version.insert_after();
		let v2 = v1.insert_after();
		self.tree.insert(v1, OwnedOrPointer::Owned(value));
		self.tree.insert(v2, OwnedOrPointer::Pointer(self.get_pointer(version)));
		v1
	}

	/// Get the version identifier of the last version. Really the dual should just have a
	/// pointer to the value but that is unsafe without Rc which is needlessly slow.	
	fn get_pointer(&self, version: Version) -> Option<NonNull<T>> {
		match self.tree.range(..=version).last() {
			Some((_, OwnedOrPointer::Owned(v))) => Some(NonNull::from(v as &T)),
			Some((_, OwnedOrPointer::Pointer(v))) => *v,
			None => None,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::version::Version;

	use super::PersistentCell;

	#[test]
	fn partial_persistent_test() {
		let mut vec = Vec::new();
		let mut cell = PersistentCell::new();
		let mut version = Version::new();
		for _ in 0..10 {
			let value = fastrand::u64(..);
			version = cell.insert_after(version, Box::new(value));
			vec.push((version, value));
		}
		for (version, value) in vec {
			assert_eq!(cell.get(version), Some(&value));
		}
	}

	#[test]
	fn double_test() {
		let mut vec = Vec::new();
		let mut cell1 = PersistentCell::new();
		let mut cell2 = PersistentCell::new();
		let mut version = Version::new();
		vec.push((version, None, None));
		for _ in 0..20 {
			if fastrand::bool() {
				let value = fastrand::u64(..);
				version = cell1.insert_after(version, Box::new(value));
				let (_, _, b) = vec.last().unwrap();
				vec.push((version, Some(value), *b));
			} else {
				let value = fastrand::u64(..);
				version = cell2.insert_after(version, Box::new(value));
				let (_, a, _) = vec.last().unwrap();
				vec.push((version, *a, Some(value)));
			}
		}
		for (version, value1, value2) in vec {
			assert_eq!(cell1.get(version), value1.as_ref());
			assert_eq!(cell2.get(version), value2.as_ref());
		}
	}

	fn branch(
		mut version: Version,
		cell1: &mut PersistentCell<u64>,
		cell2: &mut PersistentCell<u64>,
		value1: Option<u64>,
		value2: Option<u64>,
	) {
		let mut vec = Vec::new();
		vec.push((version, value1, value2));
		for _ in 0..10 {
			if fastrand::bool() {
				let value = fastrand::u64(..);
				version = cell1.insert_after(version, Box::new(value));
				let (_, _, b) = vec.last().unwrap();
				vec.push((version, Some(value), *b));
			} else {
				let value = fastrand::u64(..);
				version = cell2.insert_after(version, Box::new(value));
				let (_, a, _) = vec.last().unwrap();
				vec.push((version, *a, Some(value)));
			}
		}
		for &(version, value1, value2) in &vec {
			assert_eq!(cell1.get(version), value1.as_ref());
			assert_eq!(cell2.get(version), value2.as_ref());
		}
	}

	#[test]
	fn full_persistent_test() {
		let mut vec = Vec::new();
		let mut cell1 = PersistentCell::new();
		let mut cell2 = PersistentCell::new();
		let mut version = Version::new();
		vec.push((version, None, None));
		for _ in 0..20 {
			if fastrand::bool() {
				let value = fastrand::u64(..);
				version = cell1.insert_after(version, Box::new(value));
				let (_, _, b) = vec.last().unwrap();
				vec.push((version, Some(value), *b));
			} else {
				let value = fastrand::u64(..);
				version = cell2.insert_after(version, Box::new(value));
				let (_, a, _) = vec.last().unwrap();
				vec.push((version, *a, Some(value)));
			}
		}
		for &(version, value1, value2) in &vec {
			assert_eq!(cell1.get(version), value1.as_ref());
			assert_eq!(cell2.get(version), value2.as_ref());
		}
		for &(version, value1, value2) in &vec {
			branch(version, &mut cell1, &mut cell2, value1, value2);
		}
		for &(version, value1, value2) in &vec {
			assert_eq!(cell1.get(version), value1.as_ref());
			assert_eq!(cell2.get(version), value2.as_ref());
		}
	}
}
