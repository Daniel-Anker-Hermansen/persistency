use std::ptr::NonNull;

use crate::{
	link::{self, Link, Node as _},
	util::alloc,
	version::PartialVersion,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
	LeftChild,
	RightChild,
	LeftParent,
	RightParent,
}

impl link::LinkTag for Tag {
	fn reverse(self) -> Self {
		match self {
			Tag::LeftChild => Tag::LeftParent,
			Tag::RightChild => Tag::RightParent,
			Tag::LeftParent => Tag::LeftChild,
			Tag::RightParent => Tag::RightChild,
		}
	}
}

pub struct Node<T> {
	link_container: [Option<Link<Self, Tag>>; 4],
	value: T,
	copy: Option<NonNull<Self>>,
}

unsafe impl<T: Clone> link::Node<Tag> for Node<T> {
	fn link_container_mut(&mut self) -> &mut [Option<Link<Self, Tag>>] {
		&mut self.link_container
	}

	fn link_container(&self) -> &[Option<Link<Self, Tag>>] {
		&self.link_container
	}

	fn copy_pointer(&self) -> Option<NonNull<Self>> {
		self.copy
	}

	fn copy(&mut self) -> NonNull<Self> {
		let copy = alloc(Node {
			link_container: core::array::from_fn(|_| None),
			value: self.value.clone(),
			copy: None,
		});
		self.copy = Some(copy);
		copy
	}
}

impl<T: Ord + Clone> Node<T> {
	pub fn insert(&mut self, value: T, version: PartialVersion) {
		if value < self.value {
			match self.get(Tag::LeftChild, version) {
				Some(mut left) => unsafe { left.as_mut() }.insert(value, version),
				None => {
					self.add(
						Tag::LeftChild,
						alloc(Node {
							link_container: core::array::from_fn(|_| None),
							value,
							copy: None,
						}),
						version,
						false,
					);
				}
			}
		} else {
			match self.get(Tag::RightChild, version) {
				Some(mut right) => unsafe { right.as_mut() }.insert(value, version),
				None => {
					self.add(
						Tag::RightChild,
						alloc(Node {
							link_container: core::array::from_fn(|_| None),
							value,
							copy: None,
						}),
						version,
						false,
					);
				}
			}
		}
	}

	pub fn contains(&self, value: &T, version: PartialVersion) -> bool {
		match value.cmp(&self.value) {
			std::cmp::Ordering::Less => self
				.get(Tag::LeftChild, version)
				.map(|left| unsafe { left.as_ref() }.contains(value, version))
				.unwrap_or(false),
			std::cmp::Ordering::Equal => true,
			std::cmp::Ordering::Greater => self
				.get(Tag::RightChild, version)
				.map(|right| unsafe { right.as_ref() }.contains(value, version))
				.unwrap_or(false),
		}
	}
}
