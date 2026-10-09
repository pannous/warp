#![cfg(feature = "native")]
// cards orm-nested, orm-list-add: `red.players.add(p)` of a one-to-many field points p's key to red, written to p's
// row, or inserts p into people when it has no row yet; no row is duplicated
use crate::is;
use warp::wasm_emitter::eval;

const CLASSES: &str = "class Team{name: text; players: [Person]}\nclass Person{name: text; team: Team}";
const SEED: &str = "teams.add(Team(\"Red\"))\nteams.add(Team(\"Blue\"))\npeople.add(Person(\"Bo\", teams#1))";

fn program(tables: &str, rest: &str) -> String {
	format!("{CLASSES}\nteams: [Team] = database.teams_{tables}\npeople: [Person] = database.people_{tables}\n{rest}")
}

#[test]
fn adding_a_row_to_a_list_field_moves_it() {
	eval(&program("moved", SEED));
	is!(&program("moved", "teams#2.players.add(people#1)\ncount(teams#2.players)"), 1);
	is!(&program("moved", "people#1.team.name"), "Blue");
	is!(&program("moved", "count(people)"), 1);
}

#[test]
fn adding_a_new_instance_to_a_list_field_inserts_it() {
	eval(&program("inserted", SEED));
	is!(&program("inserted", "teams#1.players.add(Person(\"Cy\", teams#2))\ncount(teams#1.players)"), 2);
	is!(&program("inserted", "people#2.team.name"), "Red");
}
