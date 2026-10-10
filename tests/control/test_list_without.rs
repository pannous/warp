// `people without team` (user, 2026-10-10, card people-where): `without` a field of a list of a class's instances
// filters like `where not`; it is the opposite of `with`. Removing values, `[1 2 3] without 2`, stays undefined
use crate::is;

const PEOPLE: &str = "class P{name: text; team: text?; age: int}\npeople: [P] = [P{name: \"Al\", team: \"red\", age: 30}, P{name: \"Bo\", age: 10}, P{name: \"Cy\", age: 40}]";

fn program(rest: &str) -> String {
	format!("{PEOPLE}\n{rest}")
}

#[test]
fn without_a_field_keeps_the_elements_lacking_it() {
	is!(&program("str(name of people without team)"), "[\"Bo\" \"Cy\"]");
	is!(&program("count(people without team)"), 2);
	is!(&program("str(name of people with team)"), "[\"Al\"]");
}

#[test]
fn without_a_condition_keeps_the_elements_failing_it() {
	is!(&program("str(name of people without age > 20)"), "[\"Bo\"]");
	is!(&program("loners = name of people without team\nloners#2"), "Cy");
	is!(&program("young = people without age > 20\ncount(young)"), 1);
}
