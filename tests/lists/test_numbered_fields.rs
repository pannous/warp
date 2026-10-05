// `pair.0`, `pair.1`: the items of a tuple or list by number, counted from 0 like `pair[0]`
use warp::is;

#[test]
fn numbered_fields_of_a_tuple() {
	is!("h=(1,2); h.0", 1);
	is!("h=(1,2); h.1", 2);
	is!("h=[5,6]; h.1", 6);
}
