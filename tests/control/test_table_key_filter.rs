#![cfg(feature = "native")]
// card table-key-filter: `people where it.team == red` of a foreign key compares its column with red's row id in SQL
// (it was "table.select: Team{…} is no column value"); `it.team == ø` keeps the rows without one
use crate::is;
use warp::wasm_emitter::eval;

const SEED: &str = "teams.add(Team(\"Red\"))\nteams.add(Team(\"Blue\"))\npeople.add(Person(\"Ann\", teams#1))\npeople.add(Person(\"Bo\", teams#2))\npeople.add(Person(\"Cy\"))";

fn program(rest: &str) -> String {
	format!("class Team{{name: text}}\nclass Person{{name: text; team: Team?}}\nteams: [Team] = database.teams_key_filter\npeople: [Person] = database.people_key_filter\n{rest}")
}

#[test]
fn rows_whose_foreign_key_is_a_given_row() {
	eval(&program(SEED));
	is!(&program("red = teams#1\n(people where it.team == red).map(p => p.name)"), warp::texts(vec!["Ann"]));
	is!(&program("count(people where it.team != teams#1)"), 2);
	is!(&program("(people where it.team == ø).map(p => p.name)"), warp::texts(vec!["Cy"]));
}
