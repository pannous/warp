#![cfg(feature = "native")] // runtime warnings are collected by the native host
// card orm-dangling: a foreign key whose row is gone, or the 0 of a column added for one, no longer fails the open:
// a required key (`team: Team`) leaves its row out with a warning naming it, an optional one (`team: Team?`) reads ø
use crate::is;
use warp::wasm_emitter::eval;

const TEAMS: &str = "class Team{name: text}\nteams: [Team] = database.teams_";
const PEOPLE_BEFORE: &str = "class Person{name: text}\npeople: [Person] = database.people_";

/// Bo and Cy in a people table that had no team column yet, so their added column holds 0
fn seeded(tables: &str) {
	eval(&format!("{TEAMS}{tables}\nteams.add(Team(\"Red\"))\n{PEOPLE_BEFORE}{tables}\npeople.add(Person(\"Bo\"))\npeople.add(Person(\"Cy\"))"));
}

fn program(tables: &str, team_type: &str, rest: &str) -> String {
	format!("{TEAMS}{tables}\nclass Person{{name: text; team: {team_type}}}\npeople: [Person] = database.people_{tables}\n{rest}")
}

#[test]
fn a_required_key_without_its_row_leaves_the_row_out_with_a_warning() {
	seeded("required");
	warp::diagnostic::take_runtime_warnings();
	is!(&program("required", "Team", "count(people)"), 0);
	let warnings = warp::diagnostic::take_runtime_warnings();
	assert!(warnings.iter().any(|warning| warning.contains("people row 1") && warning.contains("Team?")), "{warnings:?}");
	is!(&program("required", "Team", "people.add(Person(\"Di\", teams#1))\npeople#1.team.name"), "Red");
}

#[test]
fn an_optional_key_without_its_row_reads_empty() {
	seeded("optional");
	is!(&program("optional", "Team?", "count(people)"), 2);
	is!(&program("optional", "Team?", "people#1.team == ø"), true);
	is!(&program("optional", "Team?", "people.add(Person(\"Di\", teams#1))\npeople#3.team.name"), "Red");
	is!(&program("optional", "Team?", "people.add(Person(\"Ed\", ø))\npeople#4.name"), "Ed");
}
