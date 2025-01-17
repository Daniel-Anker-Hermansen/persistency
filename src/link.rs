use core::ptr::NonNull;

use crate::version::PartialVersion;

pub struct Link<Node, Tag>
where
	Node: ?Sized,
{
	tag: Tag,
	version: PartialVersion,
	node_pointer: NonNull<Node>,
	link_pointer: NonNull<Link<Node, Tag>>,
}

/// The trait is marked unsafe since implementation of the copy function must return a
/// dereferenciable pointer.
pub unsafe trait Node<Tag: PartialEq + Eq + Clone + LinkTag> {
	fn link_container_mut(&mut self) -> &mut [Option<Link<Self, Tag>>];

	fn link_container(&self) -> &[Option<Link<Self, Tag>>];

	fn copy(&mut self) -> NonNull<Self>;

	fn copy_pointer(&self) -> Option<NonNull<Self>>;

	fn current_version(&mut self, _version: PartialVersion) -> &mut Self {
		self.copy_pointer().map(|mut pointer| unsafe { pointer.as_mut() }).unwrap_or(self)
	}

	fn copy_and_prepare(&mut self, version: PartialVersion) -> NonNull<Self> {
		let mut copy = self.copy();
		let container = unsafe { copy.as_mut() }.link_container_mut();
		let mut to_move = Vec::new();
		for i in 0..container.len() {
			if let Some(current) = &container[i] {
				if container
					.iter()
					.filter_map(Option::as_ref)
					.all(|link| link.tag != current.tag || link.version <= current.version)
				{
					to_move.push(i);
				}
			}
		}
		for i in to_move {
			let Some(link) = &mut container[i] else {
				unreachable!()
			};
			if link.version == version {
				let free = unsafe { copy.as_mut() }.link_container_mut()
					.iter_mut().find(|link| link.is_none())
					.expect("It has just been cloned. This means that the capacity is less than the tag size");
				*free = Some(Link {
					tag: link.tag.clone(),
					version,
					node_pointer: link.node_pointer,
					link_pointer: link.link_pointer,
				});
				unsafe { link.link_pointer.as_mut() }.node_pointer = copy;
				unsafe { link.link_pointer.as_mut() }.link_pointer =
					NonNull::from(free.as_mut().expect("was just intialized to Some"));
				container[i] = None;
			} else {
				unsafe { copy.as_mut() }.add(link.tag.clone(), link.node_pointer, version, false);
			}
		}
		copy
	}

	fn add(
		&mut self,
		tag: Tag,
		mut pointer: NonNull<Self>,
		version: PartialVersion,
		reverse: bool,
	) -> (NonNull<Self>, NonNull<Link<Self, Tag>>) {
		if let Some(free) = self
			.link_container_mut()
			.iter_mut()
			.find(|link| link.is_none())
		{
			*free = Some(Link {
				tag: tag.clone(),
				version,
				node_pointer: pointer,
				link_pointer: NonNull::dangling(),
			});
			let mut link_non_null =
				NonNull::from(free.as_mut().expect("was just initialized to Some"));

			if !reverse {
				let (pointer, mut link_pointer) = unsafe { pointer.as_mut() }.add(
					tag.reverse(),
					unsafe { NonNull::new_unchecked(self as *mut _) },
					version,
					false,
				);
				unsafe { link_non_null.as_mut() }.node_pointer = pointer;
				unsafe { link_non_null.as_mut() }.link_pointer = link_pointer;
				unsafe { link_pointer.as_mut() }.link_pointer = link_non_null;
			}

			let self_non_null = NonNull::from(self);
			(self_non_null, link_non_null)
		} else {
			let mut copy = self.copy_and_prepare(version);
			unsafe { copy.as_mut() }.add(tag, pointer, version, reverse)
		}
	}

	fn get(&self, tag: Tag, version: PartialVersion) -> Option<NonNull<Self>> {
		self.link_container()
			.iter()
			.filter_map(Option::as_ref)
			.filter(|link| link.tag == tag && link.version <= version)
			.max_by_key(|link| link.version)
			.map(|link| link.node_pointer)
	}
}

pub trait LinkTag {
	fn reverse(self) -> Self;
}
