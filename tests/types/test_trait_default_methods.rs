// A default method in a trait, `trait shape{area; describe(s) := …}`: every type that defines the other operations gets
// it, unless it defines its own
use crate::is;

const SHAPES: &str = "trait shape{area; describe(s) := area(s) * 2}; class square{side:int}; class rect{w:int h:int}; area(s:square) := s.side*s.side; area(r:rect) := r.w*r.h; ";

#[test]
fn a_conforming_type_gets_the_default_method() {
	is!(&format!("{SHAPES}describe(square(3))"), 18);
	is!(&format!("{SHAPES}describe(square(3)) + describe(rect(2, 5))"), 38);
	is!("trait shape{area; describe(s) := \"area \" + area(s)}; class square{side:int}; area(s:square) := s.side*s.side; describe(square(3))", "area 9");
}

#[test]
fn a_type_overrides_the_default_method() {
	is!(&format!("{SHAPES}describe(s:square) := 1; describe(square(3))"), 1);
}

#[test]
fn a_type_with_the_required_operations_conforms() {
	is!(&format!("{SHAPES}square(2) is shape"), 1);
}
