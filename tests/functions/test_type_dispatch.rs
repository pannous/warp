//! Multiple dispatch by argument type (wiki/polymorphism.md, notes/wiki_features.md row 26): the variant a call takes
//! is chosen by the types of its arguments, for every spelling of a typed definition. Return-type dispatch: notes/dispatch.md.
use crate::common::fails_with;
use warp::is;

const FOO: &str = "foo of int = it+it; foo of float = it/2.0; ";

#[test]
fn test_type_name_variants_dispatch_by_argument() {
	is!(&format!("{FOO}foo 3"), 6);
	is!(&format!("{FOO}foo 3.0"), 1.5);
	is!(&format!("{FOO}x=3; foo x"), 6);
	is!(&format!("{FOO}foo(3.0)+foo(3)"), 7.5);
	fails_with(&format!("{FOO}foo \"a\""), "fits no single variant");
}

#[test]
fn test_typed_parameter_variants_dispatch_by_argument() {
	is!("foo(x:int):=x+x; foo(x:text):=x+\"!\"; foo 3", 6);
	is!("foo(x:int):=x+x; foo(x:text):=x+\"!\"; foo \"a\"", "a!");
	is!("fib int i = i+1; fib float f = f*2; fib 2.5", 5.0);
}
