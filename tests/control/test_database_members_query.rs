// One-to-many as a query (notes/orm.md "Loading": one-to-many lists are queries too): natively `team.players` selects
// the rows pointing back instead of loading the whole people table
use crate::is;
use warp::wasm_emitter::eval;

const CLASSES: &str = "class Team{name: text; players: [Person]}\nclass Person{name: text; team: Team?}";
const SEED: &str = "red = Team(\"Red\")\nblue = Team(\"Blue\")\nteams.add(red)\nteams.add(blue)\npeople.add(Person(\"Bo\", red))\npeople.add(Person(\"Cy\", red))\nfor i in 1 to 50 { people.add(Person(\"N\" + i, blue)) }";

fn program(rest: &str) -> String {
	format!("{CLASSES}\nteams: [Team] = database.teams_query\npeople: [Person] = database.people_query\n{rest}")
}

#[cfg(feature = "native")]
#[test]
fn a_list_field_reads_only_the_rows_pointing_back() {
	eval(&program(SEED));
	let before = warp::database::rows_read();
	is!(&program("count(teams#1.players)"), 2);
	assert!(warp::database::rows_read() - before < 10, "team.players loaded the whole people table: {} rows", warp::database::rows_read() - before);
	is!(&program("red = teams#1\nbo = red.players#1\nbo.name = \"Bob\"\npeople#1.name"), "Bob");
	is!(&program("bo = people#1\nbo.team = teams#2\ncount(teams#1.players) * 100 + count(teams#2.players)"), 151);
}
