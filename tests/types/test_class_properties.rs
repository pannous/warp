// Properties of a class (wiki/property.md): a getter computes a value read like a field, a setter runs when the
// property is assigned, `p.age = 30`
use crate::is;

const JS_STYLE: &str = "class P{birthday:int; get age() { 2026 - birthday }; set age(v) { birthday = 2026 - v }}; ";
const WARP_STYLE: &str = "class P{birthday:int; age:{2026 - birthday} set{birthday = 2026 - it}}; ";

#[test]
fn a_getter_reads_like_a_field() {
	is!(&format!("{JS_STYLE}P(2000).age"), 26);
	is!(&format!("{WARP_STYLE}P(2000).age"), 26);
}

#[test]
fn a_setter_runs_when_the_property_is_assigned() {
	is!(&format!("{JS_STYLE}p = P(2000); p.age = 30; p.birthday"), 1996);
	is!(&format!("{WARP_STYLE}p = P(2000); p.age = 30; p.birthday"), 1996);
	is!(&format!("{JS_STYLE}p = P(2000); p.age = 30; p.age"), 30);
}
