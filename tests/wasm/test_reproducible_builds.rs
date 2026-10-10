//! The same program compiles to the same bytes every time (card emitter-deterministic): no pass may order its output
//! by a HashMap's iteration, which differs from map to map.

const COMPILES: usize = 8;

const TWO_GENERATORS: &str = "averager() := { total = 0; n = 0; average = 0; while yes { value = yield average; total += value; n += 1; average = total / n } }
a = averager()
next(a)
echo() := { while yes { got = yield 1; if got == ø { return }; yield got } }
e = echo()
[a.send(10), next(e), e.send(7)]";

const TWO_TABLES: &str = "class Team{name: text; players: [Person]}
class Person{name: text; age: int; team: Team?}
stored teams: [Team]
stored people: [Person]
people.add(Person(\"Bo\", 30))
teams.add(Team(\"Climbers\"))
bo = people#1
teams#1.players.add(bo)
bo.age += 1
save bo";

fn assert_reproducible(program: &str) {
	let first = warp::pipeline::compile(program).expect("compiles").bytes;
	for _ in 1..COMPILES {
		assert!(warp::pipeline::compile(program).expect("compiles").bytes == first, "another compile of the same program gave other bytes:\n{program}");
	}
}

#[test]
fn test_generator_classes_compile_in_one_order() {
	assert_reproducible(TWO_GENERATORS);
}

#[test]
fn test_table_saves_compile_in_one_order() {
	assert_reproducible(TWO_TABLES);
}
