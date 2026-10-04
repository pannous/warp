// size is the element count; byte_size gives the bytes, as x.bytes does (user, P40)
use warp::is;

#[test]
fn byte_size_counts_bytes_and_size_counts_elements() {
	is!("xs=[1,2,3]; size(xs)", 3);
	is!("xs=[1,2,3]; byte_size(xs)", 24);
	is!("xs=[1,2,3]; xs.byte_size", 24);
	is!("byte_size(\"héllo\")", 6);
}
