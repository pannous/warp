//! What samples/data_structures.warp needed: data keys read like variables, fields of list elements, list
//! comprehensions, named tuples
use crate::is;

#[test] // `person: {name: "Alice"}` then `person.name`: the data is the scope
fn test_data_key_read_as_a_variable() {
	is!("person: {name: \"Alice\"}\nperson.name", "Alice");
	is!("company: {employees: [{name: \"Bob\"}, {name: \"Carol\"}, {name: \"Dave\"}]}\ncompany.employees[2].name", "Dave");
	is!("c = {e: [{name: \"Bob\", r: 1}, {name: \"Carol\", r: 2}]}; (c.e#2).r", 2);
}

#[test] // `[x * x for x in xs]`, `[x for x in xs if c]`
fn test_list_comprehension() {
	is!("count([x * x for x in 1..5])", 4);
	is!("squares = [x * x for x in 1..5]; squares#4", 16);
	is!("numbers = [1, 2, 3, 4, 5]; evens = [x for x in numbers if x % 2 == 0]; evens#2", 4);
}

#[test] // `(x: 10, y: 20)` reads its fields like an object
fn test_named_tuple_fields() {
	is!("point = (x: 10, y: 20); point.y", 20);
	is!("r = (min: 0, max: 100); r.max - r.min", 100);
}
