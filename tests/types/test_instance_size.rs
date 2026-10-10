//! card size-instance: `size(b)` of an instance whose class defines size() calls it, as count, len and length do
use crate::is;

const BAG: &str = "class Bag{items:[int]; size() := count(items)}; b = Bag([1 2 3]); ";

#[test]
fn the_free_size_of_an_instance_calls_its_size_method() {
	is!(&format!("{BAG}size(b)"), 3);
	is!(&format!("{BAG}size(b) + count(b) + len(b) + length(b) + b.size()"), 15);
}
