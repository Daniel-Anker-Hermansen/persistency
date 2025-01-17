use std::{ops::Index, vec};

use crate::{cell::PersistentCell, version::Version};

/// Persistent version of Vec.
pub struct Vec<T: ?Sized> {
	vec: vec::Vec<PersistentCell<T>>,

	// We need to know the length for each version to know where to insert push and pop, and to
	// calculate the length of course.
	len: PersistentCell<usize>,
}

impl<T: ?Sized> Default for Vec<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: ?Sized> Vec<T> {
	pub fn new() -> Vec<T> {
		Vec {
			vec: vec::Vec::new(),
			len: PersistentCell::new(),
		}
	}

	pub fn push_after(&mut self, value: Box<T>, version: Version) -> Version {
		let len = self.len(version);
		if len == self.vec.len() {
			self.vec.push(PersistentCell::new());
		}
		let cell = &mut self.vec[len];
		let version = cell.insert_after(version, value);
		self.set_len_after(version, len + 1)
	}

	pub fn pop_after(&mut self, version: Version) -> Version {
		let len = self.len(version);
		self.set_len_after(version, len - 1)
	}

	pub fn view(&self, version: Version) -> VecView<'_, T> {
		VecView {
			inner: self,
			version,
		}
	}

	pub fn len(&self, version: Version) -> usize {
		// If the version is before the vector was created this will return None, so
		// therefore unwrap_or(0)
		self.len.get(version).cloned().unwrap_or(0)
	}

	fn set_len_after(&mut self, version: Version, len: usize) -> Version {
		self.len.insert_after(version, Box::new(len))
	}
}

/// A view into a specific version of a vec
pub struct VecView<'a, T: ?Sized> {
	inner: &'a Vec<T>,
	version: Version,
}

impl<T> Index<usize> for VecView<'_, T> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		let len = self.inner.len(self.version);
		if index >= len {
			panic!("Index out of bounds. Index was {} len was {}", index, len);
		} else {
			self.inner.vec[index]
				.get(self.version)
				.expect("must be initialized in this cell as the len is greater for this version")
		}
	}
}
