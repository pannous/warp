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
#[ignore = "next"] // step 1 of the struct backend (notes/classes.md "Template")
fn a_field_read_of_a_struct_variable_searches_no_names() {
	let calls = called_function_names(&format!("{POINT}p.x * p.y"));
	assert!(!calls.iter().any(|name| name == "map_find" || name == "struct_body"), "{calls:?}");
}
