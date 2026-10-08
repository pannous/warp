#![cfg(feature = "native")]
// card orm-nested: `.add` on a list field reached through a field (`d.team.players.add(d)`) needs no temporary
// variable; it once failed with 'undefined function: add'. What the add means for the rows (card orm-list-add) is
// not pinned here: it only appends in memory so far
use crate::is;
use warp::wasm_emitter::eval;

const CLASSES: &str = "class Team{name: text; players: [Person]}\nclass Person{name: text; team: Team}";
const SEED: &str = "red = Team(\"Red\", [])\nteams.add(red)\npeople.add(Person(\"Bo\", red))";

fn program(tables: &str, rest: &str) -> String {
	format!("{CLASSES}\nteams: [Team] = database.teams_{tables}\npeople: [Person] = database.people_{tables}\n{rest}")
}

#[test]
fn add_on_a_list_field_of_a_field() {
	eval(&program("nested", SEED));
	is!(&program("nested", "d = people#1\nd.team.players.add(d)\nd.team.name"), "Red");
	is!(&program("nested", "f(p: Person) := p.team.players.add(p)\nf(people#1)\npeople#1.name"), "Bo");
}
