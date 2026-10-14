// `class b extends a` (user decision P117, notes/classes.md step 3): b gets a's fields and methods, its own definitions
// override a's; a b can be used where an a is wanted
use crate::is;

const ANIMALS: &str = "class animal{name; speak() := name + \" makes a sound\"; describe() := \"I am \" + name}; class dog extends animal{speak() := name + \" barks\"}";

#[test]
fn a_subclass_has_the_fields_and_methods_of_its_parent() {
	is!(&format!("{ANIMALS}; dog(\"Rex\").describe()"), "I am Rex");
	is!(&format!("{ANIMALS}; dog(\"Rex\").name"), "Rex");
}

#[test]
fn a_subclass_overrides_a_method() {
	is!(&format!("{ANIMALS}; dog(\"Rex\").speak()"), "Rex barks");
	is!(&format!("{ANIMALS}; animal(\"Cat\").speak()"), "Cat makes a sound");
}

#[test]
fn a_subclass_adds_fields() {
	is!("class point{x:int y:int; sum() := x + y}; class point3 extends point{z:int; sum() := x + y + z}; point3(1, 2, 3).sum() + point(1, 2).sum()", 9);
}

#[test]
fn a_subclass_instance_is_accepted_where_the_parent_is_wanted() {
	is!(&format!("{ANIMALS}; greet(a:animal) := \"hello \" + a.name; greet(dog(\"Rex\"))"), "hello Rex");
}
