// The elements of a list of object literals are objects: `xs[1].a`, `for o in xs { o.a }`, `xs.map(o => o.a)`
use crate::is;

#[test]
fn an_element_of_a_list_of_objects_has_fields() {
	is!("xs = [{a:1} {a:2}]; xs[1].a", 2);
	is!("xs = [{a:1} {a:2}]; o = xs#2; o.a", 2);
	is!("xs = [{a:1} {a:2}]; s=0; for o in xs { s += o.a }; s", 3);
	is!("xs = [{a:1} {a:2}]; xs.map(o => o.a).sum()", 3);
	is!("people = [{name:\"bo\" age:3} {name:\"al\" age:5}]; people.filter(p => p.age > 4).map(p => p.name).join(\",\")", "al");
}

#[test]
fn an_element_of_a_computed_list_of_objects_has_fields() {
	is!("data = {users: [{name: \"a\", age: 30}, {name: \"b\", age: 20}]}; data.users.filter(u => u.age > 25)#1.name", "a");
}
