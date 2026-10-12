//! Card w0-list-writes: `===` of lists asks whether two names hold one list; a list of numbers kept as a typed array
//! was boxed into a fresh Node on each side, so `xs === xs` was no
use crate::is;

#[test]
fn a_list_is_identical_to_itself_in_every_name() {
	is!("xs = [1]; xs === xs", true);
	is!("xs = [1]; ys = xs; ys === xs", true);
	is!("xs = [1.5]; ys = xs; xs !== ys", false);
	is!("xs = [1]; ys = [1]; ys === xs", false);
	is!("xs = [1, \"a\"]; ys = xs; xs === ys", true);
}
