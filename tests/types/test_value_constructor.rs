// The `value{…}` constructor block of a class (wiki/constructor.md): it runs at every construction, after the given
// fields are matched, and may set fields the class does not declare otherwise
use crate::is;

#[test]
fn a_value_block_runs_at_every_construction() {
	is!("class Person{name; value{ id = 7 }}; p = Person{name:\"Joe\"}; p.id", 7);
	is!("class Person{name; value{ id = 7 }}; p = Person(\"Joe\"); p.name", "Joe");
	is!("class Counter{n:int; value{ n = n * 2 }}; Counter(3).n", 6);
	is!("class Person{name\n  value {\n    this.tag = \"p\"\n  }\n}\nPerson{name:\"Ann\"}.tag", "p");
}

#[test]
fn a_value_constructor_takes_parameters() {
	is!("class Person{name; value(n){ name = n + \"!\" }}; Person(\"Joe\").name", "Joe!");
	is!("class Person{name\n  value(n) {\n    this.name = n\n  }\n}\nPerson(\"Ann\").name", "Ann");
	is!("class Box{w:int; h:int; value(side){ w = side; h = side }}; b = Box(3); b.w * b.h", 9);
}
