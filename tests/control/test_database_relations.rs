#![cfg(feature = "native")]
// ORM relations (card orm, notes/orm.md step 4): a field of a registered class is a foreign key, a list field of one is
// the rows of the other table pointing back (one-to-many). Rows are instances, so a team read through a person is the
// team of the teams table
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

const CLASSES: &str = "class Team{name: text; players: [Person]}\nclass Person{name: text; team: Team}";
const SEED: &str = "red = Team(\"Red\", [])\nteams.add(red)\nteams.add(Team(\"Blue\", []))\npeople.add(Person(\"Bo\", red))\npeople.add(Person(\"Cy\", red))";

fn program(tables: &str, rest: &str) -> String {
	format!("{CLASSES}\nteams: [Team] = database.teams_{tables}\npeople: [Person] = database.people_{tables}\n{rest}")
}

#[test]
fn a_field_of_a_registered_class_is_a_foreign_key() {
	eval(&program("key", SEED));
	is!(&program("key", "people#2.team.name"), "Red");
	is!(&program("key", "people#1.team.id"), 1);
}

#[test]
fn a_list_field_holds_the_rows_pointing_back() {
	eval(&program("players", SEED));
	is!(&program("players", "count(teams#1.players)"), 2);
	is!(&program("players", "teams#1.players#2.name"), "Cy");
	is!(&program("players", "count(teams#2.players)"), 0);
	is!(&program("players", "people.add(Person(\"Di\", teams#2))\nteams#2.players#1.name"), "Di");
}

#[test]
fn a_related_row_is_the_same_instance() {
	eval(&program("same", SEED));
	is!(&program("same", "red = teams#1\nred.name = \"Crimson\"\npeople#1.team.name"), "Crimson");
	is!(&program("same", "people#2.team.name"), "Crimson");
}

#[test]
fn a_changed_foreign_key_is_written_through() {
	eval(&program("moved", SEED));
	eval(&program("moved", "bo = people#1\nbo.team = teams#2"));
	is!(&program("moved", "people#1.team.name"), "Blue");
	is!(&program("moved", "count(teams#1.players)"), 1);
}

#[test]
fn a_row_pointing_to_an_instance_without_a_row_is_an_error() {
	fails_with(&program("unsaved", "people.add(Person(\"Ed\", Team(\"Green\", [])))"), "add it to its table first");
}

#[test]
fn a_table_is_registered_after_the_tables_it_points_to() {
	let swapped = format!("{CLASSES}\npeople: [Person] = database.people_swapped\nteams: [Team] = database.teams_swapped\ncount(people)");
	fails_with(&swapped, "register teams before people");
}

#[test]
fn the_orm_sample_runs_twice_alike() {
	is!("samples/orm.warp", "Ann");
	is!("samples/orm.warp", "Ann");
}
