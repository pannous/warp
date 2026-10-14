// Mixins (Dart/Scala `with`, Ruby `include`): `mixin Walker{walk() := name + " walks"}` is no class but fields and
// methods a class takes in, read on the class's own fields; the class's own ones override them
use crate::is;

const WALKER: &str = "mixin Walker{steps:int=0; walk() := name + \" walks\"}; ";

#[test]
fn a_class_takes_in_a_mixin() {
	is!(&format!("{WALKER}class Duck with Walker{{name}}; Duck(\"Don\").walk()"), "Don walks");
	is!(&format!("{WALKER}class Duck{{name; include Walker}}; Duck(\"Don\").walk()"), "Don walks");
	is!(&format!("{WALKER}class Duck with Walker{{name}}; Duck(\"Don\").steps"), 0);
}

#[test]
fn a_class_takes_in_several_mixins_and_overrides() {
	let swimmer = "mixin Swimmer{swim() := name + \" swims\"}; ";
	is!(&format!("{WALKER}{swimmer}class Duck with Walker, Swimmer{{name}}; Duck(\"Don\").swim()"), "Don swims");
	is!(&format!("{WALKER}class Duck with Walker{{name; walk() := \"waddles\"}}; Duck(\"Don\").walk()"), "waddles");
}
