// Class instances as GC structs (notes/classes.md "Representation"): a variable that only ever holds instances of one
// class is a struct, `p.x` a struct.get by field index; the values are the same as with the generic Node form
use crate::is;

const POINT: &str = "class Point{x:int; y:int}; p = Point(3, 4); ";

/// The names of the functions the compiled program calls: call targets of its code, named by its name section
fn called_function_names(code: &str) -> Vec<String> {
	use wasmparser::{KnownCustom, Name, Operator, Parser, Payload};
	let module = warp::pipeline::compile(code).expect("compiles").bytes;
	let (mut called, mut names) = (vec![], std::collections::HashMap::new());
	for payload in Parser::new(0).parse_all(&module) {
		match payload.unwrap() {
			Payload::CodeSectionEntry(body) => {
				let mut operators = body.get_operators_reader().unwrap();
				while !operators.eof() {
					if let Operator::Call { function_index } = operators.read().unwrap() {
						called.push(function_index);
					}
				}
			}
			Payload::CustomSection(section) => {
				if let KnownCustom::Name(reader) = section.as_known() {
					for subsection in reader {
						if let Name::Function(map) = subsection.unwrap() {
							names.extend(map.into_iter().map(|naming| naming.unwrap()).map(|naming| (naming.index, naming.name.to_string())));
						}
					}
				}
			}
			_ => {}
		}
	}
	called.iter().filter_map(|index| names.get(index).cloned()).collect()
}

#[test]
fn a_struct_instance_gives_the_same_values() {
	is!(&format!("{POINT}p.x * p.y"), 12);
	is!(&format!("{POINT}i=0; s=0; while i<3 {{ s += p.x; i++ }}; s"), 9);
	is!("class Point{x:int; y:int}; s=0; i=0; while i<3 { q = Point(i, 1); s += q.x + q.y; i++ }; s", 6);
	is!(&format!("{POINT}string(p)"), "Point{x:3 y:4}");
}

#[test]
fn a_field_read_of_a_struct_variable_searches_no_names() {
	let calls = called_function_names(&format!("{POINT}p.x * p.y"));
	assert!(!calls.iter().any(|name| name == "map_find" || name == "struct_body"), "{calls:?}");
}

const SUMMED: &str = "class Point{x:int; y:int; sum() := x + y}; ";

#[test]
fn a_method_of_a_struct_variable_takes_the_struct() {
	let calls = called_function_names(&format!("{SUMMED}p = Point(3, 4); p.sum()"));
	assert!(!calls.iter().any(|name| name == "map_find" || name == "struct_body"), "{calls:?}");
	is!(&format!("{SUMMED}p = Point(3, 4); p.sum()"), 7);
	is!(&format!("{SUMMED}Point(3, 4).sum()"), 7);
	is!(&format!("{SUMMED}ps = [Point(1, 2), Point(3, 4)]; s = 0; for p in ps {{ s += p.sum() }}; s"), 10);
	is!(&format!("{SUMMED}f(p:Point) := p.sum() * 2; f(Point(1, 2))"), 6);
}

#[test]
fn a_field_write_of_a_struct_variable_sets_the_field() {
	let calls = called_function_names(&format!("{POINT}p.x = 7; p.y += 1; p.x * p.y"));
	assert!(!calls.iter().any(|name| ["map_find", "struct_body", "field_with"].contains(&name.as_str())), "{calls:?}");
	is!(&format!("{POINT}p.x = 7; p.y += 1; p.x * p.y"), 35);
	is!(&format!("{POINT}i=0; while i<3 {{ p.x += i; i++ }}; string(p)"), "Point{x:6 y:4}");
	is!(&format!("{POINT}p.x++; p.y--; p.x - p.y"), 1);
	is!(&format!("{POINT}q = p; q.x = 9; p.x"), 3); // value semantics: q is a copy
}

#[test]
fn an_int_field_holds_any_int() {
	is!("class Big{n:int}; b = Big(2^70); b.n + 1 == 2^70 + 1", 1);
	is!("class Big{n:int}; b = Big(1); b.n = 2^70; b.n * 2 == 2^71", 1);
}

#[test]
fn a_field_write_of_another_kind_is_a_type_error() {
	crate::common::fails_with(&format!("{POINT}p.x = \"a\"; p"), "x of Point is an int field, got");
	crate::common::fails_with(&format!("{POINT}p.y = [1]; p"), "y of Point is an int field, got");
}

const COUNTER: &str = "class Counter{n:int; inc() := n += 1}; ";

#[test]
fn a_method_changing_its_struct_variable_keeps_the_struct() {
	let calls = called_function_names(&format!("{COUNTER}c = Counter(0); c.inc(); c.n"));
	assert!(!calls.iter().any(|name| ["map_find", "struct_body", "field_with"].contains(&name.as_str())), "{calls:?}");
	is!(&format!("{COUNTER}c = Counter(0); i=0; while i<5 {{ c.inc(); i++ }}; c.n"), 5);
	is!(&format!("{COUNTER}c = Counter(0); d = c; c.inc(); d.n"), 0); // d is a copy
	is!(&format!("{COUNTER}c = Counter(1); d = inc(c); c.n * 10 + d.n"), 12); // inc gives a changed copy
}

#[test]
fn a_field_of_an_instance_in_a_list_is_found_by_its_written_name() {
	let program = "class Point{x:int; y:int}; ps = [Point(1, 2), Point(3, 4)]; s = 0; for p in ps { s += p.x + p.y }; s";
	assert!(called_function_names(program).iter().any(|name| name == "instance_field"));
	is!(program, 10);
	is!("class P{x:int}; f() := P(4); f().x", 4);
}

/// Card class-person: a hyphenated field (P164) of a class instance reads like any other (was a stack overflow)
#[test]
fn a_hyphenated_field_of_an_instance() {
	is!("class person{phone-number:text}; p = person{phone-number:\"12\"}; p.phone-number", "12");
	is!("class person{phone-number:text}; p = person{phone-number:\"12\"}; p.phone-number = \"3\"; p.phone-number", "3");
}
