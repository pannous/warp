// card orm step 5 (notes/orm.md "Writes and transactions"): `transaction { … }` is BEGIN … COMMIT; a block that fails
// rolls its writes back and the failure goes on
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(table: &str, rest: &str) -> String {
	format!("{PERSON}\npeople: [Person] = database.{table}\n{rest}")
}

#[test]
fn a_transaction_commits_its_writes() {
	eval(&program("people_committed", "transaction {\npeople.add(Person(\"Bo\", 30))\npeople.add(Person(\"Cy\", 10))\n}"));
	is!(&program("people_committed", "count(people)"), 2);
}

#[test]
fn a_failing_transaction_rolls_back_and_fails() {
	eval(&program("people_rolled_back", "people.add(Person(\"Al\", 40))"));
	crate::common::fails_with(&program("people_rolled_back", "transaction {\npeople.add(Person(\"Bo\", 30))\nraise \"no Bo\"\n}"), "no Bo");
	is!(&program("people_rolled_back", "count(people)"), 1);
}

#[test]
fn a_field_change_in_a_failing_transaction_is_rolled_back() {
	eval(&program("people_aged", "people.add(Person(\"Al\", 40))"));
	crate::common::fails_with(&program("people_aged", "al = people#1\ntransaction {\nal.age += 1\nraise \"not now\"\n}"), "not now");
	is!(&program("people_aged", "people#1.age"), 40);
}

// a caught failure: the instances the program holds read their rows again, a row added in the block is gone
#[test]
fn a_rolled_back_transaction_restores_the_instances_held() {
	eval(&program("people_restored", "people.add(Person(\"Al\", 40))"));
	let code = "al = people#1\ntry transaction {\nal.age += 1\npeople.add(Person(\"Bo\", 30))\nraise \"not now\"\n} catch e { 0 }\n";
	is!(&program("people_restored", &format!("{code}al.age")), 40);
	is!(&program("people_restored", &format!("{code}count(people)")), 1);
	is!(&program("people_restored", &format!("{code}(people where it.age == 40)#1.age")), 40);
}

#[test]
fn a_transaction_gives_its_block_value() {
	is!(&program("people_valued", "transaction {\npeople.add(Person(\"Di\", 20))\ncount(people)\n}"), 1);
}
