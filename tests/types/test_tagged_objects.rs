// A tagged object `Person{name:"A" hobbies:[1 2]}` of no declared class is data (D4), the key Person over its fields:
// a field read reads its fields (card person-name, the README's Person example). Also card class-ticket: a static
// read and write inside the value{} constructor
use crate::is;

#[test]
fn a_field_of_a_tagged_object_is_read_from_its_fields() {
	is!("Person{name:\"A\" hobbies:[1 2]}.name", "A");
	is!("p = Person{name:\"A\" hobbies:[1 2]}; p.hobbies#2", 2);
	is!("p = Person {name:\"A\" age:3}; p.age", 3);
	is!("x = a:{b:1}; x.b", 1);
	is!("x = a:{b:1}; x.a.b", 1); // the tag itself still names the fields
	is!("p = Person {\n    name: \"Alice\"\n    age: 30\n    hobbies: [ \"reading\", \"hiking\", \"coding\" ]\n}\np.hobbies#2", "hiking");
}

#[test]
fn a_static_counts_constructions_in_the_value_block() {
	is!("class Ticket{static sold = 0; id = 0; value{ sold += 1; id = sold }}; a = Ticket(); b = Ticket(); b.id", 2);
}
