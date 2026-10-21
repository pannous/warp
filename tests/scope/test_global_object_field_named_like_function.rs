//! card pure-kinds: a main-level object's field named like a program function (emitter.warp's `module.kinds` beside
//! analyzer.warp's `kinds(program)`) reads the field, though another function has a local of the object's name
use crate::is;

#[test]
fn a_field_named_like_a_function_reads_the_field_beside_a_same_named_local() {
	is!(
		"g(s) := { module = 3; module }\nkinds(p: any): any := 5\nclass M { functions: any; kinds: any = {} }\nmodule = M({}, {value: 4})\nget_value(): any := module.kinds[\"value\"]\nget_value()",
		4
	);
}
