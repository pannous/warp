// notes/orm.md Loading: a class method reads a table as any function does (it saw the unloaded, empty list)
use crate::is;

// each program its own table: an in-memory database lives as long as its thread
fn people(table: &str, rest: &str) -> String {
	format!("class Person{{name: text; age: int}}\npeople: [Person] = database.{table}\npeople.add(Person(\"Al\", 40))\npeople.add(Person(\"Bo\", 30))\n{rest}")
}

#[test]
fn a_method_reads_the_table() {
	is!(&people("method_count", "class Club{ name: text; fn total() := count(people) }\nClub(\"x\").total()"), 2);
	is!(&people("method_loop", "class Club{ name: text; ages() := { s = 0; for p in people { s += p.age }; s } }\nClub(\"x\").ages()"), 70);
	is!(&people("method_element", "class Club{ name: text; second() := people#2.name }\nClub(\"x\").second()"), "Bo");
}

// a field named like a table is the field, not the table
#[test]
fn a_field_named_like_a_table_stays_the_field() {
	is!(&people("method_field", "class Club{ people: int; fn n() := people + 1 }\nClub(5).n()"), 6);
}

// a method's filter is lowered as a function's: a table filter is its query, `it` the element, not the instance
#[test]
fn a_method_filters_the_table() {
	is!(&people("method_filter", "class Club{ name: text; fn adults() := count(people where it.age > 35) }\nClub(\"x\").adults()"), 1);
}

#[test]
fn a_method_filters_a_list() {
	is!("xs = [1, 5, 9]; class Club{ name: text; fn big() := count(xs where it > 3) }; Club(\"x\").big()", 2);
	is!("class Club{ least: int; fn big(xs) := count(xs where it > least) }; Club(4).big([1, 5, 9])", 2);
}
