use core::fmt;
use core::ptr::NonNull;

use crate::util::alloc;

struct VersionList {
	size: usize,
	base: NonNull<VersionSuperNode>,
}

struct VersionSuperNode {
	parent: NonNull<VersionList>,
	next: NonNull<VersionSuperNode>,
	size: usize,
	value: u64,
	list: NonNull<VersionNode>,
}

struct VersionNode {
	parent: NonNull<VersionSuperNode>,
	next: Option<NonNull<VersionNode>>,
	value: u64,
}

unsafe fn node_parent(this: NonNull<VersionNode>) -> NonNull<VersionSuperNode> {
	unsafe { this.as_ref().parent }
}

unsafe fn node_next(this: NonNull<VersionNode>) -> Option<NonNull<VersionNode>> {
	unsafe { this.as_ref().next }
}

unsafe fn node_value(this: NonNull<VersionNode>) -> u64 {
	unsafe { this.as_ref().value }
}

unsafe fn super_node_parent(this: NonNull<VersionSuperNode>) -> NonNull<VersionList> {
	unsafe { this.as_ref().parent }
}

unsafe fn super_node_list(this: NonNull<VersionSuperNode>) -> NonNull<VersionNode> {
	unsafe { this.as_ref().list }
}

unsafe fn super_node_next(this: NonNull<VersionSuperNode>) -> NonNull<VersionSuperNode> {
	unsafe { this.as_ref().next }
}

unsafe fn super_node_size(this: NonNull<VersionSuperNode>) -> usize {
	unsafe { this.as_ref().size }
}

unsafe fn super_node_value(this: NonNull<VersionSuperNode>) -> u64 {
	unsafe { this.as_ref().value }
}

unsafe fn is_base(this: NonNull<VersionSuperNode>) -> bool {
	unsafe {
		let list = super_node_parent(this);
		let base = list_base(list);
		this == base
	}
}

unsafe fn list_base(this: NonNull<VersionList>) -> NonNull<VersionSuperNode> {
	unsafe { this.as_ref().base }
}

unsafe fn split_super(mut this: NonNull<VersionSuperNode>) {
	unsafe {
		let next = super_node_next(this);
		let this_value = super_node_value(this);
		let next_value = super_node_value(next);
		let value = this_value.wrapping_add(
			next_value
				.wrapping_sub(1)
				.wrapping_sub(this_value)
				.div_ceil(2),
		);
		let parent = super_node_parent(this);
		let mut new_node = alloc(VersionSuperNode {
			parent,
			next,
			size: 32,
			value,
			list: NonNull::dangling(),
		});
		this.as_mut().next = new_node;
		this.as_mut().size = 32;
		let list = super_node_list(this);
		if value == this_value {
			renumber(this);
		}
		new_node.as_mut().list = split(list, 0, new_node);
	}
}

unsafe fn renumber(this: NonNull<VersionSuperNode>) {
	unsafe {
		let mut j = 1;
		let this_value = super_node_value(this);
		let mut next = super_node_next(this);
		let mut current_value = super_node_value(next);
		while current_value.wrapping_sub(this_value) < j * j {
			next = super_node_next(next);
			current_value = super_node_value(next);
			j += 1;
		}
		dbg!(j);
		let interval = current_value.wrapping_sub(this_value) / j;
		let mut current = this;
		for i in 0..j {
			current.as_mut().value = this_value.wrapping_add(interval * i);
			current = super_node_next(current);
		}
	}
}

unsafe fn split(
	mut this: NonNull<VersionNode>,
	index: u64,
	new_parent: NonNull<VersionSuperNode>,
) -> NonNull<VersionNode> {
	const VALUE: u64 = 1 << 32;
	unsafe {
		this.as_mut().value = VALUE * index;
		let next = node_next(this).expect("the length of the linked list to be 64");

		if index == 31 {
			split_tail(next, 0, new_parent);
			this.as_mut().next = None;
			next
		} else {
			split(next, index + 1, new_parent)
		}
	}
}

unsafe fn split_tail(
	mut this: NonNull<VersionNode>,
	index: u64,
	new_parent: NonNull<VersionSuperNode>,
) {
	const VALUE: u64 = 1 << 32;
	unsafe {
		this.as_mut().value = VALUE * index;
		this.as_mut().parent = new_parent;

		if index < 31 {
			let next = node_next(this).expect("the length of the linked list to be 64");
			split_tail(next, index + 1, new_parent);
		}
	}
}

/// Represents a version in a version list. Can be compared with other versions. Comparing with
/// versions from other version lists is meaningless. The type uses pointers internally with
/// interior mutability therefore the debug print output can change when new versions are added to
/// the list.
#[derive(Clone, Copy)]
pub struct Version {
	pub primary: PartialVersion,
	pub secondary: PartialVersion,
}

impl Default for Version {
	fn default() -> Self {
		Self::new()
	}
}

impl Version {
	pub fn new() -> Version {
		let primary = PartialVersion::new();
		let secondary = primary.insert_after();
		Version { primary, secondary }
	}

	pub fn insert_after(self) -> Version {
		let primary = self.primary.insert_after();
		let secondary = primary.insert_after();
		Version { primary, secondary }
	}
}

impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.primary.eq(&other.primary)
	}
}

impl Eq for Version {}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.primary.cmp(&other.primary)
	}
}

#[derive(Clone, Copy)]
pub struct PartialVersion {
	node: NonNull<VersionNode>,
}

impl Default for PartialVersion {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialVersion {
	/// Creates a new version and the associatied version list. Comparing this with version
	/// from other version lists is meaningless.
	pub fn new() -> PartialVersion {
		let mut node = alloc(VersionNode {
			parent: NonNull::dangling(),
			next: None,
			value: 0,
		});
		let mut super_node = alloc(VersionSuperNode {
			parent: NonNull::dangling(),
			next: NonNull::dangling(),
			size: 1,
			value: 0,
			list: node,
		});
		let list = alloc(VersionList {
			size: 1,
			base: super_node,
		});

		// SAFETY: No other references exist while we use the references
		unsafe { node.as_mut() }.parent = super_node;
		unsafe { super_node.as_mut() }.parent = list;
		unsafe { super_node.as_mut() }.next = super_node;

		PartialVersion { node }
	}

	/// Inserts a new version directly after this version and returns it.
	fn insert_after(mut self) -> PartialVersion {
		unsafe {
			let next = node_next(self.node);
			let prev_value = node_value(self.node);
			let next_value = next.map(|next| node_value(next)).unwrap_or(u64::MAX);
			// It does matter if we use div_ceil or div_floor in the general case.
			// however we can approximate the out of bounds value as u64::MAX with
			// div_ceil and still get the correct value, which means that we can have
			// list of size 64 instead of 63.
			let value = prev_value + (next_value - prev_value).div_ceil(2);
			let mut parent = node_parent(self.node);
			let new_version = alloc(VersionNode {
				parent,
				next,
				value,
			});
			self.node.as_mut().next = Some(new_version);

			parent.as_mut().size += 1;
			if super_node_size(parent) == 64 {
				split_super(parent);
			}

			let mut list = super_node_parent(parent);
			list.as_mut().size += 1;

			PartialVersion { node: new_version }
		}
	}

	fn ordering_values(self) -> (u64, u64) {
		unsafe {
			let minor = node_value(self.node);
			let parent = node_parent(self.node);
			let list = super_node_parent(parent);
			let base = list_base(list);
			let major = super_node_value(parent).wrapping_sub(super_node_value(base));
			(major, minor)
		}
	}
}

impl fmt::Debug for PartialVersion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let (major, minor) = self.ordering_values();

		f.debug_struct("Version")
			.field("major", &major)
			.field("minor", &minor)
			.finish()
	}
}

impl PartialEq for PartialVersion {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other).is_eq()
	}
}

impl Eq for PartialVersion {}

impl PartialOrd for PartialVersion {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for PartialVersion {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.ordering_values().cmp(&other.ordering_values())
	}
}

#[cfg(test)]
mod test {
	use super::PartialVersion;

	#[test]
	fn version_test() {
		let mut version_list = vec![PartialVersion::new()];
		for _ in 0..10000 {
			let i = fastrand::usize(..version_list.len());
			let new_version = version_list[i].insert_after();
			version_list.insert(i + 1, new_version);
		}
		for k in 0..version_list.len() {
			assert_eq!(version_list[k], version_list[k]);
			let i = fastrand::usize(..version_list.len() - 1);
			let j = fastrand::usize(i + 1..version_list.len());
			assert!(version_list[i] < version_list[j]);
			assert!(version_list[j] > version_list[i]);
		}
	}

	#[test]
	fn adversarial() {
		let mut version_list = vec![];
		let version = PartialVersion::new();
		for _ in 0..100000 {
			version_list.push(version.insert_after());
		}
		version_list.reverse();
		for k in 0..version_list.len() {
			assert_eq!(version_list[k], version_list[k]);
			let i = fastrand::usize(..version_list.len() - 1);
			let j = fastrand::usize(i + 1..version_list.len());
			assert!(version_list[i] < version_list[j]);
			assert!(version_list[j] > version_list[i]);
		}
	}
}
