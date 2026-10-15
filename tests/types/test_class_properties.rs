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

/// `p.f = v` of a function f(p, v) read as `p.f`: `p = f(p, v)`, not a definition of f(p) (decision shared-buses)
#[test]
fn a_function_member_assigned_stores_its_call_back() {
	is!("class P {x: int}; f(p: P, v) := P(v * 2); p = P(1); p.f = 5; p.x", 10);
	is!("class B {x: int}; r(b: B, m) := B(m); r(xs: list, m) := xs; b = B(1); b.r = 5; b.x", 5);
	is!("class P {x: int}; f(p: P, v) := P(v); p = P(1); p.f = 5; f(p, 3).x", 3);
}
