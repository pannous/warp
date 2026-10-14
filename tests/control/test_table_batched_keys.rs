// card orm-batched (notes/orm.md Loading): rows read before their table loads find their foreign keys' rows together,
// one query `id IN (…)` for a page of rows, instead of loading the whole other table; each key's row is still the one
// instance of its row
use crate::is;
use warp::wasm_emitter::eval;

const CLASSES: &str = "class Team{name: text}\nclass Person{name: text; team: Team?}";
const SEED: &str =
	"for i in 1 to 300 { teams.add(Team(\"T\" + i)) }\nfor i in 1 to 300 { people.add(Person(\"N\" + i, teams#i)) }";

fn program(rest: &str) -> String {
	format!(
		"{CLASSES}\nstored teams: [Team] in \"teams_batched\"\nstored people: [Person] in \"people_batched\"\n{rest}"
	)
}

#[cfg(feature = "native")]
#[test]
fn a_page_of_rows_reads_only_the_rows_its_keys_point_to() {
	eval(&program(SEED));
	let read = |rest: &str, expected: &str, rows: usize| {
		let before = warp::host::database::rows_read();
		is!(&program(rest), expected);
		assert_eq!(warp::host::database::rows_read(), before + rows, "{rest} read other rows than its {rows}");
	};
	read("people#7.team.name", "T7", 2);
	read("found = \"\"\nfor p in people { if p.name == \"N50\" { found = p.team.name; break } }\nfound", "T50", 200);
	read("names = [p.team.name for p in (people where it.name == \"N9\")]\nnames#1", "T9", 2);
}

#[test]
fn a_key_read_alone_is_the_one_instance_of_its_row() {
	eval(&program(SEED));
	is!(&program("t = people#3.team\nt.name = \"Three\"\nteams#3.name"), "Three");
	is!(&program("people#3.team.name"), "Three");
	is!(&program("teams#4.name = \"Four\"\npeople#4.team.name"), "Four");
}
