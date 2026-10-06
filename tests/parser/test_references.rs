// wiki/reference.md: wasp data refers back to an enclosing node by name (`$a`) or by its id (`a[id=1]{…}`, `$1`), so a
// literal describes a cyclic graph; the reference stays a name, so printing never loops
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn bracketed_attributes_before_a_body_are_meta_entries() {
	assert_eq!(parse("a[id=1]{ b }"), parse("a{@id:1 b}"));
	assert_eq!(parse("a[id=1, kind=\"x\"]{ b }"), parse("a{@id:1 @kind:\"x\" b}"));
	assert_eq!(parse("xs[1]"), parse("xs#2")); // an index stays one (0-based)
}

#[test]
fn references_survive_a_round_trip() {
	for source in ["a{b c{parent=$a}}", "a{@id:1 b c{parent=$1}}"] {
		let printed = parse(source).serialize();
		assert_eq!(parse(&printed), parse(source), "{source} printed as {printed}");
	}
}

#[test]
fn a_path_follows_a_reference() {
	is!("x = a{ b:2 c:{ parent:$a } }; x.c.parent.b", 2);
	is!("x = a{ b:2 c{ parent=$a } }; x.c.parent.c.parent.b", 2);
	is!("x = a[id=1]{ b:2 c{ parent=$1 } }; x.c.parent.b", 2);
	is!("x = a{ b:2 c:{ d:{ up:$c e:5 } } }; x.c.d.up.d.e", 5);
}

#[test]
fn a_dollar_digit_without_an_id_stays_a_parameter() {
	is!("f = {$0 * 2}; f(3)", 6);
}
