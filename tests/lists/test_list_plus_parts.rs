// Card def-ps (decision list-append-plus): `out = out + p` of parts of unknown kind appends an item and concatenates a
// list, as `out += p` does, keeping every part after the first
use crate::is;
use warp::parse;

const JOINED: &str = "def j(ps: any): list { out = [1]; for p in ps { out = out + p }; out }\n";
const GROWN: &str = "def j(ps: any): list { out = [1]; for p in ps { out += p }; out }\n";

#[test]
fn a_list_plus_parts_of_unknown_kind_keeps_them_all() {
	for program in [JOINED, GROWN] {
		is!(&format!("{program}j([2, [3, 4], \"x\"])"), parse("[1 2 3 4 \"x\"]"));
	}
}
