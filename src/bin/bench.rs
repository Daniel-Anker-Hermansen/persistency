use persistent::PersistenLinkedList;

fn main() {
	let mut list = PersistenLinkedList::new();
	list.insert(0, 0);
	#[cfg(bench)]
	println!("{}: {}", 0, persistent::ALLOC_COUNTER.with_borrow(|v| *v));
	for i in 1..1000 {
		list = list.insert((14) % i, 0).unwrap();
		#[cfg(bench)]
		println!("{}: {}", i, persistent::ALLOC_COUNTER.with_borrow(|v| *v));
	}
}
