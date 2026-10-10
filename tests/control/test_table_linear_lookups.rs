// card orm-linear: a foreign key finds its row, and a loaded table's filter its instances, in the table's identity map
// (notes/orm.md): loading 10k rows with a required key, and filtering 10k loaded rows, take linear time
use crate::is;
use crate::lists::test_int_map::{timed, ENTRIES};
use warp::wasm_emitter::eval;

const CLASSES: &str = "class Team{name: text}\nclass Person{name: text; age: int; team: Team}";

fn program(tables: &str, rest: &str) -> String {
	format!("{CLASSES}\nteams: [Team] = database.teams_{tables}\npeople: [Person] = database.people_{tables}\n{rest}")
}

fn seeded(tables: &str) {
	eval(&program(tables, &format!("for i in 1 to {ENTRIES} {{ t = Team(\"t\"); teams.add(t); people.add(Person(\"p\", i, t)) }}")));
}

#[test]
fn ten_thousand_rows_with_a_required_key_load_in_linear_time() {
	seeded("linear_keys");
	is!(&program("linear_keys", &timed("s = 0; for p in people { s += p.team.id }", "s")), ENTRIES * (ENTRIES + 1) / 2);
}

#[test]
fn a_loaded_table_filters_in_linear_time() {
	seeded("linear_filter");
	is!(&program("linear_filter", &timed("all = people; kept = people where it.age > 10", "count(kept)")), ENTRIES - 10);
}
