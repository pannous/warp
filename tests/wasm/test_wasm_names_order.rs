// The name section lists its subsections by id and every name map by index: Firefox and wasm-opt warn otherwise
// ("out of order name subsections"), seen when the browser test runner compiled every test's module
use wasmparser::{Name, NameSectionReader, Parser, Payload};

fn assert_names_in_order(code: &str) {
	let bytes = warp::wasm_emitter::compile(code).unwrap_or_else(|value| panic!("{code} compiled to no module: {value:?}")).bytes;
	for payload in Parser::new(0).parse_all(&bytes) {
		let Payload::CustomSection(section) = payload.unwrap() else { continue };
		if section.name() != "name" {
			continue;
		}
		let mut previous_id = 0;
		for subsection in NameSectionReader::new(wasmparser::BinaryReader::new(section.data(), section.data_offset())) {
			let subsection = subsection.unwrap();
			let (id, indices): (u8, Vec<u32>) = match subsection {
				Name::Module { .. } => (0, vec![]),
				Name::Function(map) => (1, map.into_iter().map(|naming| naming.unwrap().index).collect()),
				Name::Type(map) => (4, map.into_iter().map(|naming| naming.unwrap().index).collect()),
				Name::Global(map) => (7, map.into_iter().map(|naming| naming.unwrap().index).collect()),
				Name::Field(map) => (10, map.into_iter().map(|naming| naming.unwrap().index).collect()),
				Name::Tag(map) => (11, map.into_iter().map(|naming| naming.unwrap().index).collect()),
				_ => continue,
			};
			assert!(id >= previous_id, "{code}: name subsection {id} after {previous_id}");
			assert!(indices.windows(2).all(|pair| pair[0] < pair[1]), "{code}: subsection {id} indices out of order: {indices:?}");
			previous_id = id;
		}
	}
}

#[test]
fn name_subsections_and_maps_are_in_order() {
	assert_names_in_order("x=3; x+1");
	assert_names_in_order("class Point{x:int y:int}\nclass Size{w:int}\np=Point{x:1 y:2}\ns=Size{w:3}\np.x+s.w");
	assert_names_in_order("try 1/0 else 7");
}
