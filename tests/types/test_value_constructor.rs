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
